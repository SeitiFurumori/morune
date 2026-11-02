use std::path::{Path, PathBuf};

use directories::ProjectDirs;

/// Onde o Otomori guarda cada tipo de dado.
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
        match ProjectDirs::from("dev", "otomori", "Otomori") {
            Some(dirs) => Self::from_dirs(&dirs),
            None => Self::portable(&std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))),
        }
    }

    /// Onde a 1.0, ainda com o nome Morune, guardava tudo.
    fn legado() -> Option<Self> {
        ProjectDirs::from("dev", "morune", "Morune").map(|dirs| Self::from_dirs(&dirs))
    }

    fn from_dirs(dirs: &ProjectDirs) -> Self {
        Self {
            config_dir: dirs.config_dir().to_path_buf(),
            cache_dir: dirs.cache_dir().to_path_buf(),
            data_dir: dirs.data_dir().to_path_buf(),
        }
    }

    /// Traz configuracao e dados da pasta do Morune na primeira abertura.
    ///
    /// Copia, nao move: a pasta antiga fica intacta como reserva, e uma copia
    /// interrompida no meio nao perde nada -- a proxima abertura tenta de
    /// novo, porque so o fim da copia cria a pasta nova de vez. O cache fica
    /// para tras: e descartavel por definicao, e o de audio pode ter gigabytes.
    ///
    /// Devolve `true` quando copiou.
    pub fn migrate_from_morune(&self) -> std::io::Result<bool> {
        match Self::legado() {
            Some(antigo) => self.migrate_from(&antigo),
            None => Ok(false),
        }
    }

    fn migrate_from(&self, antigo: &AppPaths) -> std::io::Result<bool> {
        if self.config_dir.exists() || !antigo.config_dir.is_dir() {
            return Ok(false);
        }
        let pares = [
            (&antigo.data_dir, &self.data_dir),
            (&antigo.config_dir, &self.config_dir),
        ];
        for (de, para) in pares {
            if !de.is_dir() {
                continue;
            }
            // Copia ao lado e renomeia no fim: a pasta de config so passa a
            // existir quando esta completa, e e ela que diz "ja migrou".
            let parcial = para.with_extension("migrando");
            let _ = std::fs::remove_dir_all(&parcial);
            copy_dir(de, &parcial)?;
            if let Some(pai) = para.parent() {
                std::fs::create_dir_all(pai)?;
            }
            if para.exists() {
                std::fs::remove_dir_all(para)?;
            }
            std::fs::rename(&parcial, para)?;
        }
        // O fundo escolhido e guardado com caminho completo, dentro da pasta
        // antiga. Sem trocar, a tela recusaria a imagem: so serve arquivo das
        // pastas do app atual.
        let mut trocas = Vec::new();
        for (de, para) in [
            (&antigo.data_dir, &self.data_dir),
            (&antigo.config_dir, &self.config_dir),
        ] {
            let de = de.to_string_lossy().to_string();
            let para = para.to_string_lossy().to_string();
            // Primeiro a forma escapada, `\\` dentro de aspas duplas; depois a
            // crua, de aspas simples.
            trocas.push((de.replace('\\', "\\\\"), para.replace('\\', "\\\\")));
            trocas.push((de, para));
        }
        trocar_caminho_nos_toml(&self.config_dir, &trocas)?;
        Ok(true)
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

    /// Biblioteca que pertence ao Morune, nao a um provedor externo.
    pub fn favorites_file(&self) -> PathBuf {
        self.data_dir.join("favorites.toml")
    }

    /// Historico local: tocadas recentemente, buscas recentes, faixas
    /// escondidas. Ver [`crate::History`].
    pub fn history_file(&self) -> PathBuf {
        self.data_dir.join("history.toml")
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

    /// Imagens de fundo escolhidas pelo usuario.
    ///
    /// Fica em `data_dir`, e nao em `cache_dir`: e escolha da pessoa, e limpar
    /// o cache nao pode deixar a janela sem fundo.
    pub fn backgrounds_dir(&self) -> PathBuf {
        self.data_dir.join("backgrounds")
    }

    /// Cache de audio do backend de streaming.
    pub fn audio_cache_dir(&self) -> PathBuf {
        self.cache_dir.join("audio")
    }

    /// Instaladores baixados pelo botao de atualizar.
    ///
    /// Fica em `cache_dir` porque e descartavel por definicao: depois de
    /// instalar, o arquivo so ocupa espaco, e apagar a pasta inteira nao perde
    /// nada que o usuario tenha criado.
    pub fn updates_dir(&self) -> PathBuf {
        self.cache_dir.join("updates")
    }

    pub fn log_file(&self) -> PathBuf {
        self.data_dir.join("otomori.log")
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

/// Aplica as trocas `(velho, novo)` em todo `.toml` dentro de `pasta`.
fn trocar_caminho_nos_toml(pasta: &Path, trocas: &[(String, String)]) -> std::io::Result<()> {
    for entrada in std::fs::read_dir(pasta)? {
        let caminho = entrada?.path();
        if caminho.is_dir() {
            trocar_caminho_nos_toml(&caminho, trocas)?;
            continue;
        }
        if caminho.extension().is_none_or(|e| e != "toml") {
            continue;
        }
        let Ok(texto) = std::fs::read_to_string(&caminho) else {
            continue;
        };
        let mut novo = texto.clone();
        for (velho, atual) in trocas {
            novo = novo.replace(velho.as_str(), atual);
        }
        if novo != texto {
            std::fs::write(&caminho, novo)?;
        }
    }
    Ok(())
}

/// Copia uma pasta inteira, com subpastas.
fn copy_dir(de: &Path, para: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(para)?;
    for entrada in std::fs::read_dir(de)? {
        let entrada = entrada?;
        let destino = para.join(entrada.file_name());
        if entrada.file_type()?.is_dir() {
            copy_dir(&entrada.path(), &destino)?;
        } else {
            std::fs::copy(entrada.path(), &destino)?;
        }
    }
    Ok(())
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
    fn downloaded_installers_are_discardable_cache() {
        let p = AppPaths::portable(Path::new("D:/morune"));
        assert!(p.updates_dir().starts_with(p.cache_dir()));
    }

    #[test]
    fn cache_is_separate_from_config() {
        let p = AppPaths::discover();
        assert_ne!(p.config_dir(), p.cache_dir());
    }

    #[test]
    fn favorites_are_user_data_not_discardable_cache() {
        let p = AppPaths::portable(Path::new("D:/morune"));
        assert!(p.favorites_file().starts_with(p.data_dir()));
        assert!(!p.favorites_file().starts_with(p.cache_dir()));
    }

    #[test]
    fn migration_copies_config_and_data_and_keeps_the_old_folder() {
        let root = std::env::temp_dir().join(format!("morune-migra-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let antigo = AppPaths::portable(&root.join("morune"));
        let novo = AppPaths::portable(&root.join("otomori"));
        std::fs::create_dir_all(antigo.theme_dir("meu")).unwrap();
        std::fs::write(antigo.config_file(), "tema = 'meu'").unwrap();
        let fundo_antigo = antigo.backgrounds_dir().join("chuva.mp4");
        let fundo_novo = novo.backgrounds_dir().join("chuva.mp4");
        std::fs::write(
            antigo.theme_dir("meu").parent().unwrap().join("fundo.toml"),
            format!(
                "cru = '{}'
escapado = {:?}
",
                fundo_antigo.display(),
                fundo_antigo.display().to_string()
            ),
        )
        .unwrap();
        std::fs::write(antigo.theme_dir("meu").join("theme.toml"), "x").unwrap();
        std::fs::create_dir_all(antigo.data_dir()).unwrap();
        std::fs::write(antigo.history_file(), "h").unwrap();
        std::fs::create_dir_all(antigo.audio_cache_dir()).unwrap();
        std::fs::write(antigo.audio_cache_dir().join("faixa"), "pesado").unwrap();

        assert!(novo.migrate_from(&antigo).unwrap());

        assert_eq!(
            std::fs::read_to_string(novo.config_file()).unwrap(),
            "tema = 'meu'"
        );
        assert!(novo.theme_dir("meu").join("theme.toml").is_file());
        assert!(novo.history_file().is_file());
        let fundo = std::fs::read_to_string(novo.themes_dir().join("fundo.toml")).unwrap();
        let velho = fundo_antigo.display().to_string();
        assert!(
            !fundo.contains(&velho) && !fundo.contains(&format!("{velho:?}")),
            "caminho antigo ficou no toml: {fundo}"
        );
        assert!(fundo.contains(&fundo_novo.display().to_string()));
        assert!(!novo.audio_cache_dir().exists(), "cache nao viaja");
        assert!(antigo.config_file().is_file(), "a pasta antiga fica");

        // Ja migrado: a segunda abertura nao copia por cima do que mudou.
        std::fs::write(novo.config_file(), "tema = 'outro'").unwrap();
        assert!(!novo.migrate_from(&antigo).unwrap());
        assert_eq!(
            std::fs::read_to_string(novo.config_file()).unwrap(),
            "tema = 'outro'"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn migration_without_an_old_install_does_nothing() {
        let root = std::env::temp_dir().join(format!("morune-migra0-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let novo = AppPaths::portable(&root.join("otomori"));
        assert!(!novo
            .migrate_from(&AppPaths::portable(&root.join("morune")))
            .unwrap());
        assert!(!novo.config_dir().exists());
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
