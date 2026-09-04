//! Temas que acompanham o aplicativo.
//!
//! Ficam embutidos no binario e sao gravados na pasta de temas do usuario na
//! primeira execucao. Duas consequencias importantes:
//!
//! - o aplicativo funciona logo depois de instalado, sem depender de arquivos
//!   soltos ao lado do executavel;
//! - o usuario pode abrir, editar e duplicar esses temas como qualquer outro,
//!   em vez de eles serem uma caixa preta.
//!
//! Um tema ja existente nunca e sobrescrito: edicoes do usuario tem prioridade
//! sobre a copia que veio conosco.

use std::path::Path;

/// Conteudo de um arquivo empacotado.
///
/// Texto e bytes ficam separados porque a imagem de fundo do Aquario nao e
/// UTF-8: `include_str!` nem compilaria com ela, e gravar TOML como bytes
/// esconderia um arquivo mal formado ate a hora de carregar.
enum BundledContents {
    Text(&'static str),
    Bytes(&'static [u8]),
}

/// Um arquivo de um tema empacotado.
struct BundledFile {
    name: &'static str,
    contents: BundledContents,
}

/// Um tema empacotado.
struct BundledTheme {
    id: &'static str,
    files: &'static [BundledFile],
}

const THEMES: &[BundledTheme] = &[
    BundledTheme {
        id: "paper",
        files: &[
            BundledFile {
                name: "manifest.toml",
                contents: BundledContents::Text(include_str!("../themes/paper/manifest.toml")),
            },
            BundledFile {
                name: "theme.toml",
                contents: BundledContents::Text(include_str!("../themes/paper/theme.toml")),
            },
            BundledFile {
                name: "layout.toml",
                contents: BundledContents::Text(include_str!("../themes/paper/layout.toml")),
            },
        ],
    },
    BundledTheme {
        id: "bruma",
        files: &[
            BundledFile {
                name: "manifest.toml",
                contents: BundledContents::Text(include_str!("../themes/bruma/manifest.toml")),
            },
            BundledFile {
                name: "theme.toml",
                contents: BundledContents::Text(include_str!("../themes/bruma/theme.toml")),
            },
        ],
    },
    BundledTheme {
        id: "cristal",
        files: &[
            BundledFile {
                name: "manifest.toml",
                contents: BundledContents::Text(include_str!("../themes/cristal/manifest.toml")),
            },
            BundledFile {
                name: "theme.toml",
                contents: BundledContents::Text(include_str!("../themes/cristal/theme.toml")),
            },
        ],
    },
    BundledTheme {
        id: "pulse",
        files: &[
            BundledFile {
                name: "manifest.toml",
                contents: BundledContents::Text(include_str!("../themes/pulse/manifest.toml")),
            },
            BundledFile {
                name: "theme.toml",
                contents: BundledContents::Text(include_str!("../themes/pulse/theme.toml")),
            },
        ],
    },
    BundledTheme {
        id: "aquario",
        files: &[
            BundledFile {
                name: "manifest.toml",
                contents: BundledContents::Text(include_str!("../themes/aquario/manifest.toml")),
            },
            BundledFile {
                name: "theme.toml",
                contents: BundledContents::Text(include_str!("../themes/aquario/theme.toml")),
            },
            BundledFile {
                name: "layout.toml",
                contents: BundledContents::Text(include_str!("../themes/aquario/layout.toml")),
            },
            BundledFile {
                name: "fundo.png",
                contents: BundledContents::Bytes(include_bytes!("../themes/aquario/fundo.png")),
            },
        ],
    },
];

/// Grava os temas que ainda nao existem em `themes_dir`.
///
/// Devolve os ids instalados nesta chamada. Erros sao registrados e ignorados:
/// nao poder gravar um tema de exemplo nao e motivo para o aplicativo nao abrir.
pub fn install_missing(themes_dir: &Path) -> Vec<&'static str> {
    let mut installed = Vec::new();

    for theme in THEMES {
        let dir = themes_dir.join(theme.id);
        if dir.exists() {
            continue;
        }
        if let Err(e) = std::fs::create_dir_all(&dir) {
            tracing::warn!(theme = theme.id, error = %e, "nao foi possivel criar a pasta do tema");
            continue;
        }

        let mut ok = true;
        for file in theme.files {
            let escrita = match file.contents {
                BundledContents::Text(t) => std::fs::write(dir.join(file.name), t),
                BundledContents::Bytes(b) => std::fs::write(dir.join(file.name), b),
            };
            if let Err(e) = escrita {
                tracing::warn!(theme = theme.id, file = file.name, error = %e, "falha ao gravar");
                ok = false;
            }
        }
        if ok {
            installed.push(theme.id);
        }
    }

    installed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("morune-bundled-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn every_bundled_theme_parses_and_loads() {
        let dir = temp_dir("parse");
        let installed = install_missing(&dir);
        assert_eq!(installed.len(), THEMES.len());

        for theme in THEMES {
            let loaded = morune_theme::load(&dir, theme.id);
            assert!(!loaded.fell_back, "{} caiu para o embutido", theme.id);
            assert!(
                loaded.errors.is_empty(),
                "{}: {:?}",
                theme.id,
                loaded.errors
            );
            assert!(
                loaded.warnings.is_empty(),
                "{}: {:?}",
                theme.id,
                loaded.warnings
            );
            assert_eq!(loaded.spec.manifest.id, theme.id);
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bundled_themes_are_visually_distinct_from_the_builtin() {
        let dir = temp_dir("distinct");
        install_missing(&dir);

        let builtin = morune_theme::ThemeSpec::default();
        let paper = morune_theme::load(&dir, "paper").spec;

        assert_ne!(paper.colors.background, builtin.colors.background);
        assert_ne!(
            paper.layout.sidebar.position,
            builtin.layout.sidebar.position
        );
        assert_ne!(paper.layout.player.position, builtin.layout.player.position);
        assert_ne!(
            paper.layout.content.view_mode,
            builtin.layout.content.view_mode
        );
        assert_ne!(paper.shape.radius_md, builtin.shape.radius_md);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn derived_theme_inherits_layout_but_overrides_color() {
        let dir = temp_dir("derived");
        install_missing(&dir);

        let builtin = morune_theme::ThemeSpec::default();
        let pulse = morune_theme::load(&dir, "pulse").spec;

        // Pulse nao declara layout.toml: a composicao tem de vir do pai.
        assert_eq!(pulse.layout, builtin.layout);
        assert_ne!(pulse.colors.accent, builtin.colors.accent);
        assert_ne!(pulse.shape.radius_lg, builtin.shape.radius_lg);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Os quatro temas que acompanham o aplicativo nao podem sair de fabrica
    /// reprovando contraste.
    ///
    /// Este teste existe porque `border` e `scrollbar` reprovaram 3:1 nos
    /// quatro ao mesmo tempo, e nada acusou: a validacao rodava a cada boot,
    /// mas so olhava texto, e nenhum teste conferia os temas embutidos com ela.
    /// A crate de tema so avisa -- proibir seria proibir tema de baixo
    /// contraste de proposito, que e escolha de quem faz o tema. Nos temas de
    /// fabrica, porem, o aviso e defeito nosso, e aqui ele reprova.
    #[test]
    fn temas_de_fabrica_nao_reprovam_contraste() {
        let dir = temp_dir("contraste");
        install_missing(&dir);

        let mut culpados = Vec::new();
        for id in ["paper", "bruma", "cristal", "pulse", "aquario"] {
            for aviso in morune_theme::load(&dir, id).spec.contrast_warnings() {
                culpados.push(format!("{id}: {} -- {}", aviso.field, aviso.message));
            }
        }

        let _ = std::fs::remove_dir_all(&dir);
        assert!(
            culpados.is_empty(),
            "tema de fabrica reprovando contraste:\n{}",
            culpados.join("\n")
        );
    }

    #[test]
    fn existing_themes_are_never_overwritten() {
        let dir = temp_dir("preserve");
        install_missing(&dir);

        let edited = dir.join("paper").join("theme.toml");
        std::fs::write(&edited, "[color]\naccent = \"#123456\"\n").unwrap();

        let installed = install_missing(&dir);
        assert!(installed.is_empty(), "reinstalou por cima: {installed:?}");
        assert!(std::fs::read_to_string(&edited).unwrap().contains("123456"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
