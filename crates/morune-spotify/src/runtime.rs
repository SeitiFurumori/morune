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
use morune_core::catalog::{Artwork, Catalog, Library};
use morune_core::playback::PlaybackEngine;
use morune_core::{CoreError, CoreResult};

use crate::auth::{SharedSession, SpotifyAuthenticator};
use crate::artwork::SpotifyArtwork;
use crate::catalog::SpotifyCatalog;
use crate::engine::SpotifyEngine;
use crate::token::TokenSource;

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
    catalog: Arc<SpotifyCatalog>,
    artwork: Arc<SpotifyArtwork>,
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

        // O catalogo nao recebe mais a fonte de token: depois que o Web API
        // saiu do caminho, tudo que ele faz passa pela propria sessao da
        // librespot, que renova o que precisa sozinha. So o login ainda guarda
        // token, e por causa do refresh no cofre do Windows.
        let session = SharedSession::default();
        let tokens = Arc::new(TokenSource::new(credentials));
        let authenticator =
            Arc::new(SpotifyAuthenticator::with_tokens(tokens.clone(), session.clone()));
        let catalog = Arc::new(SpotifyCatalog::new(session.clone()));
        let artwork = Arc::new(SpotifyArtwork::new(session.clone()));

        Ok(Self { runtime, authenticator, catalog, artwork, session })
    }

    /// Autenticador, para a aplicacao ligar aos botoes de entrar e sair.
    pub fn authenticator(&self) -> Arc<dyn Authenticator> {
        self.authenticator.clone()
    }

    /// Catalogo do provedor, para busca e navegacao.
    ///
    /// Existe desde a abertura, mesmo sem login: as requisicoes e que falham
    /// com [`CoreError::NotAuthenticated`] enquanto nao ha sessao. Assim a
    /// interface guarda uma referencia so, e nao um `Option` que ela teria de
    /// checar em cada tela.
    pub fn catalog(&self) -> Arc<dyn Catalog> {
        self.catalog.clone()
    }

    /// `true` quando havia sessao e ela caiu.
    ///
    /// O Spotify derruba sessao ociosa e a rede cai sozinha; sem isto o
    /// aplicativo ficava com uma sessao morta na mao e toda requisicao
    /// respondia `channel closed`.
    pub fn session_lost(&self) -> bool {
        self.session.is_lost()
    }

    /// Descarta a sessao morta antes de tentar de novo.
    pub fn forget_lost_session(&self) {
        self.session.forget_lost();
    }

    /// Fonte de capas, para a camada que as guarda em disco.
    pub fn artwork(&self) -> Arc<dyn Artwork> {
        self.artwork.clone()
    }

    /// Biblioteca do usuario. Mesmo objeto do catalogo, outro contrato.
    pub fn library(&self) -> Arc<dyn Library> {
        self.catalog.clone()
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
    fn the_catalog_refuses_work_before_login_instead_of_panicking() {
        // A interface guarda o catalogo desde a abertura; sem sessao ele
        // precisa recusar com o erro que a tela sabe traduzir.
        let backend = backend();
        let catalog = backend.catalog();
        let result = backend.block_on(catalog.search("qualquer coisa", Default::default(), 10));
        assert!(matches!(result, Err(CoreError::NotAuthenticated)));
    }

    #[test]
    fn an_empty_query_never_reaches_the_network() {
        // Buscar por nada nao e erro nem requisicao: e uma lista vazia.
        let backend = backend();
        let catalog = backend.catalog();
        let results =
            backend.block_on(catalog.search("   ", Default::default(), 10)).expect("sem rede");
        assert!(results.is_empty());
    }

    #[test]
    fn restoring_without_a_stored_session_yields_no_profile() {
        let backend = backend();
        let profile = backend.block_on(backend.authenticator().restore()).unwrap();
        assert!(profile.is_none());
    }
}
