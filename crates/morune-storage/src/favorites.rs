use std::fs;
use std::path::Path;

use morune_core::Track;
use serde::{Deserialize, Serialize};

const FAVORITES_VERSION: u32 = 1;

/// Faixas que o usuario decidiu guardar no proprio Morune.
///
/// A colecao e local e independente do provedor: o id canonico de cada faixa
/// preserva a origem, mas nenhuma conta externa precisa receber permissao de
/// escrita para que o usuario monte sua biblioteca.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Favorites {
    version: u32,
    tracks: Vec<Track>,
}

impl Default for Favorites {
    fn default() -> Self {
        Self {
            version: FAVORITES_VERSION,
            tracks: Vec::new(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FavoritesError {
    #[error("erro de arquivo: {0}")]
    Io(#[from] std::io::Error),
    #[error("biblioteca invalida: {0}")]
    Parse(String),
}

impl Favorites {
    /// Le a biblioteca local. Um arquivo corrompido e preservado antes de a
    /// interface continuar com uma colecao vazia.
    pub fn load(path: &Path) -> Self {
        // Se o processo caiu entre guardar o arquivo anterior e promover o
        // novo, a copia transitoria ainda e a biblioteca mais recente integra.
        let previous = path.with_extension("toml.previous");
        let source = if path.exists() || !previous.exists() {
            path
        } else {
            previous.as_path()
        };
        let raw = match fs::read_to_string(source) {
            Ok(raw) => raw,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Self::default(),
            Err(e) => {
                tracing::warn!(error = %e, "biblioteca local ilegivel");
                return Self::default();
            }
        };

        match toml::from_str::<Self>(&raw) {
            Ok(mut favorites) => {
                favorites.version = FAVORITES_VERSION;
                favorites.deduplicate();
                favorites
            }
            Err(e) => {
                tracing::error!(error = %e, "biblioteca local invalida");
                let backup = path.with_extension("toml.bak");
                if let Err(e) = fs::write(&backup, &raw) {
                    tracing::warn!(error = %e, "nao foi possivel preservar a biblioteca invalida");
                }
                Self::default()
            }
        }
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn contains(&self, track: &Track) -> bool {
        self.tracks.iter().any(|saved| saved.id == track.id)
    }

    pub fn contains_id(&self, id: &morune_core::TrackId) -> bool {
        self.tracks.iter().any(|saved| saved.id == *id)
    }

    /// Alterna uma faixa e devolve `true` quando ela ficou salva.
    pub fn toggle(&mut self, track: Track) -> bool {
        if let Some(index) = self.tracks.iter().position(|saved| saved.id == track.id) {
            self.tracks.remove(index);
            false
        } else {
            // O que acabou de ser guardado aparece primeiro, como uma biblioteca
            // musical costuma se comportar.
            self.tracks.insert(0, track);
            true
        }
    }

    /// Grava sem deixar um arquivo parcialmente escrito. Quando ja existe uma
    /// biblioteca, ela vira uma copia transitoria ate a nova ocupar o lugar.
    pub fn save(&self, path: &Path) -> Result<(), FavoritesError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text =
            toml::to_string_pretty(self).map_err(|e| FavoritesError::Parse(e.to_string()))?;
        let temp = path.with_extension("toml.tmp");
        let previous = path.with_extension("toml.previous");
        fs::write(&temp, text)?;

        if path.exists() {
            if previous.exists() {
                fs::remove_file(&previous)?;
            }
            fs::rename(path, &previous)?;
        }

        if let Err(error) = fs::rename(&temp, path) {
            if previous.exists() {
                let _ = fs::rename(&previous, path);
            }
            return Err(error.into());
        }

        if previous.exists() {
            if let Err(error) = fs::remove_file(previous) {
                // A nova biblioteca ja esta no lugar. Falhar apenas na limpeza
                // nao pode fazer a interface desfazer um favorito que foi salvo.
                tracing::warn!(%error, "nao foi possivel limpar copia anterior da biblioteca");
            }
        }
        Ok(())
    }

    fn deduplicate(&mut self) {
        let mut seen = std::collections::HashSet::new();
        self.tracks.retain(|track| seen.insert(track.id.clone()));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use morune_core::model::{ArtistId, ArtistRef, TrackId};

    use super::*;

    fn track(id: &str, name: &str) -> Track {
        Track {
            id: TrackId::spotify(id),
            name: Arc::from(name),
            artists: vec![ArtistRef {
                id: ArtistId::spotify("artist"),
                name: Arc::from("Artista"),
            }],
            album: None,
            duration: Duration::from_secs(180),
            track_number: None,
            disc_number: None,
            explicit: false,
            playable: true,
        }
    }

    fn temp_file(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "morune-favorites-{name}-{}.toml",
            std::process::id()
        ))
    }

    #[test]
    fn toggle_adds_newest_first_and_removes_without_duplicates() {
        let mut favorites = Favorites::default();
        assert!(favorites.toggle(track("one", "Um")));
        assert!(favorites.toggle(track("two", "Dois")));
        assert_eq!(favorites.tracks()[0].id, TrackId::spotify("two"));
        assert!(!favorites.toggle(track("one", "Um atualizado")));
        assert_eq!(favorites.tracks().len(), 1);
    }

    #[test]
    fn save_then_load_survives_multiple_replacements() {
        let path = temp_file("round-trip");
        let _ = fs::remove_file(&path);

        let mut favorites = Favorites::default();
        favorites.toggle(track("one", "Um"));
        favorites.save(&path).unwrap();
        favorites.toggle(track("two", "Dois"));
        favorites.save(&path).unwrap();

        let loaded = Favorites::load(&path);
        assert_eq!(loaded.tracks().len(), 2);
        assert_eq!(loaded.tracks()[0].name.as_ref(), "Dois");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn corrupt_library_is_preserved_and_does_not_crash_startup() {
        let path = temp_file("corrupt");
        fs::write(&path, "isto nao e toml = [").unwrap();

        let loaded = Favorites::load(&path);
        assert!(loaded.tracks().is_empty());
        assert!(path.with_extension("toml.bak").exists());

        let _ = fs::remove_file(path.with_extension("toml.bak"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn interrupted_replacement_recovers_the_previous_library() {
        let path = temp_file("recover-previous");
        let previous = path.with_extension("toml.previous");
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(&previous);

        let mut favorites = Favorites::default();
        favorites.toggle(track("safe", "Ainda aqui"));
        fs::write(&previous, toml::to_string_pretty(&favorites).unwrap()).unwrap();

        let loaded = Favorites::load(&path);
        assert_eq!(loaded.tracks()[0].name.as_ref(), "Ainda aqui");
        let _ = fs::remove_file(previous);
    }
}
