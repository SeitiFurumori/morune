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

/// Hashes da mutacao `addToPlaylist` (tambem `removeFromPlaylist`), o atual
/// primeiro: o novo e da lista do Meld em 01/11/2026 (trocado em 08/10), o
/// anterior e o medido em 23/09/2026.
pub(crate) const HASHES_PLAYLIST_ITEMS: [&str; 2] = [
    "b907ad6b088d5bf7680fa81198996d58398c2d55dfcdf59fcc959e0be98a418a",
    "47b2a1234b17748d332dd0431534f22450e9ecbb3d5ddcdacbd83368636a0990",
];

fn so_spotify(provider: Provider, id: &str) -> CoreResult<&str> {
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

    /// Tira faixas da playlist, todas as ocorrencias de cada uma.
    ///
    /// **Inferido**, como renomear: e a mesma remocao por URI que o rootlist
    /// aceita (`itemsAsKey`), mandada para as mudancas da playlist. O player
    /// web usa `removeFromPlaylist`, que pede o `uid` de cada linha -- dado que
    /// o Morune nao guarda. Se o Spotify recusar, o erro volta para a tela.
    pub(crate) async fn remover_faixas(
        &self,
        id: &PlaylistId,
        faixas: &[TrackId],
    ) -> CoreResult<()> {
        let id = so_spotify(id.provider, &id.id)?;
        let itens: Vec<serde_json::Value> = faixas
            .iter()
            .filter(|t| t.provider == Provider::Spotify)
            .map(|t| serde_json::json!({ "uri": format!("spotify:track:{}", t.id) }))
            .collect();
        if itens.is_empty() {
            return Err(CoreError::Unsupported("remover faixa de outro provedor"));
        }
        let corpo = serde_json::json!({
            "deltas": [{
                "ops": [{ "kind": 3, "rem": { "items": itens, "itemsAsKey": true } }],
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

    /// Leva o item da posicao `de` para a posicao `para` (contada na lista ja
    /// com o item no lugar novo).
    ///
    /// **Inferido**, como renomear: e a operacao `MOV` (4) do `playlist4`,
    /// mandada para as mudancas da playlist. O `toIndex` dela conta na lista
    /// de antes da mudanca, como o `insert_before` do Web API -- descer exige
    /// somar um.
    /// Diferente de adicionar e remover (por URI), mover e por posicao, e o
    /// Spotify so aceita com a `baseRevision` da lista a que a posicao se
    /// refere.
    pub(crate) async fn mover(
        &self,
        id: &PlaylistId,
        revisao: &[u8],
        de: usize,
        para: usize,
    ) -> CoreResult<()> {
        let id = so_spotify(id.provider, &id.id)?;
        if de == para {
            return Ok(());
        }
        let corpo = serde_json::json!({
            "baseRevision": base64(revisao),
            "deltas": [{
                "ops": [{ "kind": 4, "mov": mov(de, para) }],
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

/// Corpo do `MOV`: um item, de `de` para a posicao final `para`.
fn mov(de: usize, para: usize) -> serde_json::Value {
    let to = if para > de { para + 1 } else { para };
    serde_json::json!({ "fromIndex": de, "length": 1, "toIndex": to })
}

/// Base64 padrao, com `=`: e como bytes viajam no JSON do protobuf.
fn base64(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for bloco in bytes.chunks(3) {
        let n = (u32::from(bloco[0]) << 16)
            | (u32::from(*bloco.get(1).unwrap_or(&0)) << 8)
            | u32::from(*bloco.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= bloco.len() {
                s.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
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
    fn base64_como_o_padrao() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(&[0xff, 0xfe, 0x01, 0x00]), "//4BAA==");
    }

    #[test]
    fn mover_conta_o_destino_na_lista_de_antes() {
        // Subir: o destino nao muda com a saida do item.
        assert_eq!(mov(3, 0)["toIndex"], 0);
        // Descer: o item ainda ocupa um lugar antes do destino.
        assert_eq!(mov(0, 3)["toIndex"], 4);
        assert_eq!(mov(0, 3)["length"], 1);
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
