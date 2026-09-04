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
use librespot_core::cache::Cache;
use librespot_core::{Session, SessionConfig};
use librespot_oauth::OAuthToken;
use morune_core::auth::{Authenticator, CredentialStore, UserProfile};
use morune_core::catalog::BoxFuture;
use morune_core::{CoreError, CoreResult};
use serde::Deserialize;

use crate::error::from_librespot;
use crate::token::TokenSource;

/// Sessao ativa da librespot, compartilhada entre autenticador e motor.
///
/// O motor de reproducao precisa da mesma `Session` que o login criou: abrir
/// uma segunda conexao gastaria outro slot de dispositivo na conta, e o Spotify
/// derrubaria uma das duas. O catalogo tambem entra aqui: o cliente HTTP da
/// sessao ja resolve TLS, proxy e limite de requisicoes.
#[derive(Clone, Default)]
pub struct SharedSession(Arc<Mutex<Option<Session>>>);

impl SharedSession {
    /// A sessao, se houver uma **viva**.
    ///
    /// Sessao invalidada nao serve para nada: a librespot fecha os canais e
    /// toda requisicao passa a responder `channel closed`, que na tela virava
    /// "estado invalido" sem dizer o que fazer. Devolver `None` faz o erro
    /// virar [`CoreError::NotAuthenticated`], que a interface ja sabe tratar.
    pub fn get(&self) -> Option<Session> {
        self.0.lock().unwrap().clone().filter(|s| !s.is_invalid())
    }

    /// `true` quando havia sessao e ela morreu.
    ///
    /// Diferente de nunca ter entrado: aqui houve login, e a conexao caiu -- o
    /// Spotify derruba sessao ociosa, e a rede cai sozinha. Quem le isto e o
    /// aplicativo, para reconectar sem pedir nada ao usuario.
    pub fn is_lost(&self) -> bool {
        self.0
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|s| s.is_invalid())
    }

    /// Esquece a sessao morta, para que a proxima tentativa comece limpa.
    pub fn forget_lost(&self) {
        let mut guarda = self.0.lock().unwrap();
        if guarda.as_ref().is_some_and(|s| s.is_invalid()) {
            *guarda = None;
        }
    }

    fn set(&self, session: Option<Session>) {
        *self.0.lock().unwrap() = session;
    }
}

impl std::fmt::Debug for SharedSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedSession")
            .field("conectada", &self.get().is_some())
            .finish()
    }
}

/// Autenticador do Spotify.
pub struct SpotifyAuthenticator {
    tokens: Arc<TokenSource>,
    session: SharedSession,
    /// Autorizacao em andamento, entre `begin_login` e `complete_login`.
    ///
    /// Antes guardava o token ja obtido, porque `begin_login` so retornava
    /// depois de o login inteiro terminar. Agora guarda a espera em si.
    pending: Mutex<Option<crate::token::PendingAuth>>,
    /// Cache de audio em disco, quando o usuario nao o desligou.
    ///
    /// **So audio.** A `Cache` da librespot tambem sabe guardar credenciais em
    /// `credentials.json`, e isso nunca e ligado aqui: a credencial do Morune
    /// vive no Gerenciador de Credenciais do Windows, e escreve-la em texto ao
    /// lado do cache desfaria essa decisao inteira.
    cache: Option<Cache>,
}

impl std::fmt::Debug for SpotifyAuthenticator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpotifyAuthenticator")
            .field("session", &self.session)
            .finish()
    }
}

impl SpotifyAuthenticator {
    pub fn new(credentials: Arc<dyn CredentialStore>, session: SharedSession) -> Self {
        Self::with_tokens(Arc::new(TokenSource::new(credentials)), session)
    }

    /// Liga o cache de audio em disco.
    ///
    /// Falhar aqui **nao** impede o login: sem cache o Spotify e baixado a cada
    /// reproducao, que e mais lento e gasta mais rede, mas funciona. Uma pasta
    /// sem permissao nao pode deixar ninguem sem musica.
    pub fn with_audio_cache(mut self, dir: &std::path::Path, limit_mb: u32) -> Self {
        if limit_mb == 0 {
            return self;
        }

        let limite = u64::from(limit_mb) * 1024 * 1024;
        match Cache::new(None::<&std::path::Path>, None, Some(dir), Some(limite)) {
            Ok(cache) => self.cache = Some(cache),
            Err(e) => {
                tracing::warn!(error = %e, dir = %dir.display(), "cache de audio indisponivel")
            }
        }
        self
    }

    pub(crate) fn with_tokens(tokens: Arc<TokenSource>, session: SharedSession) -> Self {
        Self {
            tokens,
            session,
            pending: Mutex::new(None),
            cache: None,
        }
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

        let session = Session::new(SessionConfig::default(), self.cache.clone());
        if let Err(e) = session
            .connect(Credentials::with_access_token(&token.access_token), false)
            .await
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
        // Nome de exibicao e foto vem do `user-profile-view`, e nao do `/v1/me`
        // -- aquele responde 429 para este aplicativo. A sonda de 19/08/2026
        // confirmou o formato; ver `bench-out/sonda/perfil.json`.
        let perfil = fetch_profile(&session, &data.canonical_username).await;
        let profile = UserProfile {
            id: data.canonical_username.clone(),
            // O identificador da sessao e o piso: sem ele a barra lateral
            // ficaria com um espaco vazio quando o perfil nao responder.
            display_name: perfil
                .as_ref()
                .and_then(|p| p.name.clone())
                .filter(|s| !s.is_empty())
                .or_else(|| Some(data.canonical_username.clone()).filter(|s| !s.is_empty())),
            avatar_url: perfil.and_then(|p| p.image_url).filter(|s| !s.is_empty()),
            country: Some(data.country.clone()).filter(|s| !s.is_empty()),
            can_stream: true,
        };

        self.session.set(Some(session));
        Ok(profile)
    }
}

/// Nome de exibicao e foto da conta, do `user-profile-view`.
///
/// Campos opcionais porque o perfil pode nao ter nenhum dos dois: quem nunca
/// escolheu nome nem foto recebe os derivados, e o proprio Spotify sinaliza
/// isso em `has_spotify_name` e `has_spotify_image`.
#[derive(Debug, Deserialize)]
struct ProfileView {
    name: Option<String>,
    image_url: Option<String>,
}

/// Busca o perfil da conta. `None` quando nao responde.
///
/// **Nunca falha o login.** O perfil e enfeite: sem ele a barra lateral mostra
/// o identificador da sessao e um avatar com a inicial, que e exatamente o que
/// o Morune fez ate agora. Derrubar uma sessao boa porque a foto nao veio seria
/// trocar o essencial pelo cosmetico.
///
/// Os limites de playlists e artistas vao em zero de proposito: a resposta traz
/// prateleiras inteiras que nao sao usadas aqui, e pedir cinco de cada so faria
/// o corpo crescer no caminho do login.
async fn fetch_profile(session: &Session, username: &str) -> Option<ProfileView> {
    let bytes = match session
        .spclient()
        .get_user_profile(username, Some(0), Some(0))
        .await
    {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::debug!(error = %e, "perfil da conta indisponivel");
            return None;
        }
    };

    match serde_json::from_slice::<ProfileView>(&bytes) {
        Ok(perfil) => Some(perfil),
        Err(e) => {
            tracing::debug!(error = %e, "perfil da conta em formato inesperado");
            None
        }
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

    /// Monta a autorizacao e devolve **a URL** que o usuario precisa abrir.
    ///
    /// Devolve antes de o usuario fazer qualquer coisa. A implementacao
    /// anterior so retornava depois que o login inteiro terminava, e entregava
    /// o endereco de retorno no lugar da URL -- o contrato pedia uma coisa e
    /// recebia outra. Com a URL de verdade em maos, a tela pode oferece-la para
    /// copiar, que e a saida de quem tem mais de um navegador.
    fn begin_login(&self) -> BoxFuture<'_, CoreResult<String>> {
        Box::pin(async move {
            let pendente = TokenSource::begin_interactive().await?;
            let url = pendente.url.clone();

            // Abrir o navegador depois de a porta estar escutando: aberto
            // antes, um retorno muito rapido bateria em porta fechada.
            pendente.open_browser();

            *self.pending.lock().unwrap() = Some(pendente);
            Ok(url)
        })
    }

    /// Espera o retorno do navegador e abre a sessao.
    ///
    /// A espera tem prazo e morre junto com a tarefa que a executa: cancelar
    /// aqui **solta a porta**, ao contrario do fluxo anterior, que a segurava
    /// ate o processo terminar.
    fn complete_login(&self) -> BoxFuture<'_, CoreResult<UserProfile>> {
        Box::pin(async move {
            let pendente = self
                .pending
                .lock()
                .unwrap()
                .take()
                .ok_or_else(|| CoreError::InvalidState("nenhum login em andamento".into()))?;

            let token = TokenSource::finish_interactive(pendente).await?;
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
