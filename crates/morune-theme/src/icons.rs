//! Icones substituiveis por tema.
//!
//! Um tema pode trocar qualquer icone da interface pondo um `.svg` de mesmo
//! nome em `assets/icons/`. O que ele **nao** pode e inventar nomes: a lista
//! abaixo e fechada, e um arquivo fora dela e ignorado com aviso.
//!
//! A lista fechada nao e burocracia. Sem ela, um erro de digitacao no nome do
//! arquivo viraria silencio -- o icone simplesmente nao apareceria trocado e o
//! autor do tema nao teria como saber por que.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Nomes de icone que a interface conhece.
///
/// Precisa acompanhar o `global Icons` de `app.slint`. Um nome que existe aqui
/// e nao la simplesmente nunca e usado; o contrario deixa um icone sem como ser
/// substituido, que e o defeito a evitar.
pub const ICON_NAMES: &[&str] = &[
    "home",
    "search",
    "library",
    "settings",
    "play",
    "pause",
    "stop",
    "next",
    "previous",
    "shuffle",
    "repeat",
    "repeat-one",
    "queue",
    "mini-player",
    "volume",
    "chevron-left",
    "chevron-right",
    "close",
    "heart",
    "queue-add",
    "queue-next",
    "chevron-up",
    "chevron-down",
    "minimize-window",
    "maximize-window",
    "restore-window",
];

/// Subpasta, dentro do tema, onde os substitutos ficam.
pub const ICONS_SUBDIR: &str = "assets/icons";

/// Encontra os icones que um tema substitui.
///
/// Devolve so o que existe: um tema que troca dois icones devolve dois pares, e
/// os outros vinte e quatro continuam vindo do desenho embutido.
///
/// So `.svg`. Um PNG serviria, mas perderia nitidez em 150% e 200% de DPI --
/// que sao as escalas em que a maior parte das pessoas usa o Windows.
pub fn resolve(theme_dir: &Path) -> HashMap<&'static str, PathBuf> {
    let dir = theme_dir.join(ICONS_SUBDIR);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return HashMap::new();
    };

    // Canonicalizar a raiz uma vez, e nao a cada arquivo: e a comparacao que
    // garante que um link simbolico dentro de `assets/icons` nao consiga
    // apontar para fora do tema.
    let Ok(root) = theme_dir.canonicalize() else {
        return HashMap::new();
    };

    let mut found = HashMap::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("svg") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(name) = ICON_NAMES.iter().find(|n| **n == stem) else {
            tracing::warn!(
                arquivo = stem,
                "icone com nome desconhecido, ignorado; ver ICON_NAMES"
            );
            continue;
        };
        let Ok(full) = path.canonicalize() else {
            continue;
        };
        if !full.starts_with(&root) {
            tracing::warn!(path = %full.display(), "icone aponta para fora do tema, ignorado");
            continue;
        }
        found.insert(*name, full);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let p = std::env::temp_dir().join(format!("morune-icons-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(p.join(ICONS_SUBDIR)).unwrap();
            Self(p)
        }

        fn icon(&self, name: &str) {
            std::fs::write(self.0.join(ICONS_SUBDIR).join(name), "<svg/>").unwrap();
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_theme_without_icons_replaces_nothing() {
        let dir = TempDir::new("vazio");
        assert!(resolve(&dir.0).is_empty());
        // Nem sequer a pasta precisa existir.
        assert!(resolve(&dir.0.join("nao-existe")).is_empty());
    }

    #[test]
    fn only_the_declared_names_are_accepted() {
        let dir = TempDir::new("nomes");
        dir.icon("play.svg");
        dir.icon("pause.svg");
        // Erro de digitacao: nao vira icone nenhum, e o aviso fica no log.
        dir.icon("paly.svg");
        // Nome valido, formato errado.
        dir.icon("stop.png");

        let found = resolve(&dir.0);
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found.contains_key("play"));
        assert!(found.contains_key("pause"));
        assert!(!found.contains_key("stop"));
    }

    #[test]
    fn every_declared_name_is_unique() {
        let mut seen = ICON_NAMES.to_vec();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(before, seen.len(), "nome de icone repetido em ICON_NAMES");
    }
}
