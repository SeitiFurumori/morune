//! Edicao de playlists sem o Web API.
//!
//! Tudo aqui repete o que o player web do Spotify faz, conferido no pacote dele
//! em 23/09/2026 (`web-player.86442c64.js`):
//!
//! - **adicionar faixa**: mutacao `addToPlaylist` no pathfinder, hash
//!   `47b2a123...` (o mesmo de `removeFromPlaylist` e `moveItemsInPlaylist`);
//! - **criar**: `POST /playlist/v2/playlist` no spclient com a lista de
//!   operacoes em JSON, e em seguida a playlist nova entra no `rootlist` do
//!   usuario (`POST /playlist/v2/user/{usuario}/rootlist/changes`, que o player
//!   web manda com `withJsonContentType()`);
//! - **apagar**: tirar do `rootlist` -- e o que o cliente oficial chama de
//!   apagar; o Spotify nao expoe exclusao definitiva;
//! - **renomear**: `POST /playlist/v2/playlist/{id}/changes` com a operacao
//!   `UPDATE_LIST_ATTRIBUTES` (6). **Inferido** pela forma da criacao, sem
//!   captura do player web fazendo exatamente isto -- se o Spotify recusar, o
//!   erro volta para a tela e nada mais quebra.
//!
//! Formatos de corpo tirados de Aran404/SpotAPI (`spotapi/playlist.py`,
//! 2026-07).

use http::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use http::Method;
use morune_core::model::{PlaylistId, Provider, TrackId};
use morune_core::{CoreError, CoreResult};

use crate::auth::SharedSession;
use crate::error::from_librespot;

/// Hash da mutacao `addToPlaylist` (tambem `removeFromPlaylist`).
pub(crate) const HASH_PLAYLIST_ITEMS: &str =
    "47b2a1234b17748d332dd0431534f22450e9ecbb3d5ddcdacbd83368636a0990";

fn so_spotify<'a>(provider: Provider, id: &'a str) -> CoreResult<&'a str> {
    if provider == Provider::Spotify {
        Ok(id)
    } else {
        Err(CoreError::Unsupported("editar playlist de outro provedor"))
    }
}

fn corpo_json() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    h
}

/// `changes` do rootlist, com uma operacao de adicionar ou remover.
fn rootlist_delta(uri: &str, adicionar: bool) -> serde_json::Value {
    let agora = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let op = if adicionar {
        serde_json::json!({
            "kind": 2,
            "add": {
                "items": [{
                    "uri": uri,
                    "attributes": {
                        "timestamp": agora,
                        "formatAttributes": [],
                        "availableSignals": []
                    }
                }],
                "addFirst": true
            }
        })
    } else {
        serde_json::json!({
            "kind": 3,
            "rem": { "items": [{ "uri": uri }], "itemsAsKey": true }
        })
    };
    serde_json::json!({
        "deltas": [{ "ops": [op], "info": { "source": { "client": 5 } } }],
        "wantResultingRevisions": false,
        "wantSyncResult": false,
        "nonces": []
    })
}

/// Tira o `spotify:playlist:<id>` da resposta de criacao.
///
/// A resposta e a lista criada em JSON; o id aparece como URI. Procurar o
/// padrao em vez de modelar a resposta inteira mantem isto de pe se o Spotify
/// acrescentar campos.
fn id_da_resposta(corpo: &str) -> Option<String> {
    let i = corpo.find("spotify:playlist:")? + "spotify:playlist:".len();
    let id: String = corpo[i..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    (id.len() == 22).then_some(id)
}

#[derive(Clone)]
pub(crate) struct Edicao {
    session: SharedSession,
}

impl std::fmt::Debug for Edicao {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Edicao").finish()
    }
}

impl Edicao {
    pub(crate) fn new(session: SharedSession) -> Self {
        Self { session }
    }

    async fn spclient_post(&self, caminho: &str, corpo: &serde_json::Value) -> CoreResult<String> {
        let session = self.session.get().ok_or(CoreError::NotAuthenticated)?;
        let texto = corpo.to_string();
        let resposta = session
            .spclient()
            .request(
                &Method::POST,
                caminho,
                Some(corpo_json()),
                Some(texto.as_bytes()),
            )
            .await
            .map_err(from_librespot)?;
        Ok(String::from_utf8_lossy(&resposta).into_owned())
    }

    fn usuario(&self) -> CoreResult<String> {
        let session = self.session.get().ok_or(CoreError::NotAuthenticated)?;
        Ok(session.username())
    }

    /// Cria uma playlist com `nome` e a poe no topo da biblioteca.
    pub(crate) async fn criar(&self, nome: &str) -> CoreResult<PlaylistId> {
        let corpo = serde_json::json!({
            "ops": [{
                "kind": 6,
                "updateListAttributes": {
                    "newAttributes": {
                        "values": { "name": nome, "formatAttributes": [], "pictureSize": [] },
                        "noValue": []
                    }
                }
            }]
        });
        let resposta = self.spclient_post("/playlist/v2/playlist", &corpo).await?;
        let id = id_da_resposta(&resposta).ok_or_else(|| {
            CoreError::Decode("o Spotify criou a playlist mas nao devolveu o id".into())
        })?;
        let uri = format!("spotify:playlist:{id}");
        let usuario = self.usuario()?;
        self.spclient_post(
            &format!("/playlist/v2/user/{usuario}/rootlist/changes"),
            &rootlist_delta(&uri, true),
        )
        .await?;
        Ok(PlaylistId::spotify(id.as_str()))
    }

    /// Tira a playlist da biblioteca -- o "apagar" do cliente oficial.
    pub(crate) async fn apagar(&self, id: &PlaylistId) -> CoreResult<()> {
        let id = so_spotify(id.provider, &id.id)?;
        let usuario = self.usuario()?;
        self.spclient_post(
            &format!("/playlist/v2/user/{usuario}/rootlist/changes"),
            &rootlist_delta(&format!("spotify:playlist:{id}"), false),
        )
        .await
        .map(|_| ())
    }

    /// Troca o nome. Ver a nota de "inferido" no cabecalho.
    pub(crate) async fn renomear(&self, id: &PlaylistId, nome: &str) -> CoreResult<()> {
        let id = so_spotify(id.provider, &id.id)?;
        let corpo = serde_json::json!({
            "deltas": [{
                "ops": [{
                    "kind": 6,
                    "updateListAttributes": {
                        "newAttributes": {
                            "values": { "name": nome, "formatAttributes": [], "pictureSize": [] },
                            "noValue": []
                        }
                    }
                }],
                "info": { "source": { "client": 5 } }
            }],
            "wantResultingRevisions": false,
            "wantSyncResult": false,
            "nonces": []
        });
        self.spclient_post(&format!("/playlist/v2/playlist/{id}/changes"), &corpo)
            .await
            .map(|_| ())
    }
}

/// Variaveis de `addToPlaylist`: as faixas vao para o fim, como no Spotify.
pub(crate) fn variaveis_adicionar(
    playlist: &PlaylistId,
    faixas: &[TrackId],
) -> CoreResult<serde_json::Value> {
    let id = so_spotify(playlist.provider, &playlist.id)?;
    let uris: Vec<String> = faixas
        .iter()
        .filter(|t| t.provider == Provider::Spotify)
        .map(|t| format!("spotify:track:{}", t.id))
        .collect();
    if uris.is_empty() {
        return Err(CoreError::Unsupported("adicionar faixa de outro provedor"));
    }
    Ok(serde_json::json!({
        "playlistItemUris": uris,
        "playlistUri": format!("spotify:playlist:{id}"),
        "newPosition": { "moveType": "BOTTOM_OF_PLAYLIST", "fromUid": null }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acha_o_id_na_resposta_de_criacao() {
        let corpo = r#"{"revision":"AAA","uri":"spotify:playlist:37i9dQZF1DXcBWIGoYBM5M","x":1}"#;
        assert_eq!(
            id_da_resposta(corpo).as_deref(),
            Some("37i9dQZF1DXcBWIGoYBM5M")
        );
        assert_eq!(id_da_resposta("{}"), None);
    }

    #[test]
    fn remover_do_rootlist_usa_itens_como_chave() {
        let d = rootlist_delta("spotify:playlist:x", false);
        assert_eq!(d["deltas"][0]["ops"][0]["kind"], 3);
        assert_eq!(d["deltas"][0]["ops"][0]["rem"]["itemsAsKey"], true);
    }

    #[test]
    fn adicionar_vai_para_o_fim() {
        let v = variaveis_adicionar(
            &PlaylistId::spotify("p"),
            &[TrackId::spotify("a"), TrackId::local("x.flac")],
        )
        .unwrap();
        assert_eq!(
            v["playlistItemUris"],
            serde_json::json!(["spotify:track:a"])
        );
        assert_eq!(v["newPosition"]["moveType"], "BOTTOM_OF_PLAYLIST");
    }
}
