//! Token de acesso do Spotify: obtencao, guarda e renovacao.
//!
//! Existe separado do login porque duas partes do backend precisam do mesmo
//! token e por motivos diferentes:
//!
//! - o **login** troca o codigo do navegador por um token e abre a sessao;
//! - o **catalogo** assina cada requisicao ao Web API com ele.
//!
//! Se cada uma guardasse o seu, a segunda renovaria por conta propria e a conta
//! acumularia tokens vivos sem necessidade. Aqui ha um so, renovado sob trava,
//! e quem pede um token vencido espera a mesma renovacao em vez de disparar
//! outra.
//!
//! O refresh token nunca toca o disco em texto: vai para o [`CredentialStore`],
//! que no Windows e o Gerenciador de Credenciais.

use std::sync::Arc;

use librespot_oauth::{OAuthClient, OAuthClientBuilder, OAuthToken};
use morune_core::auth::CredentialStore;
use morune_core::{CoreError, CoreResult};
use tokio::sync::Mutex;

use crate::error::from_oauth;

/// Client ID publico usado pelos clientes de desktop do Spotify.
///
/// Nao e segredo e nao pode ser: um `.exe` distribuido nao guarda segredo
/// nenhum. E por isso que o fluxo e PKCE, que dispensa client secret.
pub(crate) const CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";

/// Endereco de retorno do navegador.
///
/// Precisa ser fixo porque tem de bater com o que esta registrado no client ID
/// -- nao da para sortear porta a cada login.
pub(crate) const REDIRECT_URI: &str = "http://127.0.0.1:5588/login";

/// Permissoes pedidas ao usuario.
///
/// A lista e curta de proposito: cada escopo aqui e uma coisa que o Morune
/// consegue fazer com a conta dele, e a tela de consentimento mostra todas.
/// Nada de escrita enquanto o aplicativo nao souber escrever.
pub(crate) const SCOPES: &[&str] = &[
    "streaming",
    "user-read-email",
    "user-read-private",
    "user-library-read",
    "playlist-read-private",
    "playlist-read-collaborative",
    "user-top-read",
    "user-read-recently-played",
    "user-follow-read",
];

/// Chave do refresh token no cofre do sistema.
const REFRESH_KEY: &str = "spotify.refresh_token";

/// Pagina mostrada no navegador quando o login termina.
const BROWSER_MESSAGE: &str = concat!(
    "<!doctype html><meta charset=\"utf-8\"><title>Morune</title>",
    "<body style=\"font-family:system-ui;display:grid;place-items:center;",
    "height:100vh;margin:0;background:#0b0d12;color:#eceff4\">",
    "<div style=\"text-align:center\"><h1 style=\"font-weight:600\">Pronto</h1>",
    "<p>Pode fechar esta aba e voltar para o Morune.</p></div></body>"
);

/// Fonte unica de token de acesso.
pub(crate) struct TokenSource {
    credentials: Arc<dyn CredentialStore>,
    /// Trava assincrona, e nao `std::sync::Mutex`: a renovacao faz rede
    /// enquanto a segura, e segurar uma trava sincrona atravessando `await`
    /// prenderia a thread do executor.
    current: Mutex<Option<OAuthToken>>,
}

impl std::fmt::Debug for TokenSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenSource").finish_non_exhaustive()
    }
}

impl TokenSource {
    pub(crate) fn new(credentials: Arc<dyn CredentialStore>) -> Self {
        Self { credentials, current: Mutex::new(None) }
    }

    /// Cliente OAuth que abre o navegador. So o login usa.
    pub(crate) fn interactive_client() -> CoreResult<OAuthClient> {
        Self::builder().open_in_browser().build().map_err(from_oauth)
    }

    /// Cliente OAuth silencioso, para renovar sem interromper o usuario.
    fn silent_client() -> CoreResult<OAuthClient> {
        Self::builder().build().map_err(from_oauth)
    }

    fn builder() -> OAuthClientBuilder {
        OAuthClientBuilder::new(CLIENT_ID, REDIRECT_URI, SCOPES.to_vec())
            .with_custom_message(BROWSER_MESSAGE)
    }

    /// Adota um token recem-obtido e guarda o refresh no cofre.
    pub(crate) async fn adopt(&self, token: OAuthToken) {
        self.persist(&token);
        *self.current.lock().await = Some(token);
    }

    /// Esquece o token em memoria e apaga o segredo guardado.
    pub(crate) async fn forget(&self) -> CoreResult<()> {
        *self.current.lock().await = None;
        self.credentials.delete(REFRESH_KEY)?;
        Ok(())
    }

    /// Apaga so o segredo guardado, sem tocar no token em memoria.
    ///
    /// Usado quando o refresh token e recusado: mante-lo faria toda abertura
    /// seguinte tentar o mesmo segredo morto.
    pub(crate) fn discard_stored(&self) {
        let _ = self.credentials.delete(REFRESH_KEY);
    }

    /// Refresh token guardado da ultima sessao, se houver.
    pub(crate) fn stored_refresh(&self) -> CoreResult<Option<String>> {
        let Some(bytes) = self.credentials.load(REFRESH_KEY)? else {
            return Ok(None);
        };
        String::from_utf8(bytes)
            .map(Some)
            .map_err(|_| CoreError::Storage("refresh token corrompido".into()))
    }

    /// Troca um refresh token por um token de acesso novo, sem adota-lo.
    ///
    /// A restauracao de sessao usa isto: se a troca falhar, quem chamou decide
    /// se apaga o segredo, e nao este modulo.
    pub(crate) async fn exchange(&self, refresh: &str) -> CoreResult<OAuthToken> {
        let token = Self::silent_client()?
            .refresh_token_async(refresh)
            .await
            .map_err(from_oauth)?;
        Ok(Self::keeping_refresh(token, refresh))
    }

    fn persist(&self, token: &OAuthToken) {
        if token.refresh_token.is_empty() {
            return;
        }
        if let Err(e) = self.credentials.store(REFRESH_KEY, token.refresh_token.as_bytes()) {
            // Falhar aqui custa um login a mais na proxima abertura, e nada
            // mais: nao vale derrubar uma sessao que ja esta funcionando.
            tracing::warn!(error = %e, "nao foi possivel guardar o refresh token");
        }
    }

    /// Mantem o refresh token anterior quando a resposta nao traz um novo.
    ///
    /// O Spotify so devolve refresh token quando ele muda. Adotar a resposta
    /// crua apagaria o unico segredo que permite voltar sem login.
    fn keeping_refresh(mut token: OAuthToken, previous: &str) -> OAuthToken {
        if token.refresh_token.is_empty() {
            token.refresh_token = previous.to_string();
        }
        token
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
    use morune_core::auth::MemoryCredentialStore;

    fn token(refresh: &str, valid_for: Duration) -> OAuthToken {
        OAuthToken {
            access_token: "acesso".into(),
            refresh_token: refresh.into(),
            expires_at: Instant::now() + valid_for,
            token_type: "Bearer".into(),
            scopes: SCOPES.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn source() -> (Arc<MemoryCredentialStore>, TokenSource) {
        let store = Arc::new(MemoryCredentialStore::default());
        (store.clone(), TokenSource::new(store))
    }

    #[tokio::test]
    async fn adopting_a_token_stores_the_refresh_secret() {
        let (store, tokens) = source();
        tokens.adopt(token("segredo", Duration::from_secs(3600))).await;
        assert_eq!(store.load(REFRESH_KEY).unwrap().as_deref(), Some(&b"segredo"[..]));
    }

    #[tokio::test]
    async fn forgetting_clears_memory_and_the_vault() {
        let (store, tokens) = source();
        tokens.adopt(token("segredo", Duration::from_secs(3600))).await;
        tokens.forget().await.unwrap();

        assert!(store.load(REFRESH_KEY).unwrap().is_none());
        assert!(tokens.stored_refresh().unwrap().is_none());
    }

    #[test]
    fn renewal_keeps_the_previous_refresh_token_when_none_comes_back() {
        let renewed = TokenSource::keeping_refresh(token("", Duration::from_secs(60)), "antigo");
        assert_eq!(renewed.refresh_token, "antigo");

        let rotated = TokenSource::keeping_refresh(token("novo", Duration::from_secs(60)), "antigo");
        assert_eq!(rotated.refresh_token, "novo");
    }

    #[test]
    fn the_oauth_client_builds_with_the_registered_redirect() {
        // Um erro de digitacao no endereco de retorno so apareceria na hora do
        // login, depois de abrir o navegador. Aqui aparece no teste.
        assert!(TokenSource::interactive_client().is_ok());
        assert!(TokenSource::silent_client().is_ok());
    }

    #[test]
    fn scopes_never_ask_for_write_access() {
        // A tela de consentimento mostra cada escopo. Pedir escrita sem usar
        // custa confianca do usuario e nao entrega nada.
        for scope in SCOPES {
            assert!(!scope.contains("modify"), "escopo de escrita pedido sem uso: {scope}");
        }
    }
}
