//! Nucleo do Morune: modelo de dominio, fila de reproducao e os contratos que
//! todo backend precisa satisfazer.
//!
//! Esta crate nao conhece Spotify, Slint, HTTP nem sistema de arquivos. Essa
//! restricao e o que garante que:
//!
//! - a logica de fila, shuffle e repeticao seja testavel sem I/O;
//! - trocar de provedor de musica nao toque na UI;
//! - a UI possa rodar contra implementacoes falsas em teste e em modo de tema.
//!
//! Os contratos principais sao [`playback::PlaybackEngine`], [`catalog::Catalog`],
//! [`catalog::Library`] e [`auth::Authenticator`].

#![forbid(unsafe_code)]
#![warn(missing_debug_implementations, rust_2018_idioms)]

pub mod auth;
pub mod catalog;
pub mod error;
pub mod model;
pub mod playback;
pub mod queue;

pub use error::{CoreError, CoreResult, ErrorKind};
pub use model::{
    Album, AlbumId, AlbumRef, Artist, ArtistId, ArtistRef, ImageRef, ImageSet, Playlist,
    PlaylistId, Provider, Track, TrackId,
};
pub use playback::{
    EngineCapabilities, NullEngine, PlaybackEngine, PlaybackState, PlayerCommand, PlayerEvent,
    PlayerSnapshot,
};
pub use catalog::Artwork;
pub use model::PlaylistKind;
pub use queue::{Queue, QueueOrigin, RepeatMode};

/// Versao da crate, exposta para telas de "sobre" e para log de diagnostico.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
