//! Login no Spotify por OAuth com PKCE.
//!
//! Tres decisoes moldam este modulo:
//!
//! **Sem senha, nunca.** O contrato [`Authenticator`] nao tem campo de senha, e
//! aqui tambem nao ha. O usuario digita a senha no site do Spotify, no navegador
//! dele, e o Morune so ve o codigo de autorizacao que volta.
//!
//! **Sem client secret.** PKCE existe justamente para aplicativos que nao
//! conseguem guardar segredo -- e um aplicativo de desktop distribuido nao
//! consegue: qualquer segredo embutido no `.exe` e publico.
//!
//! **O token nunca toca o disco em texto.** Quem cuida disso e
//! [`crate::token::TokenSource`]; aqui so se decide quando pedir um.

use std::sync::{Arc, Mutex};

use librespot_core::authentication::Credentials;
use librespot_core::{Session, SessionConfig};
use librespot_oauth::OAuthToken;
use morune_core::auth::{Authenticator, CredentialStore, UserProfile};
use morune_core::catalog::BoxFuture;
use morune_core::{CoreError, CoreResult};

use crate::error::{from_librespot, from_oauth};
use crate::token::{REDIRECT_URI, TokenSource};

/// Sessao ativa da librespot, compartilhada entre autenticador e motor.
///
/// O motor de reproducao precisa da mesma `Session` que o login criou: abrir
/// uma segunda conexao gastaria outro slot de dispositivo na conta, e o Spotify
/// derrubaria uma das duas. O catalogo tambem entra aqui: o cliente HTTP da
/// sessao ja resolve TLS, proxy e limite de requisicoes.
#[derive(Clone, Default)]
pub struct SharedSession(Arc<Mutex<Option<Session>>>);

impl SharedSession {
    pub fn get(&self) -> Option<Session> {
        self.0.lock().unwrap().clone()
    }

    fn set(&self, session: Option<Session>) {
        *self.0.lock().unwrap() = session;
    }
}

impl std::fmt::Debug for SharedSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedSession").field("conectada", &self.get().is_some()).finish()
    }
}

/// Autenticador do Spotify.
pub struct SpotifyAuthenticator {
    tokens: Arc<TokenSource>,
    session: SharedSession,
    /// Token do login em andamento, entre `begin_login` e `complete_login`.
    pending: Mutex<Option<OAuthToken>>,
}

impl std::fmt::Debug for SpotifyAuthenticator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpotifyAuthenticator").field("session", &self.session).finish()
    }
}

impl SpotifyAuthenticator {
    pub fn new(credentials: Arc<dyn CredentialStore>, session: SharedSession) -> Self {
        Self::with_tokens(Arc::new(TokenSource::new(credentials)), session)
    }

    pub(crate) fn with_tokens(tokens: Arc<TokenSource>, session: SharedSession) -> Self {
        Self { tokens, session, pending: Mutex::new(None) }
    }

    /// Abre a sessao da librespot com um token de acesso e devolve o perfil.
    async fn connect(&self, token: OAuthToken) -> CoreResult<UserProfile> {
        let session = Session::new(SessionConfig::default(), None);
        session
            .connect(Credentials::with_access_token(&token.access_token), false)
            .await
            .map_err(from_librespot)?;

        let data = session.user_data();
        let profile = UserProfile {
            id: data.canonical_username.clone(),
            display_name: Some(data.canonical_username.clone()).filter(|s| !s.is_empty()),
            avatar_url: None,
            country: Some(data.country.clone()).filter(|s| !s.is_empty()),
            // `can_stream` decide se a UI oferece reproducao. A librespot so
            // conecta contas que podem tocar, entao chegar aqui ja e a resposta.
            can_stream: true,
        };

        // O token so e adotado depois de a conexao dar certo: guardar o segredo
        // de um login que nao abriu sessao faria a proxima abertura tentar de
        // novo o que ja se sabe que nao funciona.
        self.tokens.adopt(token).await;
        self.session.set(Some(session));
        Ok(profile)
    }
}

impl Authenticator for SpotifyAuthenticator {
    fn name(&self) -> &'static str {
        "spotify"
    }

    fn restore(&self) -> BoxFuture<'_, CoreResult<Option<UserProfile>>> {
        Box::pin(async move {
            let Some(refresh) = self.tokens.stored_refresh()? else {
                return Ok(None);
            };

            let token = match self.tokens.exchange(&refresh).await {
                Ok(token) => token,
                Err(e) => {
                    // Refresh token revogado ou expirado nao e erro para quem
                    // esta abrindo o aplicativo: e so nao ter sessao. Apagar o
                    // segredo morto evita tentar de novo em toda abertura.
                    tracing::info!(error = %e, "sessao anterior nao pode ser restaurada");
                    self.tokens.discard_stored();
                    return Ok(None);
                }
            };

            self.connect(token).await.map(Some)
        })
    }

    fn begin_login(&self) -> BoxFuture<'_, CoreResult<String>> {
        Box::pin(async move {
            // O fluxo inteiro acontece aqui: a librespot abre o navegador, sobe
            // um servidor local e espera o retorno. Bloqueia ate o usuario
            // decidir, entao vai para uma thread propria.
            //
            // O contrato pede a URL que o usuario deve abrir, e a librespot nao
            // a expoe -- ela mesma abre o navegador. Devolvemos o endereco de
            // retorno, que e o que a tela precisa mostrar se o navegador nao
            // abrir sozinho. Trocar isto por um fluxo proprio sobre `oauth2`
            // devolveria a URL de verdade; ver docs/HANDOFF.md.
            let token = tokio::task::spawn_blocking(|| {
                TokenSource::interactive_client()
                    .and_then(|c| c.get_access_token().map_err(from_oauth))
            })
            .await
            .map_err(|e| CoreError::InvalidState(e.to_string()))??;

            *self.pending.lock().unwrap() = Some(token);
            Ok(REDIRECT_URI.to_string())
        })
    }

    fn complete_login(&self) -> BoxFuture<'_, CoreResult<UserProfile>> {
        Box::pin(async move {
            let token = self
                .pending
                .lock()
                .unwrap()
                .take()
                .ok_or_else(|| CoreError::InvalidState("nenhum login em andamento".into()))?;

            self.connect(token).await
        })
    }

    fn logout(&self) -> BoxFuture<'_, CoreResult<()>> {
        Box::pin(async move {
            self.session.set(None);
            self.tokens.forget().await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use morune_core::auth::MemoryCredentialStore;

    fn authenticator() -> SpotifyAuthenticator {
        SpotifyAuthenticator::new(
            Arc::new(MemoryCredentialStore::default()),
            SharedSession::default(),
        )
    }

    #[tokio::test]
    async fn restore_without_stored_token_is_not_an_error() {
        let auth = authenticator();
        assert!(auth.restore().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn complete_login_without_begin_fails_clearly() {
        let auth = authenticator();
        let error = auth.complete_login().await.unwrap_err();
        assert!(matches!(error, CoreError::InvalidState(_)));
    }

    #[tokio::test]
    async fn logout_clears_the_stored_secret() {
        let store = Arc::new(MemoryCredentialStore::default());
        store.store("spotify.refresh_token", b"segredo").unwrap();
        let auth = SpotifyAuthenticator::new(store.clone(), SharedSession::default());

        auth.logout().await.unwrap();
        assert!(store.load("spotify.refresh_token").unwrap().is_none());
    }
}
