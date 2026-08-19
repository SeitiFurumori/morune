//! Runtime dedicado do backend.
//!
//! A aplicacao roda no laco de eventos do Slint, que e sincrono e mora na
//! thread principal. A librespot e async e precisa de um executor. Este modulo
//! e a fronteira entre os dois mundos: um runtime tokio proprio, em threads
//! separadas, com o qual a interface conversa apenas por canal.
//!
//! O runtime e criado com poucas threads de proposito. O trabalho pesado da
//! reproducao acontece na thread de audio da librespot; o executor aqui so
//! carrega faixa, fala com a rede e traduz evento. Deixar o tokio abrir uma
//! thread por nucleo daria ao Morune mais presenca no escalonador do que ele
//! precisa -- e este aplicativo divide a maquina com um jogo.

use std::sync::Arc;

use morune_core::auth::{Authenticator, CredentialStore};
use morune_core::playback::PlaybackEngine;
use morune_core::{CoreError, CoreResult};

use crate::auth::{SharedSession, SpotifyAuthenticator};
use crate::engine::SpotifyEngine;

/// Numero de threads de trabalho do runtime.
///
/// Duas bastam: uma para o I/O da sessao e outra para nao bloquear enquanto a
/// primeira espera. Mais do que isso e presenca desnecessaria no escalonador.
const WORKER_THREADS: usize = 2;

/// Backend de Spotify pronto para uso.
///
/// Dono do runtime: enquanto este valor viver, a sessao vive. Descartar encerra
/// tudo de forma ordenada.
pub struct SpotifyBackend {
    runtime: tokio::runtime::Runtime,
    authenticator: Arc<SpotifyAuthenticator>,
    session: SharedSession,
}

impl std::fmt::Debug for SpotifyBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpotifyBackend").field("session", &self.session).finish()
    }
}

impl SpotifyBackend {
    /// Sobe o runtime do backend.
    ///
    /// Nao conecta nada: sem isto, abrir o aplicativo dependeria de rede, e o
    /// orcamento de startup nao permite. O login vem depois, por
    /// [`SpotifyBackend::restore`] ou pela acao do usuario.
    pub fn new(credentials: Arc<dyn CredentialStore>) -> CoreResult<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(WORKER_THREADS)
            .thread_name("morune-spotify")
            .enable_all()
            .build()
            .map_err(|e| CoreError::InvalidState(format!("runtime do Spotify: {e}")))?;

        let session = SharedSession::default();
        let authenticator =
            Arc::new(SpotifyAuthenticator::new(credentials, session.clone()));

        Ok(Self { runtime, authenticator, session })
    }

    /// Autenticador, para a aplicacao ligar aos botoes de entrar e sair.
    pub fn authenticator(&self) -> Arc<dyn Authenticator> {
        self.authenticator.clone()
    }

    /// Handle do runtime, para quem precisa agendar trabalho no mesmo executor.
    pub fn handle(&self) -> tokio::runtime::Handle {
        self.runtime.handle().clone()
    }

    /// `true` quando ha sessao conectada e o motor pode ser criado.
    pub fn is_connected(&self) -> bool {
        self.session.get().is_some()
    }

    /// Cria o motor de reproducao sobre a sessao ativa.
    ///
    /// Falha com [`CoreError::NotAuthenticated`] enquanto nao houver login --
    /// e a aplicacao continua com o `NullEngine`, sem tela quebrada.
    pub fn engine(&self) -> CoreResult<Arc<dyn PlaybackEngine>> {
        let engine = SpotifyEngine::new(self.session.clone(), self.handle())?;
        Ok(Arc::new(engine))
    }

    /// Executa um future do backend a partir da thread da interface.
    ///
    /// Bloqueia quem chama, entao **nao** serve para o caminho de um clique.
    /// Existe para a restauracao de sessao na abertura e para testes.
    pub fn block_on<F: std::future::Future>(&self, future: F) -> F::Output {
        self.runtime.block_on(future)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use morune_core::auth::MemoryCredentialStore;

    fn backend() -> SpotifyBackend {
        SpotifyBackend::new(Arc::new(MemoryCredentialStore::default())).expect("runtime sobe")
    }

    #[test]
    fn starts_without_touching_the_network() {
        // Se subir o backend exigisse rede, abrir o aplicativo sem internet
        // ficaria preso aqui -- e o orcamento de startup nao tem esse espaco.
        let backend = backend();
        assert!(!backend.is_connected());
    }

    #[test]
    fn engine_is_refused_before_login_instead_of_panicking() {
        let backend = backend();
        assert!(matches!(backend.engine(), Err(CoreError::NotAuthenticated)));
    }

    #[test]
    fn restoring_without_a_stored_session_yields_no_profile() {
        let backend = backend();
        let profile = backend.block_on(backend.authenticator().restore()).unwrap();
        assert!(profile.is_none());
    }
}
