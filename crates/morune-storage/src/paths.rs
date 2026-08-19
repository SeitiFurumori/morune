use std::path::{Path, PathBuf};

use directories::ProjectDirs;

/// Onde o Morune guarda cada tipo de dado.
///
/// A separacao segue a convencao do Windows: configuracao e temas em
/// `%APPDATA%` (acompanha o perfil itinerante), cache em `%LOCALAPPDATA%`
/// (descartavel, nao deve viajar entre maquinas).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    config_dir: PathBuf,
    cache_dir: PathBuf,
    data_dir: PathBuf,
}

impl AppPaths {
    /// Caminhos padrao do sistema.
    ///
    /// Cai para uma pasta ao lado do executavel quando o sistema nao expoe
    /// diretorios de usuario -- e o que permite rodar em modo portatil de um
    /// pendrive sem nenhuma configuracao.
    pub fn discover() -> Self {
        match ProjectDirs::from("dev", "morune", "Morune") {
            Some(dirs) => Self {
                config_dir: dirs.config_dir().to_path_buf(),
                cache_dir: dirs.cache_dir().to_path_buf(),
                data_dir: dirs.data_dir().to_path_buf(),
            },
            None => Self::portable(&std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))),
        }
    }

    /// Todos os dados dentro de uma unica pasta, para modo portatil e teste.
    pub fn portable(root: &Path) -> Self {
        Self {
            config_dir: root.join("config"),
            cache_dir: root.join("cache"),
            data_dir: root.join("data"),
        }
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    /// Pasta de temas do usuario. Aberta pelo menu "abrir pasta do tema".
    pub fn themes_dir(&self) -> PathBuf {
        self.config_dir.join("themes")
    }

    pub fn theme_dir(&self, id: &str) -> PathBuf {
        self.themes_dir().join(id)
    }

    /// Cache de capas. Fica em cache_dir porque pode ser apagado a qualquer
    /// momento sem perda de informacao do usuario.
    pub fn artwork_cache_dir(&self) -> PathBuf {
        self.cache_dir.join("artwork")
    }

    /// Cache de audio do backend de streaming.
    pub fn audio_cache_dir(&self) -> PathBuf {
        self.cache_dir.join("audio")
    }

    pub fn log_file(&self) -> PathBuf {
        self.data_dir.join("morune.log")
    }

    /// Cria as pastas necessarias. Idempotente.
    pub fn ensure(&self) -> std::io::Result<()> {
        for dir in [&self.config_dir, &self.cache_dir, &self.data_dir] {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::create_dir_all(self.themes_dir())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_layout_keeps_everything_under_one_root() {
        let p = AppPaths::portable(Path::new("D:/morune"));
        for dir in [p.config_dir(), p.cache_dir(), p.data_dir()] {
            assert!(dir.starts_with("D:/morune"), "{dir:?} escapou da raiz");
        }
        assert!(p.themes_dir().starts_with("D:/morune"));
    }

    #[test]
    fn theme_directories_hang_off_the_themes_directory() {
        let p = AppPaths::portable(Path::new("D:/morune"));
        assert_eq!(p.theme_dir("midnight"), p.themes_dir().join("midnight"));
    }

    #[test]
    fn cache_is_separate_from_config() {
        let p = AppPaths::discover();
        assert_ne!(p.config_dir(), p.cache_dir());
    }

    #[test]
    fn ensure_creates_every_directory_and_is_idempotent() {
        let root = std::env::temp_dir().join(format!("morune-paths-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let p = AppPaths::portable(&root);

        p.ensure().unwrap();
        p.ensure().unwrap();

        assert!(p.config_dir().is_dir());
        assert!(p.cache_dir().is_dir());
        assert!(p.data_dir().is_dir());
        assert!(p.themes_dir().is_dir());

        let _ = std::fs::remove_dir_all(&root);
    }
}
