//! Sessao do Spotify vista pela interface.
//!
//! O login abre o navegador e espera o usuario decidir -- isso pode levar
//! segundos ou minutos. A thread da interface nao pode esperar nada disso, e
//! por isso o fluxo inteiro vive aqui atras de um canal: quem dispara nao
//! bloqueia, e o resultado e recolhido depois, no mesmo temporizador que ja
//! atende a bandeja.
//!
//! O tipo tambem existe para manter `AppState` legivel: sem ele, cada acao de
//! sessao viraria tres campos soltos e uma maquina de estados implicita.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};

use morune_core::auth::UserProfile;
use morune_core::playback::PlaybackEngine;
use morune_core::{CoreError, CoreResult};
use morune_spotify::SpotifyBackend;

use crate::browse::Browse;

/// Em que ponto da sessao o aplicativo esta.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SessionState {
    #[default]
    LoggedOut,
    /// Fluxo em andamento. O navegador esta aberto esperando o usuario.
    Connecting,
    LoggedIn(UserProfile),
    /// Ultima tentativa falhou. A mensagem ja esta pronta para a tela.
    Failed(String),
}

impl SessionState {
    pub fn is_logged_in(&self) -> bool {
        matches!(self, SessionState::LoggedIn(_))
    }

    /// Nome para mostrar na barra lateral.
    pub fn account_name(&self) -> &str {
        match self {
            SessionState::LoggedIn(profile) => {
                profile.display_name.as_deref().unwrap_or(profile.id.as_str())
            }
            _ => "",
        }
    }
}

/// O que uma tentativa de sessao produziu.
enum Outcome {
    /// Restauracao encontrou (ou nao) uma sessao guardada.
    Restored(CoreResult<Option<UserProfile>>),
    /// Login interativo terminou.
    LoggedIn(CoreResult<UserProfile>),
}

/// Sessao do Spotify e o backend por tras dela.
pub struct Session {
    backend: Option<SpotifyBackend>,
    state: SessionState,
    /// Resultado de uma tentativa em andamento, quando ha uma.
    pending: Option<Receiver<Outcome>>,
    /// Busca e biblioteca. Existe desde a abertura, junto com o backend: as
    /// consultas e que recusam trabalho enquanto nao ha sessao.
    browse: Option<Browse>,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("estado", &self.state)
            .field("backend", &self.backend.is_some())
            .field("aguardando", &self.pending.is_some())
            .finish()
    }
}

impl Session {
    /// Sobe o backend. Nao toca a rede: falhar aqui nao pode impedir o
    /// aplicativo de abrir, entao o erro vira estado, nao panico.
    pub fn new(credentials: Arc<dyn morune_core::auth::CredentialStore>) -> Self {
        match SpotifyBackend::new(credentials) {
            Ok(backend) => {
                let browse =
                    Browse::new(backend.catalog(), backend.library(), backend.handle());
                Self {
                    backend: Some(backend),
                    state: SessionState::LoggedOut,
                    pending: None,
                    browse: Some(browse),
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "backend do Spotify indisponivel");
                Self {
                    backend: None,
                    state: SessionState::Failed(format!("Spotify indisponivel: {e}")),
                    pending: None,
                    browse: None,
                }
            }
        }
    }

    pub fn state(&self) -> &SessionState {
        &self.state
    }

    /// Catalogo e biblioteca, quando ha backend.
    pub fn browse_mut(&mut self) -> Option<&mut Browse> {
        self.browse.as_mut()
    }

    /// `true` enquanto ha uma tentativa em andamento.
    pub fn is_busy(&self) -> bool {
        self.pending.is_some()
    }

    /// Tenta reabrir a ultima sessao, sem interacao.
    ///
    /// Chamado uma vez na abertura. Nada acontece se nao houver segredo
    /// guardado -- e o caso comum na primeira execucao.
    pub fn restore(&mut self) {
        let Some(backend) = &self.backend else { return };
        if self.pending.is_some() {
            return;
        }

        let (tx, rx) = std::sync::mpsc::channel();
        let auth = backend.authenticator();
        backend.handle().spawn(async move {
            let _ = tx.send(Outcome::Restored(auth.restore().await));
        });
        self.pending = Some(rx);
    }

    /// Inicia o login interativo.
    ///
    /// Devolve a mensagem que a interface deve mostrar enquanto espera.
    pub fn login(&mut self) -> String {
        let Some(backend) = &self.backend else {
            return "Backend do Spotify indisponivel nesta maquina.".into();
        };
        if self.pending.is_some() {
            return "Ja ha um login em andamento; conclua no navegador.".into();
        }

        let (tx, rx) = std::sync::mpsc::channel();
        let auth = backend.authenticator();
        backend.handle().spawn(async move {
            // `begin_login` so retorna depois que o usuario autoriza no
            // navegador; `complete_login` abre a sessao com o token.
            let result = match auth.begin_login().await {
                Ok(_) => auth.complete_login().await,
                Err(e) => Err(e),
            };
            let _ = tx.send(Outcome::LoggedIn(result));
        });

        self.pending = Some(rx);
        self.state = SessionState::Connecting;
        "Abrimos seu navegador para entrar no Spotify.".into()
    }

    /// Encerra a sessao e apaga o segredo guardado.
    pub fn logout(&mut self) {
        if let Some(backend) = &self.backend {
            // O logout escreve no cofre do sistema; esperar aqui e aceitavel
            // porque e uma operacao local e rapida, sem rede.
            if let Err(e) = backend.block_on(backend.authenticator().logout()) {
                tracing::warn!(error = %e, "falha ao encerrar sessao");
            }
        }
        if let Some(browse) = &mut self.browse {
            browse.cancel();
        }
        self.pending = None;
        self.state = SessionState::LoggedOut;
    }

    /// Recolhe o resultado de uma tentativa em andamento.
    ///
    /// Devolve `Some` quando algo mudou -- com a mensagem para a barra de
    /// status e o motor novo, quando passou a haver um.
    pub fn poll(&mut self) -> Option<SessionChange> {
        let rx = self.pending.as_ref()?;
        let outcome = match rx.try_recv() {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => {
                self.pending = None;
                self.state = SessionState::Failed("o login foi interrompido".into());
                return Some(SessionChange {
                    message: "O login foi interrompido.".into(),
                    engine: None,
                });
            }
        };
        self.pending = None;

        match outcome {
            Outcome::Restored(Ok(None)) => {
                self.state = SessionState::LoggedOut;
                None
            }
            Outcome::Restored(Ok(Some(profile))) | Outcome::LoggedIn(Ok(profile)) => {
                let name = profile.display_name.clone().unwrap_or_else(|| profile.id.clone());
                self.state = SessionState::LoggedIn(profile);
                Some(SessionChange {
                    message: format!("Conectado como {name}."),
                    engine: self.build_engine(),
                })
            }
            Outcome::Restored(Err(e)) | Outcome::LoggedIn(Err(e)) => {
                let message = describe(&e);
                self.state = SessionState::Failed(message.clone());
                Some(SessionChange { message, engine: None })
            }
        }
    }

    /// Cria o motor de reproducao sobre a sessao recem-aberta.
    fn build_engine(&self) -> Option<Arc<dyn PlaybackEngine>> {
        let backend = self.backend.as_ref()?;
        match backend.engine() {
            Ok(engine) => Some(engine),
            Err(e) => {
                // Conectar e conseguir tocar sao coisas diferentes: o
                // dispositivo de audio pode faltar. A sessao continua valida.
                tracing::error!(error = %e, "sessao aberta mas sem motor de reproducao");
                None
            }
        }
    }
}

/// Mudanca de sessao a ser aplicada pela interface.
pub struct SessionChange {
    pub message: String,
    pub engine: Option<Arc<dyn PlaybackEngine>>,
}

/// Traduz um erro para uma frase que ajuda quem esta olhando a tela.
///
/// A mensagem tecnica vai para o log; aqui vale o que o usuario pode fazer a
/// respeito.
fn describe(error: &CoreError) -> String {
    match error {
        CoreError::AuthExpired | CoreError::NotAuthenticated => {
            "O Spotify recusou as credenciais. Entre de novo.".into()
        }
        // Ja vem pronta do backend, que e quem sabe o nome do plano. Repetir
        // aqui so faria a frase envelhecer em dois lugares.
        CoreError::AccountPlan(message) => message.clone(),
        CoreError::Network(_) => "Sem conexao com o Spotify. Verifique a internet.".into(),
        CoreError::Cancelled => "Login cancelado.".into(),
        CoreError::AudioDevice(_) => "Nenhum dispositivo de audio disponivel.".into(),
        other => format!("Nao foi possivel entrar: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use morune_core::auth::MemoryCredentialStore;

    fn session() -> Session {
        Session::new(Arc::new(MemoryCredentialStore::default()))
    }

    #[test]
    fn starts_logged_out_without_touching_the_network() {
        let session = session();
        assert_eq!(*session.state(), SessionState::LoggedOut);
        assert!(!session.is_busy());
    }

    #[test]
    fn restoring_without_a_stored_secret_changes_nothing() {
        let mut session = session();
        session.restore();

        // A tentativa e assincrona; esperar por ela aqui e o unico jeito de
        // afirmar que ela termina sem sessao em vez de ficar pendurada.
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while session.is_busy() && std::time::Instant::now() < deadline {
            if session.poll().is_some() {
                panic!("restauracao sem segredo guardado nao deveria mudar nada");
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        assert!(!session.is_busy(), "restauracao nao terminou");
        assert_eq!(*session.state(), SessionState::LoggedOut);
    }

    #[test]
    fn a_second_login_while_one_is_pending_is_refused() {
        let mut session = session();
        session.restore();
        if !session.is_busy() {
            // A restauracao pode ter terminado antes desta linha; nesse caso o
            // teste nao tem o que verificar.
            return;
        }
        assert!(session.login().contains("andamento"));
    }

    #[test]
    fn logout_from_a_clean_state_is_harmless() {
        let mut session = session();
        session.logout();
        assert_eq!(*session.state(), SessionState::LoggedOut);
    }

    #[test]
    fn account_name_prefers_the_display_name() {
        let profile = UserProfile {
            id: "id-tecnico".into(),
            display_name: Some("Felipe".into()),
            avatar_url: None,
            country: Some("BR".into()),
            can_stream: true,
        };
        assert_eq!(SessionState::LoggedIn(profile).account_name(), "Felipe");
    }

    #[test]
    fn account_name_falls_back_to_the_id() {
        let profile = UserProfile {
            id: "id-tecnico".into(),
            display_name: None,
            avatar_url: None,
            country: None,
            can_stream: true,
        };
        assert_eq!(SessionState::LoggedIn(profile).account_name(), "id-tecnico");
    }

    #[test]
    fn error_messages_say_what_to_do_next() {
        assert!(describe(&CoreError::AuthExpired).contains("Entre de novo"));
        assert!(describe(&CoreError::Network("timeout".into())).contains("internet"));
        assert_eq!(describe(&CoreError::Cancelled), "Login cancelado.");
    }

    #[test]
    fn an_account_that_cannot_stream_is_told_why_and_not_asked_to_retry() {
        // O caso mais comum de quem baixa um player aberto: conta gratuita.
        // A frase vem do backend inteira, sem "tente de novo" grudado nela.
        let message = describe(&CoreError::AccountPlan(
            "O Spotify so entrega musica para contas Premium, e esta e free.".into(),
        ));
        assert!(message.contains("Premium"));
        assert!(!message.contains("Nao foi possivel entrar"));
    }

    use std::time::Duration;
}
