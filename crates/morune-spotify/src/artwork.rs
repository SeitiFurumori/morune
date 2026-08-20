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

/// Hosts de onde capa e aceita.
///
/// A URL vem de resposta do servidor, e nao de entrada do usuario -- mas uma
/// resposta adulterada nao pode fazer o Morune buscar arquivo em qualquer
/// lugar da internet.
///
/// Sao dois porque o Spotify usa dois: capa de album e de artista vem do
/// primeiro; capa gerada de playlist, do segundo. A barra no fim e o que
/// impede um dominio como `i.scdn.co.exemplo.invalido` de passar.
const HOSTS_PERMITIDOS: [&str; 2] =
    ["https://i.scdn.co/", "https://pickasso.spotifycdn.com/"];

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
        f.debug_struct("SpotifyArtwork").field("session", &self.session).finish()
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
            if !HOSTS_PERMITIDOS.iter().any(|host| url.starts_with(host)) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn aceita(url: &str) -> bool {
        HOSTS_PERMITIDOS.iter().any(|host| url.starts_with(host))
    }

    #[test]
    fn only_the_spotify_image_hosts_are_accepted() {
        // Uma URL vinda de resposta adulterada nao pode virar download de
        // qualquer coisa. O corte acontece antes de haver requisicao.
        assert!(aceita("https://i.scdn.co/image/ab67"));
        // Capa gerada de playlist mora noutro host, e o rootlist entrega a URL
        // pronta apontando para ele.
        assert!(aceita("https://pickasso.spotifycdn.com/image/ab67c0de/dt/v1/img"));
        assert!(!aceita("https://exemplo.invalido/x.jpg"));
        // Prefixo parecido nao passa: o `/` no fim e o que separa o host.
        assert!(!aceita("https://i.scdn.co.exemplo.invalido/x"));
        assert!(!aceita("http://i.scdn.co/image/ab67"));
    }
}
