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
            BundledFile {
                name: "assets/atmosfera.svg",
                contents: BundledContents::Text(include_str!(
                    "../themes/bruma/assets/atmosfera.svg"
                )),
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
            BundledFile {
                name: "assets/cidade-aero.jpg",
                contents: BundledContents::Bytes(include_bytes!(
                    "../themes/aquario/assets/cidade-aero.jpg"
                )),
            },
        ],
    },
];

/// Grava os temas embutidos que faltam, e atualiza os que ficaram para tras.
///
/// **Por que atualizar, e nao so instalar.** A versao anterior pulava qualquer
/// tema cuja pasta ja existisse, e o efeito foi este: uma correcao de contraste
/// feita nos temas de fabrica em agosto nunca chegou a quem ja os tinha --
/// meses depois, o Paper instalado ainda estava com a borda reprovada que o
/// projeto acreditava ter consertado. Tema embutido e parte do aplicativo, e
/// aplicativo que se atualiza tem de atualizar o que ele traz.
///
/// **O que decide e a `version` do manifesto**, comparada campo a campo. Tema
/// que o usuario editou nao perde a edicao em silencio: a pasta antiga vira
/// `<id>.bak` antes de a nova ser gravada. Perder trabalho alheio para entregar
/// uma correcao seria trocar um problema por um pior.
///
/// Devolve os ids instalados ou atualizados. Erros sao registrados e ignorados:
/// nao poder gravar um tema de exemplo nao e motivo para o aplicativo nao abrir.
/// Escolhe onde guardar a copia antiga de um tema que vai ser atualizado.
///
/// O nome comum e `<id>.bak`. Se ele ja existe e nao da para apagar -- no
/// Windows basta um arquivo de dentro estar aberto por outro programa, e o
/// `remove_dir_all` volta sem erro visivel enquanto a pasta continua la --, a
/// copia vai para `<id>.bak-<hora>`. Foi assim que a atualizacao do Aquario
/// ficou presa em 13/09/2026: o `.bak` de 05/09 nao saia, o `rename` falhava
/// com "pasta nao vazia" e o tema instalado nunca recebia a versao nova.
/// Desistir de atualizar por causa da copia antiga seria inverter a prioridade.
fn pasta_de_copia(themes_dir: &Path, id: &str) -> std::path::PathBuf {
    let padrao = themes_dir.join(format!("{id}.bak"));
    let _ = std::fs::remove_dir_all(&padrao);
    if !padrao.exists() {
        return padrao;
    }
    let carimbo = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    themes_dir.join(format!("{id}.bak-{carimbo}"))
}

pub fn install_missing(themes_dir: &Path) -> Vec<&'static str> {
    let mut installed = Vec::new();

    for theme in THEMES {
        let dir = themes_dir.join(theme.id);
        if dir.exists() {
            if !bundled_is_newer(theme, &dir) {
                continue;
            }
            let backup = pasta_de_copia(themes_dir, theme.id);
            if let Err(e) = std::fs::rename(&dir, &backup) {
                tracing::warn!(theme = theme.id, error = %e, "nao consegui guardar a copia antiga; tema mantido como esta");
                continue;
            }
            tracing::info!(
                theme = theme.id,
                "tema embutido atualizado; copia antiga em .bak"
            );
        }
        if let Err(e) = std::fs::create_dir_all(&dir) {
            tracing::warn!(theme = theme.id, error = %e, "nao foi possivel criar a pasta do tema");
            continue;
        }

        let mut ok = true;
        for file in theme.files {
            // Os assets de musicpack vivem em subpastas do tema.
            if let Some(parent) = dir.join(file.name).parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    tracing::warn!(theme = theme.id, file = file.name, error = %e, "falha ao criar pasta do asset");
                    ok = false;
                    continue;
                }
            }
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

/// A versao embutida e maior que a instalada?
///
/// Le so a linha `version` do manifesto dos dois lados, porque e o unico campo
/// que decide. Sem versao no **embutido** nada e substituido; sem versao no
/// **instalado**, a copia e anterior ao versionamento e recebe a atualizacao.
fn bundled_is_newer(theme: &BundledTheme, dir: &Path) -> bool {
    let instalada = std::fs::read_to_string(dir.join("manifest.toml"))
        .ok()
        .and_then(|texto| manifest_version(&texto));
    // Sem versao do lado instalado, a copia e **anterior ao versionamento** --
    // e nao "desconhecida". Todo tema embutido declara versao hoje, entao o
    // unico jeito de faltar la e a pasta ter sido gravada por uma versao do
    // Morune que nao versionava tema.
    //
    // Isto termina: depois da substituicao o manifesto instalado passa a ter
    // versao, e a proxima comparacao e normal. A primeira redacao disto
    // devolvia `false` aqui, por medo de reescrever a cada arranque, e o efeito
    // foi o Bruma nunca receber a propria correcao.
    let Some(instalada) = instalada else {
        return true;
    };

    let embutida = theme
        .files
        .iter()
        .find(|f| f.name == "manifest.toml")
        .and_then(|f| match f.contents {
            BundledContents::Text(t) => manifest_version(t),
            BundledContents::Bytes(_) => None,
        });
    let Some(embutida) = embutida else {
        return false;
    };

    embutida > instalada
}

/// `version = "1.2.3"` do manifesto, como tres numeros comparaveis.
///
/// Comparar como texto poria `1.10.0` antes de `1.9.0`, que e o erro classico
/// -- e um que este projeto ja pagou noutro lugar, na comparacao de versao do
/// atualizador.
fn manifest_version(texto: &str) -> Option<(u32, u32, u32)> {
    let linha = texto
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("version"))?;
    let valor = linha.split('=').nth(1)?.trim().trim_matches('"');
    let mut partes = valor.split('.');
    Some((
        partes.next()?.trim().parse().ok()?,
        partes.next().unwrap_or("0").trim().parse().unwrap_or(0),
        partes.next().unwrap_or("0").trim().parse().unwrap_or(0),
    ))
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

    /// Um `.bak` livre e reaproveitado, sem inventar nome novo.
    #[test]
    fn a_copia_antiga_vai_para_bak_quando_ele_esta_livre() {
        let dir = temp_dir("bak-livre");
        std::fs::create_dir_all(dir.join("paper.bak")).unwrap();
        std::fs::write(dir.join("paper.bak").join("velho"), "x").unwrap();

        assert_eq!(pasta_de_copia(&dir, "paper"), dir.join("paper.bak"));
        assert!(!dir.join("paper.bak").exists(), "o .bak antigo devia ter sido apagado");
    }

    /// O caso do Aquario em 13/09/2026: um arquivo aberto dentro do `.bak`
    /// impedia de apaga-lo, o `rename` falhava e a atualizacao nunca chegava.
    #[cfg(windows)]
    #[test]
    fn com_o_bak_preso_a_copia_vai_para_outra_pasta() {
        let dir = temp_dir("bak-preso");
        let bak = dir.join("paper.bak");
        std::fs::create_dir_all(&bak).unwrap();
        // Um arquivo aberto sem permitir exclusao segura a pasta inteira. E o
        // que faz um programa comum (visualizador, indexador) com o arquivo
        // que esta lendo; o `File::create` do Rust permite exclusao e nao
        // reproduziria o caso.
        use std::os::windows::fs::OpenOptionsExt;
        let _preso = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .share_mode(0)
            .open(bak.join("aberto"))
            .unwrap();

        let escolhida = pasta_de_copia(&dir, "paper");
        assert_ne!(escolhida, bak, "nao pode insistir numa pasta que nao sai");
        assert!(
            escolhida.file_name().unwrap().to_string_lossy().starts_with("paper.bak-"),
            "{}",
            escolhida.display()
        );
    }

    /// O defeito que a atualizacao conserta: uma correcao nos temas de fabrica
    /// nunca chegava a quem ja tinha o tema instalado.
    #[test]
    fn tema_desatualizado_e_substituido_e_o_antigo_vira_bak() {
        let dir = temp_dir("atualiza");
        install_missing(&dir);

        let paper = dir.join("paper");
        let manifesto = paper.join("manifest.toml");
        let original = std::fs::read_to_string(&manifesto).unwrap();
        // Finge uma instalacao antiga.
        let version_line = original
            .lines()
            .find(|line| line.trim_start().starts_with("version"))
            .expect("tema embutido sempre tem versao");
        let outdated = original.replacen(version_line, "version = \"0.0.0\"", 1);
        assert_ne!(
            outdated, original,
            "a versao simulada precisa ser mais antiga"
        );
        std::fs::write(&manifesto, outdated).unwrap();
        std::fs::write(paper.join("theme.toml"), "editado a mao").unwrap();

        let atualizados = install_missing(&dir);
        assert!(
            atualizados.contains(&"paper"),
            "paper devia ter sido atualizado"
        );
        assert_eq!(
            std::fs::read_to_string(&manifesto).unwrap(),
            original,
            "o manifesto instalado devia ser o novo"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("paper.bak").join("theme.toml")).unwrap(),
            "editado a mao",
            "a copia antiga precisa sobreviver: perder edicao alheia seria pior que o defeito"
        );
    }

    /// Sem isto, todo arranque reescreveria os cinco temas em disco.
    #[test]
    fn tema_em_dia_nao_e_reescrito() {
        let dir = temp_dir("em-dia");
        install_missing(&dir);
        assert!(install_missing(&dir).is_empty());
    }

    /// Copia gravada antes de haver versionamento de tema tem de ser
    /// atualizada -- e **uma vez so**. A primeira redacao disto nao atualizava,
    /// e o efeito foi o Bruma nunca receber a propria correcao.
    #[test]
    fn copia_anterior_ao_versionamento_atualiza_uma_vez() {
        let dir = temp_dir("sem-versao");
        install_missing(&dir);

        let manifesto = dir.join("bruma").join("manifest.toml");
        let sem_versao: String = std::fs::read_to_string(&manifesto)
            .unwrap()
            .lines()
            .filter(|l| !l.trim_start().starts_with("version ="))
            .collect::<Vec<_>>()
            .join(
                "
",
            );
        std::fs::write(&manifesto, sem_versao).unwrap();

        assert!(install_missing(&dir).contains(&"bruma"), "devia atualizar");
        assert!(
            install_missing(&dir).is_empty(),
            "e nao pode reescrever de novo no arranque seguinte"
        );
    }

    #[test]
    fn versao_do_manifesto_compara_como_numero() {
        assert!(manifest_version("version = \"1.10.0\"") > manifest_version("version = \"1.9.0\""));
        assert_eq!(manifest_version("version = \"2.1\""), Some((2, 1, 0)));
        assert_eq!(manifest_version("nada aqui"), None);
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

    /// A translucidez e das superficies. Alfa global tambem apaga glifos e
    /// capas via WS_EX_LAYERED, mesmo quando seus tokens sao opacos.
    #[test]
    fn temas_de_vidro_preservam_opacidade_do_conteudo() {
        let dir = temp_dir("vidro-opacidade");
        install_missing(&dir);
        for id in ["bruma", "aquario"] {
            let spec = morune_theme::load(&dir, id).spec;
            assert_eq!(spec.effects.window_opacity, 1.0, "{id}");
            for tinta in [
                spec.colors.text,
                spec.colors.text_muted,
                spec.colors.text_on_accent,
            ] {
                assert_eq!(tinta.a, 255, "{id}: tinta translucida");
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn assert_texto_legivel(
        spec: &morune_theme::ThemeSpec,
        fundo: morune_theme::Color,
        contexto: &str,
    ) {
        for (nome, tinta) in [
            ("text", spec.colors.text),
            ("text_muted", spec.colors.text_muted),
        ] {
            let contraste = tinta.over(fundo).contrast_ratio(fundo);
            assert!(
                contraste >= 4.5,
                "{}: {nome} em {contexto}: {contraste:.2}:1, fundo {fundo:?}",
                spec.manifest.id
            );
        }
    }

    /// Complementa o validador geral de 3:1 sem restringir temas personalizados.
    /// Cobre as camadas de cor, nao reflexos bitmap nem o compositor do Windows.
    #[test]
    fn texto_dos_temas_de_vidro_resiste_a_superficies_e_estados() {
        use morune_theme::Color;
        let dir = temp_dir("vidro-camadas");
        install_missing(&dir);
        for id in ["bruma", "aquario"] {
            let spec = morune_theme::load(&dir, id).spec;
            let c = &spec.colors;
            for desktop in [Color::rgb(0, 0, 0), Color::rgb(255, 255, 255)] {
                let base = c.background.over(desktop);
                for (nome, superficie) in [
                    ("conteudo", Color::TRANSPARENT),
                    ("painel", c.surface),
                    ("menu", c.surface_raised),
                    ("sidebar", c.sidebar_background),
                    ("player", c.player_background),
                ] {
                    let fundo = superficie.over(base);
                    for estado in [Color::TRANSPARENT, c.hover, c.selected] {
                        assert_texto_legivel(&spec, estado.over(fundo), nome);
                        if nome == "player" && spec.effects.artwork_tint {
                            for capa in [Color::rgb(0, 0, 0), Color::rgb(255, 255, 255)] {
                                let tingido = capa
                                    .scale_alpha(spec.effects.artwork_tint_strength)
                                    .over(fundo);
                                assert_texto_legivel(&spec, estado.over(tingido), "player tingido");
                            }
                        }
                    }
                }
                for acento in [c.accent, c.accent_hover] {
                    let fundo = acento.over(base);
                    assert!(
                        c.text_on_accent.over(fundo).contrast_ratio(fundo) >= 4.5,
                        "{id}: acento"
                    );
                }
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Decodificacao de arquivo apenas: nao cria janela nem captura a tela.
    /// Verifica todos os pixels da cena, com opacidade, veu e realces da UI.
    #[test]
    fn aquario_preserva_texto_sobre_a_imagem_de_fundo() {
        use morune_theme::Color;
        let dir = temp_dir("aquario-pixels");
        install_missing(&dir);
        let spec = morune_theme::load(&dir, "aquario").spec;
        // Usa a mesma reducao da tela, inclusive para fotografias grandes.
        let image =
            crate::wallpaper::load(Some(&dir.join("aquario")), &spec.background, None).image;
        let pixels = image.to_rgba8().expect("PNG deve disponibilizar pixels");
        assert!(!pixels.as_slice().is_empty());
        let surface = crate::optics::readable_tint(&image, spec.colors.surface, &spec);
        let sidebar = crate::optics::readable_tint(&image, spec.colors.sidebar_background, &spec);
        let mut pior = Color::rgb(255, 255, 255);
        let mut luminancia_minima = f32::MAX;
        for pixel in pixels.as_slice() {
            let cena = Color::rgba(pixel.r, pixel.g, pixel.b, pixel.a)
                .scale_alpha(spec.background.opacity)
                .over(spec.colors.background);
            let fundo = spec
                .background
                .tint
                .scale_alpha(spec.background.tint_strength)
                .over(cena);
            for estado in [Color::TRANSPARENT, spec.colors.hover, spec.colors.selected] {
                // Inclui o suporte de leitura e a copia suave da cena nas barras.
                let conteudo = surface.over(fundo);
                let barra = Color::rgba(pixel.r, pixel.g, pixel.b, pixel.a)
                    .scale_alpha(0.7 * 0.12)
                    .over(sidebar.over(fundo));
                let base = if conteudo.relative_luminance() < barra.relative_luminance() {
                    conteudo
                } else {
                    barra
                };
                let composto = estado.over(base);
                let luminancia = composto.relative_luminance();
                if luminancia < luminancia_minima {
                    luminancia_minima = luminancia;
                    pior = composto;
                }
            }
        }
        // As duas tintas sao opacas e mais escuras que qualquer pixel da cena;
        // portanto o pixel de menor luminancia e o pior caso para ambas.
        assert!(spec.colors.text.relative_luminance() < luminancia_minima);
        assert!(spec.colors.text_muted.relative_luminance() < luminancia_minima);
        assert_texto_legivel(&spec, pior, "pior pixel da cena com realce");
        println!(
            "Aquario: menor contraste secundario na cena = {:.2}:1",
            spec.colors.text_muted.contrast_ratio(pior)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bruma_preserva_leitura_sobre_cena_branca_e_reflexo() {
        use morune_theme::Color;
        let dir = temp_dir("bruma-cena-clara");
        install_missing(&dir);
        let spec = morune_theme::load(&dir, "bruma").spec;
        assert!(
            spec.effects.acrylic,
            "Bruma deve pedir o fundo nativo do desktop"
        );
        assert!(
            spec.background.image.is_empty(),
            "wallpaper nao pode encobrir o desktop"
        );
        let branco = Color::rgb(255, 255, 255);
        let mut white_pixels = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(1, 1);
        white_pixels.make_mut_slice()[0] = slint::Rgba8Pixel::new(255, 255, 255, 255);
        let white_image = slint::Image::from_rgba8(white_pixels);
        let mut dark_pixels = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(1, 1);
        dark_pixels.make_mut_slice()[0] = slint::Rgba8Pixel::new(20, 27, 34, 255);
        let dark_image = slint::Image::from_rgba8(dark_pixels);
        let mut rim_pixels = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(32, 32);
        for (i, pixel) in rim_pixels.make_mut_slice().iter_mut().enumerate() {
            let (x, y) = (i % 32, i / 32);
            *pixel = if x < 4 || y < 4 || x >= 28 || y >= 28 {
                slint::Rgba8Pixel::new(255, 255, 255, 255)
            } else {
                slint::Rgba8Pixel::new(20, 27, 34, 255)
            };
        }
        let rim_image = slint::Image::from_rgba8(rim_pixels);
        for tint in [
            spec.colors.sidebar_background,
            spec.colors.player_background,
        ] {
            let adjusted = crate::optics::readable_glass_tint(&white_image, tint, &spec);
            let clear = crate::optics::readable_glass_tint(&dark_image, tint, &spec);
            assert_eq!(
                crate::optics::readable_glass_tint(&rim_image, tint, &spec).a,
                clear.a,
                "o brilho decorativo da orla nao pode escurecer o centro"
            );
            assert_eq!(
                clear.a, tint.a,
                "cena escura preserva a transparencia pedida"
            );
            assert!(
                adjusted.a > clear.a,
                "protecao so aumenta quando a cena exige"
            );
            let painel = adjusted.over(branco);
            // Limites superiores do tint da capa e do reflexo de Glass.
            let painel = branco
                .scale_alpha(spec.effects.artwork_tint_strength)
                .over(painel);
            let painel = branco.scale_alpha(0.1 * spec.effects.gloss).over(painel);
            for estado in [Color::TRANSPARENT, spec.colors.hover, spec.colors.selected] {
                assert_texto_legivel(
                    &spec,
                    estado.over(painel),
                    "cena branca + tint + reflexo + estado",
                );
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn existing_themes_are_never_overwritten() {
        let dir = temp_dir("preserve");
        install_missing(&dir);

        let edited = dir.join("paper").join("theme.toml");
        std::fs::write(&edited, "[color]\naccent = \"#123456\"\n").unwrap();

        let installed = install_missing(&dir);
        assert!(
            installed.is_empty(),
            "mesma versao nao pode reinstalar por cima: {installed:?}"
        );
        assert!(std::fs::read_to_string(&edited).unwrap().contains("123456"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
