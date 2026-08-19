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
//! ```
//!
//! A [`SharedSession`] e o que amarra os dois: o login abre a conexao e o motor
//! usa a mesma. Duas conexoes gastariam dois slots de dispositivo na conta, e o
//! Spotify derrubaria uma delas.
//!
//! # Precisa de conta Premium
//!
//! O protocolo so entrega audio para contas Premium. Uma conta gratuita conecta
//! e falha na primeira faixa -- por isso o erro de reproducao e reportado como
//! evento, e nao como falha de login.

#![deny(unsafe_code)]

mod auth;
mod engine;
mod error;
mod runtime;

pub use auth::{SharedSession, SpotifyAuthenticator};
pub use engine::SpotifyEngine;
pub use runtime::SpotifyBackend;
