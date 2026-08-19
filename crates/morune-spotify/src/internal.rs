//! O que so existe pelo protocolo interno do Spotify.
//!
//! Em novembro de 2024 o Spotify fechou boa parte do Web API para aplicativos
//! novos. Entre o que caiu esta exatamente o que um player precisa para nao ser
//! so uma caixa de busca: recomendacao por semente, artistas parecidos,
//! playlists editoriais -- e o acesso as playlists que o proprio Spotify monta
//! para a conta, como Descobertas da Semana e Radar de Novidades. Pedi-las por
//! `/v1/playlists/{id}` responde 404.
//!
//! O cliente oficial nao usa o Web API para isso. Ele usa o mesmo protocolo que
//! a librespot ja fala para tocar audio, e ali essas coisas continuam de pe.
//! Este modulo e a parte do backend que fala por esse caminho.
//!
//! **Por que isto nao e giria de risco.** O que entra aqui e o que a librespot
//! ja expoe com protobuf tipado: `get_rootlist` e `get_playlist` devolvem
//! `SelectedListContent`, com nome, dono e tamanho decorados na mesma resposta.
//! Nao ha JSON adivinhado nem endpoint inventado.
//!
//! **O que este modulo nao faz.** Metadado de faixa. O protocolo interno
//! entrega URIs; nome, artista, album e duracao vem do Web API em lote, que
//! nao foi afetado pela mudanca. Um pedido por faixa custaria cem requisicoes
//! numa playlist de cem.

use librespot_core::SpotifyUri;
use librespot_metadata::Metadata;
use librespot_metadata::playlist::list::SelectedListContent;
use librespot_protocol::playlist4_external::SelectedListContent as SelectedListContentMessage;
use morune_core::model::PlaylistId;
use morune_core::{CoreError, CoreResult};
use protobuf::Message;

use crate::auth::SharedSession;
use crate::error::from_librespot;

/// Quantas playlists o rootlist traz de uma vez.
///
/// E o tamanho que o cliente oficial pede. Uma conta com mais que isso perde a
/// cauda da lista, e isso e melhor que uma tela que demora para abrir.
const ROOTLIST_LENGTH: usize = 200;

/// Playlist como o protocolo interno a descreve.
///
/// Vem inteira numa requisicao so: o `decorate` do rootlist preenche nome, dono
/// e tamanho ao lado das URIs.
#[derive(Debug, Clone)]
pub(crate) struct PlaylistSummary {
    pub id: PlaylistId,
    pub name: String,
    pub owner: String,
    pub length: u32,
    /// Vazio nas playlists que o usuario criou ou seguiu; preenchido nas que o
    /// Spotify monta para ele. E o unico jeito honesto de separar as duas.
    pub format: String,
}

impl PlaylistSummary {
    /// `true` quando a playlist foi montada pelo Spotify para esta conta.
    ///
    /// Duas evidencias, porque nenhuma sozinha cobre tudo: o `format`, que as
    /// playlists algoritmicas carregam, e o dono `spotify`, que as editoriais
    /// carregam.
    pub fn made_by_spotify(&self) -> bool {
        !self.format.is_empty() || self.owner.eq_ignore_ascii_case("spotify")
    }
}

/// Conteudo de uma playlist pelo caminho interno.
#[derive(Debug, Clone)]
pub(crate) struct PlaylistContents {
    pub name: String,
    /// Ids de faixa em base62, na ordem da playlist.
    pub track_ids: Vec<String>,
}

/// Acesso ao protocolo interno sobre a sessao ativa.
#[derive(Debug)]
pub(crate) struct Internal {
    session: SharedSession,
}

impl Internal {
    pub(crate) fn new(session: SharedSession) -> Self {
        Self { session }
    }

    /// Todas as playlists da conta, como o cliente oficial as ve.
    ///
    /// Inclui o que o Web API deixou de entregar. Marcadores de pasta vem na
    /// mesma lista e sao descartados: sao URIs que nao apontam para playlist
    /// nenhuma.
    pub(crate) async fn rootlist(&self) -> CoreResult<Vec<PlaylistSummary>> {
        let session = self.session.get().ok_or(CoreError::NotAuthenticated)?;
        let bytes = session
            .spclient()
            .get_rootlist(0, Some(ROOTLIST_LENGTH))
            .await
            .map_err(from_librespot)?;

        let message = SelectedListContentMessage::parse_from_bytes(&bytes)
            .map_err(|e| CoreError::Decode(format!("rootlist ilegivel: {e}")))?;
        let list = SelectedListContent::try_from(&message).map_err(from_librespot)?;

        // `items` e `meta_items` sao listas paralelas: a decoracao do item na
        // posicao N esta na posicao N da outra. Fora de sincronia -- ou sem
        // decoracao nenhuma -- a playlist ainda entra, so que sem nome.
        let meta = &list.contents.meta_items;
        let mut out = Vec::new();

        for (index, item) in list.contents.items.iter().enumerate() {
            let SpotifyUri::Playlist { id, user } = &item.id else {
                continue;
            };
            let Ok(id) = id.to_base62() else { continue };

            let decorated = meta.get(index);
            out.push(PlaylistSummary {
                id: PlaylistId::spotify(id.as_str()),
                name: decorated.map(|m| m.attributes.name.clone()).unwrap_or_default(),
                owner: decorated
                    .map(|m| m.owner_username.clone())
                    .filter(|o| !o.is_empty())
                    .or_else(|| user.clone())
                    .unwrap_or_default(),
                length: decorated.map(|m| m.length.max(0) as u32).unwrap_or_default(),
                format: decorated.map(|m| m.attributes.format.clone()).unwrap_or_default(),
            });
        }

        Ok(out)
    }

    /// Nome e faixas de uma playlist pelo caminho interno.
    ///
    /// E o que faz Descobertas da Semana tocar: pelo Web API ela responde 404.
    pub(crate) async fn playlist(&self, id: &str) -> CoreResult<PlaylistContents> {
        let session = self.session.get().ok_or(CoreError::NotAuthenticated)?;
        let uri = SpotifyUri::from_uri(&format!("spotify:playlist:{id}"))
            .map_err(|e| CoreError::NotFound(format!("playlist {id}: {e}")))?;

        let playlist = librespot_metadata::Playlist::get(&session, &uri)
            .await
            .map_err(from_librespot)?;

        let track_ids = playlist
            .tracks()
            .filter_map(|uri| match uri {
                SpotifyUri::Track { id } => id.to_base62().ok(),
                // Episodio de podcast e arquivo local entram na mesma lista e
                // nao sao faixa: somem aqui em vez de virar linha que falha.
                _ => None,
            })
            .collect();

        Ok(PlaylistContents { name: playlist.name().to_string(), track_ids })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(owner: &str, format: &str) -> PlaylistSummary {
        PlaylistSummary {
            id: PlaylistId::spotify("37i9dQZEVXcJZyENOWUFo7"),
            name: "Descobertas da Semana".into(),
            owner: owner.into(),
            length: 30,
            format: format.into(),
        }
    }

    #[test]
    fn an_algorithmic_playlist_is_recognised_by_its_format() {
        assert!(summary("felipe", "discover-weekly").made_by_spotify());
    }

    #[test]
    fn an_editorial_playlist_is_recognised_by_its_owner() {
        // As editoriais nao trazem `format`, mas pertencem ao proprio Spotify.
        assert!(summary("spotify", "").made_by_spotify());
        assert!(summary("Spotify", "").made_by_spotify());
    }

    #[test]
    fn a_playlist_the_user_made_is_not_confused_with_one_the_spotify_made() {
        assert!(!summary("felipe", "").made_by_spotify());
    }
}
