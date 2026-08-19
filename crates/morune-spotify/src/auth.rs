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
//!
//! # Como a conta gratuita e barrada
//!
//! Este e um aplicativo aberto: quem clicar em "Entrar" pode ter qualquer tipo
//! de conta, e a maioria das contas do Spotify e gratuita. O Spotify nao entrega
//! audio para elas, e isso precisa virar uma frase na tela -- nunca um sumico.
//!
//! O caminho mudou em 19/08/2026, porque o anterior parou de existir:
//!
//! **Antes.** A librespot encerrava o processo em conta nao-Premium --
//! `check_catalogue` chamava `exit(1)` ao receber o pacote de produto --, entao
//! o plano era perguntado ao `/v1/me` **antes** de a credencial chegar nela.
//!
//! **Agora.** O `api.spotify.com` recusa qualquer token deste client ID, e o
//! `/v1/me` deixou de responder; ver [`docs/HANDOFF.md`](../../../docs/HANDOFF.md).
//! Em troca, a copia da librespot em `vendor/` nao encerra mais o processo. Isso
//! permite inverter a ordem: conecta, espera o pacote de produto e recusa com
//! uma frase quando o plano nao serve.
//!
//! A inversao so e segura por causa da copia em `vendor/`. Voltar a librespot
//! original sem restaurar um guarda anterior a conexao faz a janela sumir de
//! novo, silenciosamente.

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
    ///
    /// # Por que o plano e checado depois de conectar, e nao antes
    ///
    /// Ate 19/08/2026 o plano vinha do `/v1/me`, **antes** de a librespot ver a
    /// credencial, porque conta gratuita fazia o processo inteiro encerrar. Isso
    /// deixou de funcionar: o `api.spotify.com` recusa qualquer token deste
    /// client ID. Ver [`docs/HANDOFF.md`](../../../docs/HANDOFF.md).
    ///
    /// A ordem se inverteu porque a copia da librespot em `vendor/` nao encerra
    /// mais o processo -- ela so registra o plano no log. Com isso da para
    /// conectar primeiro e perguntar depois, que e a unica fonte de plano que
    /// sobrou: o pacote de produto, que chega logo apos a autenticacao.
    async fn connect(&self, token: OAuthToken) -> CoreResult<UserProfile> {
        // Se a conta for recusada logo abaixo, o segredo e esquecido junto.
        self.tokens.adopt(token.clone()).await;

        let session = Session::new(SessionConfig::default(), None);
        if let Err(e) = session.connect(Credentials::with_access_token(&token.access_token), false).await
        {
            self.tokens.forget().await.ok();
            return Err(from_librespot(e));
        }

        match account_plan(&session).await {
            Some(plan) if plan == "premium" => {}
            other => {
                self.tokens.forget().await.ok();
                return Err(CoreError::AccountPlan(plan_message(other.as_deref())));
            }
        }

        let data = session.user_data();
        let profile = UserProfile {
            id: data.canonical_username.clone(),
            // Sem o `/v1/me` nao ha nome de exibicao nem avatar. O identificador
            // da sessao e o que sobra, e e melhor que um espaco vazio na barra
            // lateral. Trocar por `user-profile-view` da spclient depende de
            // sondar esse endereco -- ver HANDOFF.md.
            display_name: Some(data.canonical_username.clone()).filter(|s| !s.is_empty()),
            avatar_url: None,
            country: Some(data.country.clone()).filter(|s| !s.is_empty()),
            can_stream: true,
        };

        self.session.set(Some(session));
        Ok(profile)
    }
}

/// Espera o pacote de produto e devolve o plano da conta.
///
/// O valor nao existe no instante em que `connect` retorna: ele chega num
/// pacote proprio, em torno de 200 ms depois. Perguntar uma vez so devolveria
/// `None` para toda conta, inclusive Premium -- e recusar Premium por
/// impaciencia e pior do que esperar meio segundo.
///
/// Devolve `None` quando o pacote nao chega no tempo previsto. Quem chama trata
/// isso como plano desconhecido, e nao como conta gratuita.
async fn account_plan(session: &Session) -> Option<String> {
    /// Somados, cobrem com folga os ~200 ms observados, e param cedo quando o
    /// pacote chega antes -- que e o caso comum.
    const ESPERAS_MS: [u64; 6] = [50, 100, 150, 300, 600, 1200];

    for espera in ESPERAS_MS {
        if let Some(plan) = session.get_user_attribute("type") {
            return Some(plan);
        }
        tokio::time::sleep(std::time::Duration::from_millis(espera)).await;
    }

    session.get_user_attribute("type")
}

/// Frase mostrada a quem nao pode ouvir.
///
/// Separa os dois casos porque a saida e diferente: conta gratuita se resolve
/// assinando, e plano desconhecido se resolve tentando de novo.
fn plan_message(plan: Option<&str>) -> String {
    match plan {
        Some(plan) => format!(
            "O Spotify so entrega musica para contas Premium, e esta e {plan}. \
             O Morune nao consegue tocar sem isso."
        ),
        None => "O Spotify nao informou o plano desta conta a tempo. \
                 Tente entrar de novo."
            .into(),
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
