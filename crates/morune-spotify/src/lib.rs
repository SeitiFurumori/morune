//! Backend de Spotify do Morune.
//!
//! Implementa os contratos de `morune-core` sobre a librespot. Nada daqui e
//! visivel para a interface: a aplicacao guarda `Arc<dyn PlaybackEngine>` e
//! `Arc<dyn Authenticator>`, e trocar de backend nao toca em nenhuma tela.
//!
//! # Como isto se encaixa
//!
//! ```text
//!   interface (Slint)          thread da interface, nunca espera
//!        |  comandos             |  eventos e retrato
//!        v                       ^
//!   SpotifyEngine  ----------- runtime tokio proprio ----> librespot
//!        ^                                                    |
//!   SpotifyAuthenticator ---- mesma Session -------------------+
//!        ^                                                    |
//!   SpotifyCatalog ------ mesmo token, sobre HTTP ------> Web API
//! ```
//!
//! A [`SharedSession`] e o que amarra os tres: o login abre a conexao, o motor
//! toca por ela e o catalogo usa o cliente HTTP dela. Duas conexoes gastariam
//! dois slots de dispositivo na conta, e o Spotify derrubaria uma delas.
//!
//! A divisao entre os dois canais nao e escolha: o protocolo da librespot
//! entrega audio, e busca e biblioteca so existem no Web API.
//!
//! # Precisa de conta Premium
//!
//! O protocolo so entrega audio para contas Premium. Uma conta gratuita conecta
//! e falha na primeira faixa -- por isso o erro de reproducao e reportado como
//! evento, e nao como falha de login.

#![deny(unsafe_code)]

mod artwork;
mod auth;
mod catalog;
mod engine;
mod error;
mod graphql;
mod internal;
mod pathfinder;
mod runtime;
mod sink;
mod token;

pub use auth::{SharedSession, SpotifyAuthenticator};
pub use catalog::SpotifyCatalog;
pub use engine::SpotifyEngine;
pub use runtime::SpotifyBackend;

/// Le um rootlist cru e devolve `(nome, dono, tamanho, formato)` de cada
/// playlist.
///
/// Existe **so para o exemplo `rootlist`**, que confere a leitura contra a
/// resposta gravada de uma conta real sem precisar de login. Nao e usado pelo
/// aplicativo, e o tipo devolvido e proposital: uma tupla nao tenta parecer
/// parte do contrato publico.
#[doc(hidden)]
pub fn debug_rootlist(bytes: &[u8]) -> Result<Vec<(String, String, u32, String)>, String> {
    internal::summaries_for_debug(bytes)
}
