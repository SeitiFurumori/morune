//! Persistencia local do Morune: caminhos, configuracao e cofre de credenciais.
//!
//! Duas regras valem para tudo aqui:
//!
//! 1. **Nada perde dado do usuario em silencio.** Configuracao corrompida vira
//!    `.bak` antes de ser substituida; gravacao usa arquivo temporario mais
//!    renomeacao para que uma interrupcao no meio nao trunque o original.
//! 2. **Segredo nunca vai para arquivo nosso.** Tokens moram no Gerenciador de
//!    Credenciais do Windows, atras do contrato
//!    [`morune_core::auth::CredentialStore`].

#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(missing_debug_implementations, rust_2018_idioms)]

pub mod config;
pub mod credentials;
pub mod paths;

pub use config::{Config, ConfigError};
pub use credentials::platform_store;
pub use paths::AppPaths;
