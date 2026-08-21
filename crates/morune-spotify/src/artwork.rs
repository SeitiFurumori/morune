//! Download de capas.
//!
//! As URLs vem do proprio Spotify, dentro do [`morune_core::model::ImageSet`]
//! de cada album, artista ou playlist, e apontam para `i.scdn.co`. Nao ha
//! autenticacao no caminho -- a CDN e publica --, mas o cliente HTTP usado e o
//! da sessao da librespot mesmo assim: trazer um segundo cliente duplicaria a
//! pilha de TLS no binario para fazer o que este ja faz.
//!
//! **Aqui nao ha cache.** O contrato [`Artwork`] diz que guardar e de quem
//! chama, e quem chama e o aplicativo, que sabe onde fica o disco do usuario e
//! qual e o teto. Ver `crate::artwork` no lado do `morune-app`.

use bytes::Bytes;
use http::{Method, Request};
use morune_core::catalog::{Artwork, BoxFuture};
use morune_core::{CoreError, CoreResult};

use crate::auth::SharedSession;
use crate::error::from_librespot;

/// De onde capa e aceita.
///
/// A URL vem de resposta do servidor, e nao de entrada do usuario -- mas uma
/// resposta adulterada nao pode fazer o Morune buscar arquivo em qualquer
/// lugar da internet.
///
/// Listar host a host nao funcionou: o Spotify usa varios subdominios de
/// `spotifycdn.com`, um por tipo de imagem, e cada um so aparece quando a
/// conta tem aquele tipo de playlist. Ate agora vieram `pickasso` (capa
/// gerada), `blend-playlist-covers` e `wrapped-images` -- e nao ha razao para
/// crer que a lista acabou.
///
/// Entao a regra e por dominio: `i.scdn.co` exato, ou qualquer subdominio de
/// `spotifycdn.com`. Ver [`host_permitido`] para o que impede
/// `spotifycdn.com.exemplo.invalido` de passar.
const DOMINIO_CDN: &str = "spotifycdn.com";
const HOST_IMAGENS: &str = "i.scdn.co";

/// Teto de bytes por capa.
///
/// A maior capa do Spotify tem 640 px e fica bem abaixo disso. O teto existe
/// para que uma resposta inesperada nao vire alocacao sem limite na thread da
/// interface.
const MAX_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone)]
pub(crate) struct SpotifyArtwork {
    session: SharedSession,
}

impl std::fmt::Debug for SpotifyArtwork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpotifyArtwork")
            .field("session", &self.session)
            .finish()
    }
}

impl SpotifyArtwork {
    pub(crate) fn new(session: SharedSession) -> Self {
        Self { session }
    }
}

impl Artwork for SpotifyArtwork {
    fn fetch<'a>(&'a self, url: &'a str) -> BoxFuture<'a, CoreResult<Vec<u8>>> {
        Box::pin(async move {
            if !host_permitido(url) {
                return Err(CoreError::NotFound(
                    "capa fora dos hosts de imagem do Spotify".into(),
                ));
            }

            let session = self.session.get().ok_or(CoreError::NotAuthenticated)?;

            let request = Request::builder()
                .method(Method::GET)
                .uri(url)
                .body(Bytes::new())
                .map_err(|e| CoreError::InvalidState(format!("requisicao invalida: {e}")))?;

            let body = session
                .http_client()
                .request_body(request)
                .await
                .map_err(from_librespot)?;

            if body.len() > MAX_BYTES {
                return Err(CoreError::Decode(format!(
                    "capa maior que o teto de {MAX_BYTES} bytes"
                )));
            }

            Ok(body.to_vec())
        })
    }
}

/// `true` quando a URL aponta para um host de imagem do Spotify.
///
/// So `https`, e o host e comparado inteiro -- nao por prefixo. Comparar por
/// prefixo deixaria `spotifycdn.com.exemplo.invalido` passar, que e
/// exatamente o truque que a checagem existe para barrar.
fn host_permitido(url: &str) -> bool {
    let Some(resto) = url.strip_prefix("https://") else {
        return false;
    };
    let host = resto.split('/').next().unwrap_or_default();

    host == HOST_IMAGENS
        || host
            .strip_suffix(DOMINIO_CDN)
            .is_some_and(|prefixo| prefixo.ends_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aceita(url: &str) -> bool {
        host_permitido(url)
    }
    #[test]
    fn every_spotify_image_host_the_account_returned_is_accepted() {
        // Todos vistos numa conta real. Listar host a host quebrava a cada tipo
        // novo de playlist.
        assert!(aceita("https://i.scdn.co/image/ab67"));
        assert!(aceita(
            "https://pickasso.spotifycdn.com/image/ab67c0de/dt/v1/img"
        ));
        assert!(aceita(
            "https://blend-playlist-covers.spotifycdn.com/group-blends-v1/x.jpg"
        ));
        assert!(aceita(
            "https://wrapped-images.spotifycdn.com/image/yts-2023/x.jpg"
        ));
    }

    #[test]
    fn a_lookalike_domain_never_passes() {
        // O truque classico: colocar o dominio esperado como prefixo de outro.
        // Comparar o host inteiro e o que barra.
        assert!(!aceita("https://i.scdn.co.exemplo.invalido/x"));
        assert!(!aceita("https://spotifycdn.com.exemplo.invalido/x"));
        assert!(!aceita("https://exemplo.invalido/spotifycdn.com/x"));
        assert!(!aceita("https://exemplo.invalido/x.jpg"));
        // `spotifycdn.com` sem subdominio tambem nao: o Spotify sempre usa um.
        assert!(!aceita("https://spotifycdn.com/x"));
        // Sem TLS nao passa.
        assert!(!aceita("http://i.scdn.co/image/ab67"));
    }
}
