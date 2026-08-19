use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::layout::LayoutSpec;
use crate::manifest::{ManifestError, ThemeManifest};
use crate::tokens::{ColorTokens, EffectTokens, MotionTokens, ShapeTokens, TypographyTokens};

/// Um tema resolvido e pronto para ser aplicado na interface.
///
/// Este e o unico tipo que a camada de UI consome. Tudo que vem de disco passa
/// por [`ThemeSpec::sanitize`] antes de chegar aqui, entao a UI pode confiar em
/// todos os valores sem checar nada.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ThemeSpec {
    pub manifest: ThemeManifest,
    #[serde(rename = "color")]
    pub colors: ColorTokens,
    #[serde(rename = "typography")]
    pub typography: TypographyTokens,
    #[serde(rename = "shape")]
    pub shape: ShapeTokens,
    #[serde(rename = "motion")]
    pub motion: MotionTokens,
    #[serde(rename = "effects")]
    pub effects: EffectTokens,
    #[serde(rename = "layout")]
    pub layout: LayoutSpec,
}

impl Default for ThemeSpec {
    fn default() -> Self {
        Self {
            manifest: ThemeManifest::new("midnight", "Midnight"),
            colors: ColorTokens::default(),
            typography: TypographyTokens::default(),
            shape: ShapeTokens::default(),
            motion: MotionTokens::default(),
            effects: EffectTokens::default(),
            layout: LayoutSpec::default(),
        }
    }
}

/// Aviso emitido durante o carregamento de um tema.
///
/// Avisos nunca impedem o tema de ser aplicado; sao mostrados no Developer Mode
/// e no log para que o autor do tema consiga corrigir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeWarning {
    pub field: String,
    pub message: String,
}

impl ThemeWarning {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self { field: field.into(), message: message.into() }
    }
}

/// Razao de contraste minima exigida entre texto principal e fundo.
///
/// Abaixo disso o tema continua sendo aplicado, mas o aviso aparece: um tema
/// pode ser de baixo contraste de proposito, e nao cabe ao aplicativo proibir.
const MIN_TEXT_CONTRAST: f32 = 3.0;

impl ThemeSpec {
    /// Ajusta valores impossiveis e devolve os avisos correspondentes.
    ///
    /// Chamado sempre, inclusive no tema embutido, para que o caminho de
    /// validacao seja exercitado em todo boot em vez de so quando da errado.
    pub fn sanitize(&mut self) -> Vec<ThemeWarning> {
        let mut warnings: Vec<ThemeWarning> = self
            .layout
            .sanitize()
            .into_iter()
            .map(|m| ThemeWarning::new("layout", m))
            .collect();

        // Tipografia.
        if !(0.5..=3.0).contains(&self.typography.scale) || !self.typography.scale.is_finite() {
            warnings.push(ThemeWarning::new(
                "typography.scale",
                format!("{} fora da faixa [0.5, 3.0], ajustado", self.typography.scale),
            ));
            self.typography.scale =
                if self.typography.scale.is_finite() { self.typography.scale.clamp(0.5, 3.0) } else { 1.0 };
        }
        for (name, size) in [
            ("size_xs", &mut self.typography.size_xs),
            ("size_sm", &mut self.typography.size_sm),
            ("size_md", &mut self.typography.size_md),
            ("size_lg", &mut self.typography.size_lg),
            ("size_xl", &mut self.typography.size_xl),
            ("size_display", &mut self.typography.size_display),
        ] {
            if !(6.0..=96.0).contains(size) || !size.is_finite() {
                warnings.push(ThemeWarning::new(
                    format!("typography.{name}"),
                    format!("{size} fora da faixa [6, 96], ajustado"),
                ));
                *size = if size.is_finite() { size.clamp(6.0, 96.0) } else { 14.0 };
            }
        }
        if !(1.0..=2.5).contains(&self.typography.line_height) {
            self.typography.line_height = self.typography.line_height.clamp(1.0, 2.5);
            warnings.push(ThemeWarning::new("typography.line_height", "ajustado para [1.0, 2.5]"));
        }

        // Efeitos: uma janela quase transparente deixa o aplicativo
        // irrecuperavel pelo proprio usuario, entao o piso e alto.
        if !(0.2..=1.0).contains(&self.effects.window_opacity)
            || !self.effects.window_opacity.is_finite()
        {
            warnings.push(ThemeWarning::new(
                "effects.window_opacity",
                format!("{} fora da faixa [0.2, 1.0], ajustado", self.effects.window_opacity),
            ));
            self.effects.window_opacity = if self.effects.window_opacity.is_finite() {
                self.effects.window_opacity.clamp(0.2, 1.0)
            } else {
                1.0
            };
        }
        self.effects.shadow_strength = self.effects.shadow_strength.clamp(0.0, 1.0);
        self.effects.artwork_tint_strength = self.effects.artwork_tint_strength.clamp(0.0, 1.0);
        if !(0.0..=64.0).contains(&self.effects.backdrop_blur) {
            self.effects.backdrop_blur = self.effects.backdrop_blur.clamp(0.0, 64.0);
            warnings.push(ThemeWarning::new("effects.backdrop_blur", "ajustado para [0, 64]"));
        }

        // Movimento.
        for (name, d) in [
            ("duration_fast", &mut self.motion.duration_fast),
            ("duration_normal", &mut self.motion.duration_normal),
            ("duration_slow", &mut self.motion.duration_slow),
        ] {
            if !(0..=5_000).contains(d) {
                warnings.push(ThemeWarning::new(
                    format!("motion.{name}"),
                    format!("{d} fora da faixa [0, 5000], ajustado"),
                ));
                *d = (*d).clamp(0, 5_000);
            }
        }

        // Formas.
        for (name, v, max) in [
            ("radius_sm", &mut self.shape.radius_sm, 64.0),
            ("radius_md", &mut self.shape.radius_md, 64.0),
            ("radius_lg", &mut self.shape.radius_lg, 64.0),
            ("radius_artwork", &mut self.shape.radius_artwork, 999.0),
            ("radius_avatar", &mut self.shape.radius_avatar, 999.0),
            ("border_width", &mut self.shape.border_width, 8.0),
            ("progress_thickness", &mut self.shape.progress_thickness, 32.0),
            ("scrollbar_width", &mut self.shape.scrollbar_width, 40.0),
        ] {
            if !(0.0..=max).contains(v) || !v.is_finite() {
                warnings.push(ThemeWarning::new(
                    format!("shape.{name}"),
                    format!("{v} fora da faixa [0, {max}], ajustado"),
                ));
                *v = if v.is_finite() { v.clamp(0.0, max) } else { 0.0 };
            }
        }

        warnings.extend(self.contrast_warnings());
        warnings
    }

    /// Avisos de legibilidade. Nao altera nada: so relata.
    pub fn contrast_warnings(&self) -> Vec<ThemeWarning> {
        let c = &self.colors;
        let mut out = Vec::new();
        let mut check = |field: &str, fg: Color, bg: Color| {
            let ratio = fg.contrast_ratio(bg);
            if ratio < MIN_TEXT_CONTRAST {
                out.push(ThemeWarning::new(
                    field,
                    format!("contraste {ratio:.2}:1 abaixo do minimo {MIN_TEXT_CONTRAST}:1"),
                ));
            }
        };
        check("color.text", c.text, c.background);
        check("color.text_muted", c.text_muted, c.background);
        check("color.text_on_accent", c.text_on_accent, c.accent);
        check("color.sidebar_text", c.text, c.sidebar_background);
        check("color.player_text", c.text, c.player_background);
        out
    }

    /// Aplica sobre este tema apenas os campos definidos num tema derivado.
    ///
    /// Usado por `based_on`: o filho e desserializado sobre uma copia do pai,
    /// entao herdar e simplesmente nao declarar o campo.
    pub fn validate_manifest(&self) -> Result<(), ManifestError> {
        self.manifest.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_theme_is_clean() {
        let mut t = ThemeSpec::default();
        let warnings = t.sanitize();
        assert!(warnings.is_empty(), "tema embutido deveria estar limpo: {warnings:?}");
        assert!(t.validate_manifest().is_ok());
    }

    #[test]
    fn nearly_invisible_window_is_pulled_back() {
        let mut t = ThemeSpec::default();
        t.effects.window_opacity = 0.01;
        let w = t.sanitize();
        assert_eq!(t.effects.window_opacity, 0.2);
        assert!(w.iter().any(|w| w.field == "effects.window_opacity"));
    }

    #[test]
    fn unreadable_text_produces_a_warning_but_still_applies() {
        let mut t = ThemeSpec::default();
        t.colors.text = t.colors.background;
        let w = t.sanitize();
        assert!(w.iter().any(|w| w.field == "color.text"), "{w:?}");
        // O tema continua aplicavel: nada foi alterado nas cores.
        assert_eq!(t.colors.text, t.colors.background);
    }

    #[test]
    fn absurd_font_sizes_are_clamped() {
        let mut t = ThemeSpec::default();
        t.typography.size_md = 5_000.0;
        t.typography.size_xs = -3.0;
        t.sanitize();
        assert_eq!(t.typography.size_md, 96.0);
        assert_eq!(t.typography.size_xs, 6.0);
    }

    #[test]
    fn nan_anywhere_is_replaced_by_something_finite() {
        let mut t = ThemeSpec::default();
        t.typography.scale = f32::NAN;
        t.effects.window_opacity = f32::NAN;
        t.shape.radius_md = f32::NAN;
        t.sanitize();
        assert!(t.typography.scale.is_finite());
        assert!(t.effects.window_opacity.is_finite());
        assert!(t.shape.radius_md.is_finite());
    }

    #[test]
    fn sanitize_is_idempotent() {
        let mut t = ThemeSpec::default();
        t.sidebar_width_hack();
        let first = t.sanitize();
        assert!(!first.is_empty());
        let second = t.sanitize();
        assert!(second.is_empty(), "segunda passada ainda reclamou: {second:?}");
    }

    #[test]
    fn spec_round_trips_through_toml() {
        let t = ThemeSpec::default();
        let text = toml::to_string(&t).unwrap();
        let back: ThemeSpec = toml::from_str(&text).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn an_almost_empty_theme_file_still_loads() {
        let text = r#"
[manifest]
schema_version = 1
id = "quase-vazio"
name = "Quase Vazio"
"#;
        let mut t: ThemeSpec = toml::from_str(text).unwrap();
        assert!(t.validate_manifest().is_ok());
        assert!(t.sanitize().is_empty());
        assert_eq!(t.colors, ColorTokens::default());
    }

    // Auxiliar de teste: introduz um valor invalido conhecido.
    impl ThemeSpec {
        fn sidebar_width_hack(&mut self) {
            self.layout.sidebar.width = 10_000.0;
        }
    }
}
