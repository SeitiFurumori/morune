//! Motor de customizacao do Morune.
//!
//! Um tema e **declarativo**: TOML com tokens e composicao de tela. Nao existe
//! script, expressao nem qualquer forma de codigo executavel dentro de um
//! pacote, e essa e uma decisao de seguranca, nao uma limitacao temporaria --
//! ver `SECURITY.md`.
//!
//! Garantias que esta crate oferece a quem esta acima dela:
//!
//! 1. [`loader::load`] **nunca falha**. Um tema corrompido vira o tema embutido
//!    mais uma lista de diagnosticos.
//! 2. Todo [`ThemeSpec`] entregue ja passou por `sanitize`, entao a interface
//!    nao precisa validar nada.
//! 3. Nada de dentro de um `.musicpack` e escrito em disco antes de passar por
//!    [`pack::sanitize_entry_path`] e [`pack::check_entry_allowed`].

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations, rust_2018_idioms)]

pub mod color;
pub mod layout;
pub mod loader;
pub mod manifest;
pub mod pack;
pub mod spec;
pub mod tokens;

#[cfg(feature = "hot-reload")]
pub mod watch;

pub use color::Color;
pub use layout::{
    ContentLayout, Density, LayoutSpec, PlayerLayout, PlayerPosition, SidebarLayout,
    SidebarPosition, ViewMode, WindowLayout,
};
pub use loader::{discover, load, LoadedTheme, ThemeEntry};
pub use manifest::{Appearance, ThemeManifest, CURRENT_SCHEMA_VERSION};
pub use pack::{export_pack, import_pack, ImportedTheme, PackError};
pub use spec::{ThemeSpec, ThemeWarning};
pub use tokens::{ColorTokens, Easing, EffectTokens, MotionTokens, ShapeTokens, TypographyTokens};

/// Extensao dos pacotes de tema.
pub const PACK_EXTENSION: &str = "musicpack";
