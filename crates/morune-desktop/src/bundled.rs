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

/// Os quatro temas prontos da identidade de 31/10/2026: so manifesto e tokens,
/// sem imagem nem fonte -- a cor vem da capa e a fonte e a do Windows.
macro_rules! tema_so_tokens {
    ($id:literal) => {
        BundledTheme {
            id: $id,
            files: &[
                BundledFile {
                    name: "manifest.toml",
                    contents: BundledContents::Text(include_str!(concat!(
                        "../themes/",
                        $id,
                        "/manifest.toml"
                    ))),
                },
                BundledFile {
                    name: "theme.toml",
                    contents: BundledContents::Text(include_str!(concat!(
                        "../themes/",
                        $id,
                        "/theme.toml"
                    ))),
                },
            ],
        }
    };
}

const THEMES: &[BundledTheme] = &[
    tema_so_tokens!("lume"),
    tema_so_tokens!("vidro"),
    tema_so_tokens!("grade"),
    tema_so_tokens!("claro"),
];

/// Temas que ja vieram com o Morune e sairam. A copia instalada e apagada
/// (inclusive as `.bak`), e quem estava usando um deles volta para o Lume --
/// ver [`tema_aposentado`].
///
/// Decidido em 25/09/2026: eram escuros e claros genericos, sem conceito, e
/// deram lugar a Travessia e ao Dreamcore.
/// Os seis da tela em Slint sairam em 31/10/2026, com a identidade nova: eram
/// "o mesmo app com outra cor". Os arquivos deles sairam do projeto em
/// 02/11/2026, junto com o app em Slint; ficam no historico do git.
const APOSENTADOS: &[&str] = &[
    "paper",
    "cristal",
    "pulse",
    "bruma",
    "aquario",
    "travessia",
    "dreamcore",
    "lofi",
    "neo-shinjuku",
];

/// Apaga do disco os temas aposentados que vieram conosco.
pub fn remover_aposentados(themes_dir: &Path) {
    let Ok(entradas) = std::fs::read_dir(themes_dir) else {
        return;
    };
    for entrada in entradas.flatten() {
        let nome = entrada.file_name().to_string_lossy().to_string();
        // So o nome exato, a copia `.bak` e as `.bak-<n>` que `pasta_de_copia`
        // cria -- `paper.backup` ou `pulse.bak-minha` sao do usuario.
        let base = match nome.split_once(".bak") {
            None => nome.as_str(),
            Some((base, "")) => base,
            Some((base, resto))
                if resto
                    .strip_prefix('-')
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())) =>
            {
                base
            }
            Some(_) => continue,
        };
        // E so se fomos nos que instalamos: um tema importado que por acaso se
        // chame `paper` nao e o Paper antigo e nao pode sumir a cada abertura.
        if APOSENTADOS.contains(&base) && feito_pelo_morune(&entrada.path()) {
            match std::fs::remove_dir_all(entrada.path()) {
                Ok(()) => tracing::info!(tema = nome, "tema aposentado removido"),
                Err(e) => {
                    tracing::warn!(tema = nome, error = %e, "nao consegui remover tema aposentado")
                }
            }
        }
    }
}

/// `true` se o `manifest.toml` da pasta diz que o tema veio com o app: autor
/// `Otomori`, ou `Morune` nos temas instalados pela 1.0.
fn feito_pelo_morune(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join("manifest.toml"))
        .ok()
        .and_then(|texto| texto.parse::<toml::Table>().ok())
        .is_some_and(|m| {
            matches!(
                m.get("author").and_then(|a| a.as_str()),
                Some("Otomori" | "Morune")
            )
        })
}

/// `true` se o id e de um tema que saiu do aplicativo.
pub fn tema_aposentado(id: &str) -> bool {
    APOSENTADOS.contains(&id)
}

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
        std::fs::create_dir_all(dir.join("tema.bak")).unwrap();
        std::fs::write(dir.join("tema.bak").join("velho"), "x").unwrap();

        assert_eq!(pasta_de_copia(&dir, "tema"), dir.join("tema.bak"));
        assert!(
            !dir.join("tema.bak").exists(),
            "o .bak antigo devia ter sido apagado"
        );
    }

    /// O caso do Aquario em 13/09/2026: um arquivo aberto dentro do `.bak`
    /// impedia de apaga-lo, o `rename` falhava e a atualizacao nunca chegava.
    #[cfg(windows)]
    #[test]
    fn com_o_bak_preso_a_copia_vai_para_outra_pasta() {
        let dir = temp_dir("bak-preso");
        let bak = dir.join("tema.bak");
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

        let escolhida = pasta_de_copia(&dir, "tema");
        assert_ne!(escolhida, bak, "nao pode insistir numa pasta que nao sai");
        assert!(
            escolhida
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("tema.bak-"),
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

        let lume = dir.join("lume");
        let manifesto = lume.join("manifest.toml");
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
        std::fs::write(lume.join("theme.toml"), "editado a mao").unwrap();

        let atualizados = install_missing(&dir);
        assert!(
            atualizados.contains(&"lume"),
            "lume devia ter sido atualizado"
        );
        assert_eq!(
            std::fs::read_to_string(&manifesto).unwrap(),
            original,
            "o manifesto instalado devia ser o novo"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("lume.bak").join("theme.toml")).unwrap(),
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

        let manifesto = dir.join("lume").join("manifest.toml");
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

        assert!(install_missing(&dir).contains(&"lume"), "devia atualizar");
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
        let mut fundos = Vec::new();
        for theme in THEMES {
            let spec = morune_theme::load(&dir, theme.id).spec;
            assert_ne!(
                spec.colors.background, builtin.colors.background,
                "{} repete o fundo do embutido",
                theme.id
            );
            fundos.push((theme.id, spec.colors.background));
        }
        for (i, (a, fa)) in fundos.iter().enumerate() {
            for (b, fb) in &fundos[i + 1..] {
                assert_ne!(fa, fb, "{a} e {b} tem o mesmo fundo");
            }
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn temas_aposentados_saem_do_disco_com_as_copias() {
        let dir = temp_dir("aposentados");
        let nosso = "id = \"x\"
author = \"Otomori\"
";
        let alheio = "id = \"paper\"
author = \"Fulano\"
";
        for (pasta, manifest) in [
            ("paper", nosso),
            ("cristal.bak", nosso),
            ("pulse.bak-123", nosso),
            ("paper.backup", nosso),
            ("pulse.bak-minha", nosso),
            ("lume", nosso),
            ("meu-tema", nosso),
        ] {
            std::fs::create_dir_all(dir.join(pasta)).unwrap();
            std::fs::write(dir.join(pasta).join("manifest.toml"), manifest).unwrap();
        }
        remover_aposentados(&dir);
        for pasta in ["paper", "cristal.bak", "pulse.bak-123"] {
            assert!(!dir.join(pasta).exists(), "{pasta} devia ter saido");
        }
        for pasta in ["paper.backup", "pulse.bak-minha", "lume", "meu-tema"] {
            assert!(dir.join(pasta).exists(), "{pasta} nao podia ter saido");
        }
        // Tema importado de outra pessoa com o mesmo id fica.
        std::fs::create_dir_all(dir.join("paper")).unwrap();
        std::fs::write(dir.join("paper").join("manifest.toml"), alheio).unwrap();
        remover_aposentados(&dir);
        assert!(
            dir.join("paper").exists(),
            "tema alheio chamado paper sumiu"
        );
        assert!(tema_aposentado("paper") && !tema_aposentado("lume"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn derived_theme_inherits_layout_but_overrides_color() {
        let dir = temp_dir("derived");
        install_missing(&dir);

        let builtin = morune_theme::ThemeSpec::default();
        let lume = morune_theme::load(&dir, "lume").spec;

        // Travessia nao declara layout.toml: a composicao tem de vir do embutido.
        assert_eq!(lume.layout, builtin.layout);
        assert_ne!(lume.colors.accent, builtin.colors.accent);

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
        for id in ["lume", "vidro", "grade", "claro"] {
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
        for id in ["vidro"] {
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
        for id in ["lume", "vidro", "grade", "claro"] {
            let spec = morune_theme::load(&dir, id).spec;
            let c = &spec.colors;
            // Na tela web o fundo da capa fica ATRAS dos paineis (App.svelte,
            // `.fundo-capa`): a capa mais clara e a mais escura tingem o fundo,
            // e cada superficie vem por cima.
            let tintas: Vec<Color> = if spec.effects.artwork_tint {
                [
                    Color::TRANSPARENT,
                    Color::rgb(0, 0, 0),
                    Color::rgb(255, 255, 255),
                ]
                .into_iter()
                .map(|capa| capa.scale_alpha(spec.effects.artwork_tint_strength))
                .collect()
            } else {
                vec![Color::TRANSPARENT]
            };
            for desktop in [Color::rgb(0, 0, 0), Color::rgb(255, 255, 255)] {
                for tinta in &tintas {
                    let base = tinta.over(c.background.over(desktop));
                    for (nome, superficie) in [
                        ("painel", c.surface),
                        ("menu", c.surface_raised),
                        ("sidebar", c.sidebar_background),
                        ("player", c.player_background),
                    ] {
                        let fundo = superficie.over(base);
                        for estado in [Color::TRANSPARENT, c.hover, c.selected] {
                            assert_texto_legivel(&spec, estado.over(fundo), nome);
                        }
                    }
                }
                let base = c.background.over(desktop);
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
    #[test]
    fn existing_themes_are_never_overwritten() {
        let dir = temp_dir("preserve");
        install_missing(&dir);

        let edited = dir.join("lume").join("theme.toml");
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
