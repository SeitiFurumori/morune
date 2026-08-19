//! Cliente do Web API do Spotify.
//!
//! O protocolo que a librespot fala entrega **audio** e metadado de faixa, mas
//! nao busca nem biblioteca do usuario. Isso mora no Web API, que e HTTP comum
//! -- e e o que este modulo cobre.
//!
//! Duas escolhas evitam peso:
//!
//! - o cliente HTTP e o **da propria sessao** da librespot. Ele ja resolve TLS,
//!   proxy, `User-Agent` e limite de requisicoes. Trazer um segundo cliente
//!   (`reqwest` proprio, por exemplo) duplicaria a pilha de TLS no binario para
//!   fazer o que este ja faz;
//! - a resposta e desserializada direto para os tipos de
//!   [`crate::catalog`], sem passar por `serde_json::Value`. Uma playlist de mil
//!   faixas nao pode virar uma arvore de `Value` antes de virar dado util.

use std::sync::Arc;

use bytes::Bytes;
use http::header::{ACCEPT, AUTHORIZATION};
use http::{Method, Request};
use morune_core::{CoreError, CoreResult};
use serde::de::DeserializeOwned;

use crate::auth::SharedSession;
use crate::error::from_librespot;
use crate::token::TokenSource;

/// Raiz do Web API. Os caminhos passados adiante ja incluem a versao.
const BASE: &str = "https://api.spotify.com";

/// Cliente autenticado do Web API.
pub(crate) struct WebApi {
    session: SharedSession,
    tokens: Arc<TokenSource>,
}

impl std::fmt::Debug for WebApi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebApi").field("session", &self.session).finish()
    }
}

impl WebApi {
    pub(crate) fn new(session: SharedSession, tokens: Arc<TokenSource>) -> Self {
        Self { session, tokens }
    }

    /// Pais da conta, no formato que o Web API chama de `market`.
    ///
    /// Sem ele o Spotify devolve faixas que a conta nao pode tocar, e o usuario
    /// so descobre ao clicar. Vazio quando a sessao ainda nao informou.
    pub(crate) fn market(&self) -> Option<String> {
        self.session.get().map(|s| s.country()).filter(|c| c.len() == 2)
    }

    /// Faz um GET e desserializa a resposta.
    ///
    /// `path` comeca com `/` e ja vem com a query montada e escapada.
    pub(crate) async fn get<T: DeserializeOwned>(&self, path: &str) -> CoreResult<T> {
        let body = match self.send(path).await {
            // Um token dentro do prazo pode ser recusado quando o usuario
            // revoga o acesso pelo site. Renovar e tentar de novo resolve sem
            // que a tela precise pedir login.
            Err(CoreError::AuthExpired) => {
                self.tokens.invalidate().await;
                self.send(path).await?
            }
            other => other?,
        };

        serde_json::from_slice(&body).map_err(|e| {
            // O corpo bruto vai para o log, e nao para a tela: pode conter o
            // nome de tudo que o usuario salvou.
            tracing::debug!(path, error = %e, "resposta do Web API fora do formato esperado");
            CoreError::Decode(format!("resposta inesperada do Spotify em {path}"))
        })
    }

    async fn send(&self, path: &str) -> CoreResult<Bytes> {
        let session = self.session.get().ok_or(CoreError::NotAuthenticated)?;
        let bearer = self.tokens.bearer().await?;

        let request = Request::builder()
            .method(Method::GET)
            .uri(format!("{BASE}{path}"))
            .header(AUTHORIZATION, format!("Bearer {bearer}"))
            .header(ACCEPT, "application/json")
            .body(Bytes::new())
            .map_err(|e| CoreError::InvalidState(format!("requisicao invalida: {e}")))?;

        session.http_client().request_body(request).await.map_err(from_librespot)
    }
}

/// Escapa um valor para uso em query string.
///
/// Escrito a mao para nao trazer uma dependencia inteira de codificacao de URL
/// por causa de uma funcao. A regra e a do RFC 3986: o que nao e reservado
/// passa, o resto vira `%XX`. O espaco vira `%20` e nao `+`, que e o unico
/// formato aceito em todos os pontos do Web API.
pub(crate) fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Recorta um identificador do Spotify para uso em caminho de URL.
///
/// Ids vem de resposta do servidor ou de cache em disco. Um id com `/` ou `?`
/// mudaria o endpoint chamado, entao qualquer coisa fora do alfabeto base62 e
/// recusada antes de virar requisicao.
pub(crate) fn checked_id(id: &str) -> CoreResult<&str> {
    if !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric()) {
        Ok(id)
    } else {
        Err(CoreError::NotFound(format!("identificador invalido: {id}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaping_covers_what_a_search_box_accepts() {
        assert_eq!(escape("daft punk"), "daft%20punk");
        assert_eq!(escape("rock&roll"), "rock%26roll");
        assert_eq!(escape("a/b?c=d"), "a%2Fb%3Fc%3Dd");
        // Acentos sao a regra e nao a excecao em portugues: precisam sair em
        // UTF-8 escapado byte a byte.
        assert_eq!(escape("cafe\u{301}"), "cafe%CC%81");
        assert_eq!(escape("Legiao-Urbana_1.0~"), "Legiao-Urbana_1.0~");
    }

    #[test]
    fn identifiers_that_could_change_the_endpoint_are_refused() {
        assert_eq!(checked_id("4cOdK2wGLETKBW3PvgPWqT").unwrap(), "4cOdK2wGLETKBW3PvgPWqT");
        assert!(checked_id("").is_err());
        assert!(checked_id("../../me/player").is_err());
        assert!(checked_id("abc?fields=x").is_err());
    }
}
