//! Descoberta e carregamento de temas a partir do disco.
//!
//! Regra que orienta todo este modulo: **um tema quebrado nunca impede o
//! aplicativo de abrir**. Toda falha degrada para o tema embutido e vira um
//! diagnostico visivel, nunca um erro fatal.

use std::fs;
use std::path::{Path, PathBuf};

use crate::manifest::{is_safe_id, ThemeManifest};
use crate::spec::{ThemeSpec, ThemeWarning};

/// As cores que a lista de temas precisa para desenhar a previa.
///
/// Um subconjunto de [`crate::ColorTokens`], e nao o tema inteiro: a lista
/// mostra cinco cores, e carregar tudo so para pintar um retangulo de 64 px
/// seria ler dezenas de arquivos a cada vez que a tela abre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemePreview {
    pub background: crate::Color,
    pub surface: crate::Color,
    pub sidebar: crate::Color,
    pub player: crate::Color,
    pub accent: crate::Color,
    pub text: crate::Color,
}

impl From<&ThemeSpec> for ThemePreview {
    fn from(spec: &ThemeSpec) -> Self {
        Self {
            background: spec.colors.background,
            surface: spec.colors.surface,
            sidebar: spec.colors.sidebar_background,
            player: spec.colors.player_background,
            accent: spec.colors.accent,
            text: spec.colors.text,
        }
    }
}

/// Um tema encontrado no disco, ainda nao carregado por completo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeEntry {
    pub manifest: ThemeManifest,
    pub path: PathBuf,
    /// `true` para o tema embutido, que nao pode ser apagado nem editado.
    pub builtin: bool,
    /// Cores para a previa na lista.
    pub preview: ThemePreview,
}

/// Resultado de carregar um tema.
///
/// Sempre contem um [`ThemeSpec`] utilizavel. `diagnostics` diz o que deu
/// errado no caminho, e `fell_back` marca que o tema pedido nao pode ser usado.
#[derive(Debug, Clone)]
pub struct LoadedTheme {
    pub spec: ThemeSpec,
    pub source: Option<PathBuf>,
    pub warnings: Vec<ThemeWarning>,
    pub errors: Vec<String>,
    pub fell_back: bool,
}

impl LoadedTheme {
    /// O tema embutido, garantido valido.
    pub fn builtin() -> Self {
        let mut spec = ThemeSpec::default();
        let warnings = spec.sanitize();
        debug_assert!(warnings.is_empty(), "tema embutido invalido: {warnings:?}");
        Self {
            spec,
            source: None,
            warnings,
            errors: Vec::new(),
            fell_back: false,
        }
    }

    fn fallback_with(errors: Vec<String>) -> Self {
        Self {
            fell_back: true,
            errors,
            ..Self::builtin()
        }
    }

    pub fn is_healthy(&self) -> bool {
        !self.fell_back && self.errors.is_empty()
    }
}

/// Lista os temas instalados em `themes_dir`, em ordem alfabetica por nome.
///
/// Pastas invalidas sao ignoradas com um aviso no log em vez de abortar a
/// listagem: um unico tema corrompido nao pode esconder todos os outros.
pub fn discover(themes_dir: &Path) -> Vec<ThemeEntry> {
    let embutido = ThemeSpec::default();
    let mut found = vec![ThemeEntry {
        manifest: embutido.manifest.clone(),
        path: PathBuf::new(),
        builtin: true,
        preview: ThemePreview::from(&embutido),
    }];

    let Ok(entries) = fs::read_dir(themes_dir) else {
        return found;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        // Pastas temporarias de importacao comecam com ponto.
        if dir_name.starts_with('.') || !is_safe_id(dir_name) {
            continue;
        }

        match read_manifest(&path) {
            Ok(manifest) => {
                if manifest.id != dir_name {
                    tracing::warn!(
                        theme = dir_name,
                        manifest_id = manifest.id,
                        "id do manifesto difere do nome da pasta, tema ignorado"
                    );
                    continue;
                }
                // A previa sai do tema resolvido, com heranca aplicada: um
                // tema que so declara o `accent` precisa mostrar o fundo que
                // ele herdou, e nao o preto do padrao.
                let preview = ThemePreview::from(
                    &load_from_dir(&path, themes_dir, 0)
                        .map_or_else(|_| ThemeSpec::default(), |loaded| loaded.spec),
                );
                found.push(ThemeEntry {
                    manifest,
                    path,
                    builtin: false,
                    preview,
                });
            }
            Err(e) => tracing::warn!(theme = dir_name, error = %e, "tema ignorado"),
        }
    }

    found.sort_by_key(|entry| entry.manifest.name.to_lowercase());
    found
}

fn read_manifest(dir: &Path) -> Result<ThemeManifest, String> {
    let raw = fs::read_to_string(dir.join("manifest.toml"))
        .map_err(|e| format!("manifest.toml ilegivel: {e}"))?;
    let manifest: ThemeManifest =
        toml::from_str(&raw).map_err(|e| format!("manifest.toml invalido: {e}"))?;
    manifest.validate().map_err(|e| e.to_string())?;
    Ok(manifest)
}

/// Carrega o tema `id` a partir de `themes_dir`.
///
/// Nunca falha: qualquer problema resulta no tema embutido com os erros
/// registrados em [`LoadedTheme::errors`].
pub fn load(themes_dir: &Path, id: &str) -> LoadedTheme {
    if id.is_empty() || id == ThemeSpec::default().manifest.id {
        return LoadedTheme::builtin();
    }
    if !is_safe_id(id) {
        return LoadedTheme::fallback_with(vec![format!("id de tema invalido: {id:?}")]);
    }

    let dir = themes_dir.join(id);
    match load_from_dir(&dir, themes_dir, 0) {
        Ok(loaded) => loaded,
        Err(e) => {
            tracing::error!(theme = id, error = %e, "falha ao carregar tema, usando o embutido");
            LoadedTheme::fallback_with(vec![e])
        }
    }
}

/// Profundidade maxima de `based_on`, para que uma cadeia circular termine.
const MAX_INHERITANCE_DEPTH: usize = 4;

fn load_from_dir(dir: &Path, themes_dir: &Path, depth: usize) -> Result<LoadedTheme, String> {
    if depth > MAX_INHERITANCE_DEPTH {
        return Err(format!(
            "cadeia de heranca de temas maior que {MAX_INHERITANCE_DEPTH}"
        ));
    }

    let manifest = read_manifest(dir)?;

    // A base e o tema pai quando existe `based_on`, senao o tema embutido.
    // Herdar e simplesmente nao declarar um campo: os que faltam ficam com o
    // valor do pai, sem sintaxe especial.
    let mut base = match manifest.based_on.as_deref() {
        Some(parent) if is_safe_id(parent) => {
            match load_from_dir(&themes_dir.join(parent), themes_dir, depth + 1) {
                Ok(loaded) => loaded.spec,
                Err(e) => {
                    tracing::warn!(parent, error = %e, "tema pai indisponivel, herdando do embutido");
                    ThemeSpec::default()
                }
            }
        }
        Some(parent) => return Err(format!("based_on invalido: {parent:?}")),
        None => ThemeSpec::default(),
    };

    let mut errors = Vec::new();

    // `theme.toml` e `layout.toml` sao opcionais e independentes: um tema pode
    // mudar so as cores, ou so a composicao da tela.
    if let Some(raw) = read_optional(dir, "theme.toml", &mut errors) {
        match toml::from_str::<PartialTheme>(&raw) {
            Ok(partial) => partial.apply(&mut base),
            Err(e) => errors.push(format!("theme.toml invalido, ignorado: {e}")),
        }
    }
    if let Some(raw) = read_optional(dir, "layout.toml", &mut errors) {
        match toml::from_str::<crate::layout::LayoutSpec>(&raw) {
            Ok(layout) => base.layout = layout,
            Err(e) => errors.push(format!("layout.toml invalido, ignorado: {e}")),
        }
    }

    base.manifest = manifest;
    let warnings = base.sanitize();

    Ok(LoadedTheme {
        spec: base,
        source: Some(dir.to_path_buf()),
        warnings,
        errors,
        fell_back: false,
    })
}

fn read_optional(dir: &Path, name: &str, errors: &mut Vec<String>) -> Option<String> {
    let path = dir.join(name);
    if !path.is_file() {
        return None;
    }
    match fs::read_to_string(&path) {
        Ok(raw) => Some(raw),
        Err(e) => {
            errors.push(format!("{name} ilegivel, ignorado: {e}"));
            None
        }
    }
}

/// Secoes opcionais de `theme.toml`.
///
/// Cada secao ausente preserva o valor herdado; presente, substitui a secao
/// inteira. Substituir por secao, e nao por campo, mantem o formato previsivel
/// para quem escreve o tema a mao.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PartialTheme {
    color: Option<crate::tokens::ColorTokens>,
    typography: Option<crate::tokens::TypographyTokens>,
    shape: Option<crate::tokens::ShapeTokens>,
    control: Option<crate::tokens::ControlTokens>,
    motion: Option<crate::tokens::MotionTokens>,
    effects: Option<crate::tokens::EffectTokens>,
    background: Option<crate::tokens::BackgroundTokens>,
}

impl PartialTheme {
    fn apply(self, base: &mut ThemeSpec) {
        if let Some(v) = self.color {
            base.colors = v;
        }
        if let Some(v) = self.typography {
            base.typography = v;
        }
        if let Some(v) = self.shape {
            base.shape = v;
        }
        if let Some(v) = self.control {
            base.control = v;
        }
        if let Some(v) = self.motion {
            base.motion = v;
        }
        if let Some(v) = self.effects {
            base.effects = v;
        }
        if let Some(v) = self.background {
            base.background = v;
        }
    }
}

/// Grava um tema em disco como pasta editavel (`manifest.toml` + `theme.toml` +
/// `layout.toml`). Usado por "duplicar tema" e pelo Developer Mode.
pub fn write_theme(dir: &Path, spec: &ThemeSpec) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;

    let manifest = toml::to_string_pretty(&spec.manifest).map_err(|e| e.to_string())?;
    fs::write(dir.join("manifest.toml"), manifest).map_err(|e| e.to_string())?;

    #[derive(serde::Serialize)]
    struct ThemeFile<'a> {
        color: &'a crate::tokens::ColorTokens,
        typography: &'a crate::tokens::TypographyTokens,
        shape: &'a crate::tokens::ShapeTokens,
        control: &'a crate::tokens::ControlTokens,
        motion: &'a crate::tokens::MotionTokens,
        effects: &'a crate::tokens::EffectTokens,
        background: &'a crate::tokens::BackgroundTokens,
    }

    let theme = toml::to_string_pretty(&ThemeFile {
        color: &spec.colors,
        typography: &spec.typography,
        shape: &spec.shape,
        control: &spec.control,
        motion: &spec.motion,
        effects: &spec.effects,
        background: &spec.background,
    })
    .map_err(|e| e.to_string())?;
    fs::write(dir.join("theme.toml"), theme).map_err(|e| e.to_string())?;

    let layout = toml::to_string_pretty(&spec.layout).map_err(|e| e.to_string())?;
    fs::write(dir.join("layout.toml"), layout).map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let p = std::env::temp_dir()
                .join(format!("morune-theme-test-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&p);
            fs::create_dir_all(&p).unwrap();
            Self(p)
        }
        fn path(&self) -> &Path {
            &self.0
        }
        fn theme(&self, id: &str, manifest: &str, theme: Option<&str>) -> PathBuf {
            let dir = self.0.join(id);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("manifest.toml"), manifest).unwrap();
            if let Some(t) = theme {
                fs::write(dir.join("theme.toml"), t).unwrap();
            }
            dir
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn manifest_toml(id: &str) -> String {
        format!("schema_version = 1\nid = \"{id}\"\nname = \"{id}\"\n")
    }

    #[test]
    fn builtin_theme_is_always_available_and_healthy() {
        let t = LoadedTheme::builtin();
        assert!(t.is_healthy());
        assert!(t.errors.is_empty());
    }

    #[test]
    fn missing_theme_directory_falls_back_without_panicking() {
        let dir = TempDir::new("missing");
        let loaded = load(dir.path(), "nao-existe");
        assert!(loaded.fell_back);
        assert!(!loaded.errors.is_empty());
        assert_eq!(loaded.spec.colors, crate::tokens::ColorTokens::default());
    }

    #[test]
    fn unsafe_theme_id_never_touches_the_disk() {
        let dir = TempDir::new("unsafe");
        let loaded = load(dir.path(), "../../windows");
        assert!(loaded.fell_back);
        assert!(loaded.errors[0].contains("invalido"));
    }

    #[test]
    fn theme_with_only_colors_keeps_default_layout() {
        let dir = TempDir::new("colors-only");
        dir.theme(
            "azul",
            &manifest_toml("azul"),
            Some("[color]\naccent = \"#3355ff\"\n"),
        );
        let loaded = load(dir.path(), "azul");
        assert!(loaded.is_healthy(), "{:?}", loaded.errors);
        assert_eq!(loaded.spec.colors.accent, Color::rgb(0x33, 0x55, 0xff));
        assert_eq!(loaded.spec.layout, crate::layout::LayoutSpec::default());
    }

    #[test]
    fn broken_theme_toml_degrades_to_defaults_instead_of_failing() {
        let dir = TempDir::new("broken");
        dir.theme(
            "quebrado",
            &manifest_toml("quebrado"),
            Some("[color\naccent = nao-e-toml"),
        );
        let loaded = load(dir.path(), "quebrado");
        // O manifesto era valido, entao o tema carrega -- so os tokens caem
        // para o padrao, e o erro fica registrado.
        assert!(!loaded.fell_back);
        assert!(!loaded.errors.is_empty());
        assert_eq!(loaded.spec.colors, crate::tokens::ColorTokens::default());
    }

    #[test]
    fn broken_manifest_falls_back_entirely() {
        let dir = TempDir::new("bad-manifest");
        dir.theme("ruim", "isto nao e toml valido {{{", None);
        let loaded = load(dir.path(), "ruim");
        assert!(loaded.fell_back);
    }

    #[test]
    fn every_section_of_theme_toml_reaches_the_spec() {
        // Regressao: `[background]` e `[control]` existiam no `ThemeSpec` e no
        // `theme.toml`, mas o `PartialTheme` daqui tinha uma lista fixa de
        // secoes e descartava as duas **em silencio**. O tema carregava, nao
        // avisava nada, e a imagem de fundo simplesmente nao aparecia.
        let dir = TempDir::new("todas-secoes");
        dir.theme(
            "completo",
            "schema_version = 1
id = \"completo\"
name = \"Completo\"
",
            Some(concat!(
                "[color]
accent = \"#ff0066\"

",
                "[typography]
scale = 1.25

",
                "[shape]
radius_md = 3.0

",
                "[control]
button_size = 44.0

",
                "[motion]
speed = 2.0

",
                "[effects]
acrylic = true

",
                "[background]
image = \"assets/backgrounds/f.jpg\"
blur = 12.0
",
            )),
        );

        let loaded = load(dir.path(), "completo");
        assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
        assert_eq!(loaded.spec.colors.accent, Color::rgb(0xff, 0x00, 0x66));
        assert_eq!(loaded.spec.typography.scale, 1.25);
        assert_eq!(loaded.spec.shape.radius_md, 3.0);
        assert_eq!(loaded.spec.control.button_size, 44.0);
        assert_eq!(loaded.spec.motion.speed, 2.0);
        assert!(loaded.spec.effects.acrylic);
        assert_eq!(loaded.spec.background.image, "assets/backgrounds/f.jpg");
        assert_eq!(loaded.spec.background.blur, 12.0);
    }

    #[test]
    fn writing_a_theme_then_loading_it_keeps_every_section() {
        // A outra metade da mesma regressao: `write_theme` alimenta "duplicar
        // tema". Uma secao esquecida ali faz a copia perder o fundo e as
        // medidas dos controles sem nenhum erro visivel.
        let dir = TempDir::new("ida-e-volta");
        let mut spec = ThemeSpec {
            manifest: crate::manifest::ThemeManifest::new("copia", "Copia"),
            ..Default::default()
        };
        spec.control.primary_button_size = 52.0;
        spec.background.image = "assets/backgrounds/f.jpg".into();
        spec.background.tint_strength = 0.7;
        spec.effects.acrylic = true;

        write_theme(&dir.path().join("copia"), &spec).unwrap();
        let loaded = load(dir.path(), "copia");

        assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
        assert_eq!(loaded.spec.control.primary_button_size, 52.0);
        assert_eq!(loaded.spec.background.image, "assets/backgrounds/f.jpg");
        assert_eq!(loaded.spec.background.tint_strength, 0.7);
        assert!(loaded.spec.effects.acrylic);
    }

    #[test]
    fn inheritance_copies_the_parent_then_overrides() {
        let dir = TempDir::new("inherit");
        dir.theme(
            "pai",
            &manifest_toml("pai"),
            Some("[color]\naccent = \"#ff0000\"\nbackground = \"#101010\"\n"),
        );
        dir.theme(
            "filho",
            &format!("{}based_on = \"pai\"\n", manifest_toml("filho")),
            Some("[shape]\nradius_md = 20.0\n"),
        );

        let loaded = load(dir.path(), "filho");
        assert!(loaded.is_healthy(), "{:?}", loaded.errors);
        assert_eq!(loaded.spec.colors.accent, Color::rgb(255, 0, 0));
        assert_eq!(loaded.spec.shape.radius_md, 20.0);
        assert_eq!(loaded.spec.manifest.id, "filho");
    }

    #[test]
    fn inheritance_cycle_terminates() {
        let dir = TempDir::new("cycle");
        dir.theme(
            "a",
            &format!("{}based_on = \"b\"\n", manifest_toml("a")),
            None,
        );
        dir.theme(
            "b",
            &format!("{}based_on = \"a\"\n", manifest_toml("b")),
            None,
        );
        // Nao deve travar nem estourar a pilha: a profundidade e limitada e o
        // excedente cai para o embutido.
        let loaded = load(dir.path(), "a");
        assert!(loaded.spec.colors == crate::tokens::ColorTokens::default());
    }

    #[test]
    fn discover_lists_builtin_plus_valid_themes_only() {
        let dir = TempDir::new("discover");
        dir.theme("bom", &manifest_toml("bom"), None);
        dir.theme("ruim", "nao e toml {{{", None);
        // Pasta cujo id nao bate com o manifesto.
        dir.theme("mentiroso", &manifest_toml("outro"), None);
        fs::create_dir_all(dir.path().join(".importing")).unwrap();

        let found = discover(dir.path());
        let ids: Vec<&str> = found.iter().map(|t| t.manifest.id.as_str()).collect();
        assert!(ids.contains(&"bom"));
        assert!(
            ids.contains(&"midnight"),
            "embutido deveria estar sempre presente"
        );
        assert!(!ids.contains(&"ruim"));
        assert!(!ids.contains(&"outro"));
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn discover_on_a_missing_directory_still_returns_the_builtin() {
        let found = discover(Path::new("D:/caminho/que/nao/existe/morune"));
        assert_eq!(found.len(), 1);
        assert!(found[0].builtin);
    }

    #[test]
    fn write_then_load_round_trips() {
        let dir = TempDir::new("write");
        let mut spec = ThemeSpec {
            manifest: ThemeManifest::new("copia", "Copia"),
            ..Default::default()
        };
        spec.colors.accent = Color::rgb(1, 2, 3);
        spec.layout.sidebar.width = 300.0;

        write_theme(&dir.path().join("copia"), &spec).unwrap();
        let loaded = load(dir.path(), "copia");

        assert!(loaded.is_healthy(), "{:?}", loaded.errors);
        assert_eq!(loaded.spec.colors.accent, Color::rgb(1, 2, 3));
        assert_eq!(loaded.spec.layout.sidebar.width, 300.0);
        assert_eq!(loaded.spec.manifest.name, "Copia");
    }

    #[test]
    fn theme_with_absurd_values_loads_sanitized_with_warnings() {
        let dir = TempDir::new("absurd");
        dir.theme(
            "extremo",
            &manifest_toml("extremo"),
            Some("[effects]\nwindow_opacity = 0.0\n"),
        );
        let loaded = load(dir.path(), "extremo");
        assert!(!loaded.fell_back);
        assert_eq!(loaded.spec.effects.window_opacity, 0.2);
        assert!(!loaded.warnings.is_empty());
    }
}
