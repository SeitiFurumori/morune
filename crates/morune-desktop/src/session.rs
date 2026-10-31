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

use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::Arc;

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
            SessionState::LoggedIn(profile) => profile
                .display_name
                .as_deref()
                .unwrap_or(profile.id.as_str()),
            _ => "",
        }
    }

    /// Foto da conta, quando o perfil traz uma.
    ///
    /// Vazio e o caso normal, nao excecao: quem nunca escolheu foto no Spotify
    /// nao tem uma, e o avatar volta a ser a inicial do nome.
    pub fn avatar_url(&self) -> &str {
        match self {
            SessionState::LoggedIn(profile) => profile.avatar_url.as_deref().unwrap_or_default(),
            _ => "",
        }
    }
}

/// O que uma tentativa de sessao produziu.
enum Outcome {
    /// Restauracao encontrou (ou nao) uma sessao guardada.
    Restored(CoreResult<Option<UserProfile>>),
    /// A autorizacao foi montada e ja da para mostrar o endereco.
    ///
    /// Chega **antes** do resultado, e e o ponto do fluxo novo: o usuario ve o
    /// link enquanto o navegador ainda esta aberto, e pode leva-lo para outro
    /// navegador se o que abriu sozinho for o perfil errado.
    AuthUrl(String),
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
    /// Endereco de autorizacao do login em andamento.
    ///
    /// Vazio fora de um login. Enquanto tem valor, a tela mostra o link e
    /// oferece copia-lo.
    auth_url: String,
    /// A tarefa que espera o navegador.
    ///
    /// Guardada para poder ser **abortada**: abortar solta a porta 5588, que e
    /// o que o fluxo anterior nao permitia. Sem isso, desistir de um login
    /// deixava o aplicativo sem como tentar de novo ate ser reiniciado.
    login_task: Option<tokio::task::JoinHandle<()>>,
    /// Tarefa de login cancelada que talvez ainda nao tenha terminado.
    ///
    /// Abortar so pede o fim: a porta 5588 fica presa ate a tarefa de fato
    /// acabar. O proximo login espera por ela antes de abrir a porta, senao
    /// Cancelar seguido de Entrar daria "porta em uso".
    cancelled_task: Option<tokio::task::JoinHandle<()>>,
    /// Sem conexao com o Spotify, e tentando de novo sozinho.
    ///
    /// **Por que existe.** A queda da sessao disparava uma tentativa so. Se a
    /// internet ainda nao tinha voltado, a tentativa falhava e o aplicativo
    /// ficava em "Sem conexão" para sempre -- nada mais tentava, e nada na tela
    /// dizia que bastava esperar. O Spotify mostra "Você está offline" e volta
    /// sozinho quando a rede volta; aqui e o mesmo.
    offline: bool,
    /// Quando tentar de novo, e quantas tentativas ja falharam seguidas.
    retry_at: Option<std::time::Instant>,
    retry_step: u32,
}

/// Espera antes da tentativa `passo` (comecando em 1): 5 s, 10 s, 30 s e
/// depois um minuto, para sempre.
///
/// Curta no comeco porque queda de Wi-Fi costuma durar segundos. Teto de um
/// minuto porque cada tentativa e uma conexao nova, e o criterio do projeto e
/// nao aparecer para quem esta jogando com a internet ruim.
fn retry_delay(passo: u32) -> std::time::Duration {
    std::time::Duration::from_secs(match passo {
        0 | 1 => 5,
        2 => 10,
        3 => 30,
        _ => 60,
    })
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
    pub fn new(
        credentials: Arc<dyn morune_core::auth::CredentialStore>,
        covers_dir: std::path::PathBuf,
        audio: morune_core::playback::AudioSettings,
        audio_cache_dir: &std::path::Path,
    ) -> Self {
        match SpotifyBackend::new(credentials, audio, audio_cache_dir) {
            Ok(backend) => {
                let browse = Browse::new(
                    backend.catalog(),
                    backend.library(),
                    backend.artwork(),
                    covers_dir,
                    backend.handle(),
                );
                Self {
                    backend: Some(backend),
                    state: SessionState::LoggedOut,
                    pending: None,
                    browse: Some(browse),
                    auth_url: String::new(),
                    login_task: None,
                    cancelled_task: None,
                    offline: false,
                    retry_at: None,
                    retry_step: 0,
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "backend do Spotify indisponivel");
                Self {
                    backend: None,
                    state: SessionState::Failed(format!("Spotify indisponível: {e}")),
                    pending: None,
                    browse: None,
                    auth_url: String::new(),
                    login_task: None,
                    cancelled_task: None,
                    offline: false,
                    retry_at: None,
                    retry_step: 0,
                }
            }
        }
    }

    pub fn state(&self) -> &SessionState {
        &self.state
    }

    /// Endereco de autorizacao do login em andamento. Vazio fora dele.
    pub fn auth_url(&self) -> &str {
        &self.auth_url
    }

    /// Desiste do login em andamento.
    ///
    /// Abortar a tarefa derruba o `await` que espera o navegador, e com ele o
    /// ouvinte da porta 5588 -- que e o que permite tentar de novo em seguida.
    /// Antes deste caminho, desistir exigia fechar o aplicativo pela bandeja.
    pub fn cancel_login(&mut self) {
        if let Some(task) = self.login_task.take() {
            task.abort();
            self.cancelled_task = Some(task);
        }
        self.pending = None;
        self.auth_url.clear();
        self.state = SessionState::LoggedOut;
    }

    /// Guarda preferencias de audio novas para o proximo motor.
    pub fn set_audio(&self, audio: morune_core::playback::AudioSettings) {
        if let Some(backend) = &self.backend {
            backend.set_audio(audio);
        }
    }

    /// Catalogo e biblioteca, quando ha backend.
    pub fn browse_mut(&mut self) -> Option<&mut Browse> {
        self.browse.as_mut()
    }

    /// `true` quando a conexao caiu e o Morune esta tentando voltar sozinho.
    pub fn is_offline(&self) -> bool {
        self.offline
    }

    /// `true` enquanto ha uma tentativa em andamento.
    pub fn is_busy(&self) -> bool {
        self.pending.is_some()
    }

    /// Reabre a sessao quando ela caiu, sem pedir nada ao usuario.
    ///
    /// A conexao com o Spotify nao dura para sempre: ele derruba sessao ociosa,
    /// e a rede cai sozinha. Sem isto, a partir da queda **tudo** respondia
    /// `channel closed` ate o usuario reiniciar o aplicativo -- e nada na tela
    /// dizia que era so reconectar.
    ///
    /// Devolve `true` quando comecou uma tentativa. O refresh token esta no
    /// cofre, entao a volta e silenciosa: sem navegador, sem clique.
    pub fn reconnect_if_lost(&mut self) -> bool {
        let Some(backend) = &self.backend else {
            return false;
        };
        if self.pending.is_some() {
            return false;
        }
        // Tentativa marcada depois de uma falha de rede.
        if !backend.session_lost() {
            let Some(quando) = self.retry_at else {
                return false;
            };
            if std::time::Instant::now() < quando {
                return false;
            }
            self.retry_at = None;
            tracing::info!(
                tentativa = self.retry_step + 1,
                "tentando reconectar ao Spotify"
            );
            self.restore();
            return true;
        }
        self.offline = true;

        // Sem descartar a sessao morta, `restore` a encontraria de novo e o
        // aplicativo ficaria tentando para sempre.
        backend.forget_lost_session();
        tracing::info!("sessao do Spotify caiu; reconectando");
        self.restore();
        true
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
            return "Não foi possível iniciar o Spotify nesta máquina. Feche e abra o Morune para tentar de novo.".into();
        };
        // A restauracao silenciosa dura segundos e nao abre navegador: quem
        // clica nesse meio-tempo so precisa esperar.
        if self.pending.is_some() && self.login_task.is_none() {
            return "Conectando ao Spotify. Um instante.".into();
        }

        // **Clicar de novo recomeca o login; nunca recusa.**
        //
        // O caso real: a pessoa fecha a aba do navegador antes de concluir e
        // clica em Entrar outra vez. Recusar ("ja ha um login em andamento")
        // deixava ela presa ate o prazo de cinco minutos acabar, porque o
        // Cancelar so existe em Configuracoes. O Spotify faz o mesmo que aqui:
        // o "Tente de novo" dele dispara o login do zero.
        let anterior = self.login_task.take();
        let recomeco = anterior.is_some();
        if let Some(task) = &anterior {
            task.abort();
        }
        let anterior = anterior.or_else(|| self.cancelled_task.take());
        self.auth_url.clear();

        let (tx, rx) = std::sync::mpsc::channel();
        let auth = backend.authenticator();
        let task = backend.handle().spawn(async move {
            // Abortar so pede o fim; a tarefa antiga solta a porta 5588 quando
            // de fato termina. Abrir a porta antes disso daria "porta em uso".
            if let Some(task) = anterior {
                let _ = task.await;
            }
            // `begin_login` monta a autorizacao e devolve a URL sem esperar
            // ninguem; `complete_login` e que aguarda o navegador voltar.
            let result = match auth.begin_login().await {
                Ok(url) => {
                    let _ = tx.send(Outcome::AuthUrl(url));
                    auth.complete_login().await
                }
                Err(e) => Err(e),
            };
            let _ = tx.send(Outcome::LoggedIn(result));
        });

        self.pending = Some(rx);
        self.login_task = Some(task);
        self.state = SessionState::Connecting;
        if recomeco {
            "Abrimos o navegador de novo para entrar no Spotify.".into()
        } else {
            "Abrimos seu navegador para entrar no Spotify.".into()
        }
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
        self.offline = false;
        self.retry_at = None;
        self.retry_step = 0;
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
        // A URL nao encerra nada: o login continua correndo, e este e o unico
        // desfecho que deixa `pending` de pe.
        if let Outcome::AuthUrl(url) = outcome {
            self.auth_url = url;
            return Some(SessionChange {
                message: "Conclua no navegador. Se ele abriu na conta errada, copie o link.".into(),
                engine: None,
            });
        }

        self.pending = None;
        self.login_task = None;
        self.auth_url.clear();

        match outcome {
            Outcome::AuthUrl(_) => None,
            Outcome::Restored(Ok(None)) => {
                // Sem segredo guardado nao ha a que voltar: parar de tentar.
                self.offline = false;
                self.retry_at = None;
                self.retry_step = 0;
                self.state = SessionState::LoggedOut;
                None
            }
            Outcome::Restored(Ok(Some(profile))) | Outcome::LoggedIn(Ok(profile)) => {
                self.offline = false;
                self.retry_at = None;
                self.retry_step = 0;
                let name = profile
                    .display_name
                    .clone()
                    .unwrap_or_else(|| profile.id.clone());
                self.state = SessionState::LoggedIn(profile);
                Some(SessionChange {
                    message: format!("Conectado como {name}."),
                    engine: self.build_engine(),
                })
            }
            Outcome::Restored(Err(e)) | Outcome::LoggedIn(Err(e)) => {
                // Sem isto a barra de status diz "verifique a internet" e o
                // motivo real -- porta ocupada, plano recusado, token negado,
                // resposta fora do formato -- nao aparece em lugar nenhum.
                tracing::error!(error = %e, "login no Spotify falhou");
                let mut message = describe(&e);
                // Rede: o segredo continua no cofre, entao a volta e
                // silenciosa assim que a internet voltar. Marca a proxima
                // tentativa em vez de desistir.
                if matches!(e, CoreError::Network(_)) {
                    self.offline = true;
                    self.retry_step += 1;
                    let espera = retry_delay(self.retry_step);
                    self.retry_at = Some(std::time::Instant::now() + espera);
                    message = format!(
                        "Sem conexão com o Spotify. Tentando de novo em {} s.",
                        espera.as_secs()
                    );
                } else {
                    // Credencial recusada, plano, formato: esperar a rede nao
                    // resolve, e o aviso de "sem conexao" mentiria.
                    self.offline = false;
                    self.retry_at = None;
                    self.retry_step = 0;
                }
                self.state = SessionState::Failed(message.clone());
                Some(SessionChange {
                    message,
                    engine: None,
                })
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
        CoreError::Network(_) => "Sem conexão com o Spotify. Verifique a internet.".into(),
        CoreError::Cancelled => "Login cancelado.".into(),
        CoreError::TimedOut => {
            "O tempo para entrar acabou. Clique em Entrar no Spotify para tentar de novo.".into()
        }
        CoreError::AudioDevice(_) => "Nenhum dispositivo de áudio disponível.".into(),
        // A frase tecnica do erro vai para o log, nao para a barra de
        // status: "Nao foi possivel entrar: Io(Os { code: 10061 ... })"
        // nao ajuda ninguem a entrar.
        other => {
            tracing::error!(error = %other, "login no Spotify falhou");
            "Não foi possível entrar no Spotify. Tente de novo em instantes.".into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use morune_core::auth::MemoryCredentialStore;

    fn session() -> Session {
        // Cache desligado: um teste nao pode criar pasta de audio na maquina de
        // quem roda a suite.
        let audio = morune_core::playback::AudioSettings {
            cache_mb: 0,
            ..Default::default()
        };
        Session::new(
            Arc::new(MemoryCredentialStore::default()),
            std::env::temp_dir(),
            audio,
            std::path::Path::new(""),
        )
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
    fn a_login_during_the_silent_restore_asks_to_wait() {
        let mut session = session();
        session.restore();
        if !session.is_busy() {
            // A restauracao pode ter terminado antes desta linha; nesse caso o
            // teste nao tem o que verificar.
            return;
        }
        assert!(session.login().contains("Um instante"));
    }

    #[test]
    fn a_espera_entre_tentativas_cresce_e_para_em_um_minuto() {
        let segundos: Vec<u64> = (1..=6).map(|p| retry_delay(p).as_secs()).collect();
        assert_eq!(segundos, [5, 10, 30, 60, 60, 60]);
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
        assert!(describe(&CoreError::TimedOut).contains("tentar de novo"));
    }

    #[test]
    fn an_account_that_cannot_stream_is_told_why_and_not_asked_to_retry() {
        // O caso mais comum de quem baixa um player aberto: conta gratuita.
        // A frase vem do backend inteira, sem "tente de novo" grudado nela.
        let message = describe(&CoreError::AccountPlan(
            "O Spotify só entrega música para contas Premium, e esta é free.".into(),
        ));
        assert!(message.contains("Premium"));
        assert!(!message.contains("Não foi possível entrar"));
    }

    use std::time::Duration;
}
