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
//! **O token nunca toca o disco em texto.** O refresh token vai direto para o
//! [`CredentialStore`], que no Windows e o Gerenciador de Credenciais.

use std::sync::{Arc, Mutex};

use librespot_core::authentication::Credentials;
use librespot_core::{Session, SessionConfig};
use librespot_oauth::{OAuthClient, OAuthClientBuilder, OAuthToken};
use morune_core::auth::{Authenticator, CredentialStore, UserProfile};
use morune_core::catalog::BoxFuture;
use morune_core::{CoreError, CoreResult};

use crate::error::{from_librespot, from_oauth};

/// Client ID publico usado pelos clientes de desktop do Spotify.
///
/// Nao e segredo e nao pode ser: um `.exe` distribuido nao guarda segredo
/// nenhum. E por isso que o fluxo e PKCE, que dispensa client secret.
const CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";

/// Endereco de retorno do navegador.
///
/// Precisa ser fixo porque tem de bater com o que esta registrado no client ID
/// -- nao da para sortear porta a cada login.
const REDIRECT_URI: &str = "http://127.0.0.1:5588/login";

/// Permissoes pedidas ao usuario.
///
/// A lista e curta de proposito: cada escopo aqui e uma coisa que o Morune
/// consegue fazer com a conta dele, e a tela de consentimento mostra todas.
/// Nada de escrita enquanto o aplicativo nao souber escrever.
const SCOPES: &[&str] = &[
    "streaming",
    "user-read-email",
    "user-read-private",
    "user-library-read",
    "playlist-read-private",
    "playlist-read-collaborative",
    "user-top-read",
    "user-read-recently-played",
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

/// Sessao ativa da librespot, compartilhada entre autenticador e motor.
///
/// O motor de reproducao precisa da mesma `Session` que o login criou: abrir
/// uma segunda conexao gastaria outro slot de dispositivo na conta, e o Spotify
/// derrubaria uma das duas.
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
    credentials: Arc<dyn CredentialStore>,
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
        Self { credentials, session, pending: Mutex::new(None) }
    }

    fn client() -> CoreResult<OAuthClient> {
        OAuthClientBuilder::new(CLIENT_ID, REDIRECT_URI, SCOPES.to_vec())
            .open_in_browser()
            .with_custom_message(BROWSER_MESSAGE)
            .build()
            .map_err(from_oauth)
    }

    /// Abre a sessao da librespot com um token de acesso e devolve o perfil.
    async fn connect(&self, token: &OAuthToken) -> CoreResult<UserProfile> {
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

        self.session.set(Some(session));
        Ok(profile)
    }

    fn save_refresh_token(&self, token: &OAuthToken) {
        if token.refresh_token.is_empty() {
            return;
        }
        if let Err(e) = self.credentials.store(REFRESH_KEY, token.refresh_token.as_bytes()) {
            // Falhar aqui custa um login a mais na proxima abertura, e nada
            // mais: nao vale derrubar uma sessao que ja esta funcionando.
            tracing::warn!(error = %e, "nao foi possivel guardar o refresh token");
        }
    }
}

impl Authenticator for SpotifyAuthenticator {
    fn name(&self) -> &'static str {
        "spotify"
    }

    fn restore(&self) -> BoxFuture<'_, CoreResult<Option<UserProfile>>> {
        Box::pin(async move {
            let Some(stored) = self.credentials.load(REFRESH_KEY)? else {
                return Ok(None);
            };
            let refresh = String::from_utf8(stored)
                .map_err(|_| CoreError::Storage("refresh token corrompido".into()))?;

            // A troca do refresh token e sincrona na librespot e faz I/O de
            // rede. Sai da thread do runtime para nao segurar o executor.
            let token = tokio::task::spawn_blocking(move || {
                Self::client().and_then(|c| c.refresh_token(&refresh).map_err(from_oauth))
            })
            .await
            .map_err(|e| CoreError::InvalidState(e.to_string()))?;

            let token = match token {
                Ok(token) => token,
                Err(e) => {
                    // Refresh token revogado ou expirado nao e erro para quem
                    // esta abrindo o aplicativo: e so nao ter sessao. Apagar o
                    // segredo morto evita tentar de novo em toda abertura.
                    tracing::info!(error = %e, "sessao anterior nao pode ser restaurada");
                    let _ = self.credentials.delete(REFRESH_KEY);
                    return Ok(None);
                }
            };

            self.save_refresh_token(&token);
            self.connect(&token).await.map(Some)
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
                Self::client().and_then(|c| c.get_access_token().map_err(from_oauth))
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

            self.save_refresh_token(&token);
            self.connect(&token).await
        })
    }

    fn logout(&self) -> BoxFuture<'_, CoreResult<()>> {
        Box::pin(async move {
            self.session.set(None);
            self.credentials.delete(REFRESH_KEY)?;
            Ok(())
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
        store.store(REFRESH_KEY, b"segredo").unwrap();
        let auth = SpotifyAuthenticator::new(store.clone(), SharedSession::default());

        auth.logout().await.unwrap();
        assert!(store.load(REFRESH_KEY).unwrap().is_none());
    }

    #[test]
    fn oauth_client_builds_with_the_registered_redirect() {
        // Um erro de digitacao no endereco de retorno so apareceria na hora do
        // login, depois de abrir o navegador. Aqui aparece no teste.
        assert!(SpotifyAuthenticator::client().is_ok());
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
