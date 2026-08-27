use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::layout::LayoutSpec;
use crate::manifest::{ManifestError, ThemeManifest};
use crate::tokens::{
    BackgroundTokens, ColorTokens, ControlTokens, EffectTokens, MotionTokens, ShapeTokens,
    TypographyTokens,
};

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
    #[serde(rename = "control")]
    pub control: ControlTokens,
    #[serde(rename = "motion")]
    pub motion: MotionTokens,
    #[serde(rename = "effects")]
    pub effects: EffectTokens,
    #[serde(rename = "background")]
    pub background: BackgroundTokens,
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
            control: ControlTokens::default(),
            motion: MotionTokens::default(),
            effects: EffectTokens::default(),
            background: BackgroundTokens::default(),
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
        Self {
            field: field.into(),
            message: message.into(),
        }
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
                format!(
                    "{} fora da faixa [0.5, 3.0], ajustado",
                    self.typography.scale
                ),
            ));
            self.typography.scale = if self.typography.scale.is_finite() {
                self.typography.scale.clamp(0.5, 3.0)
            } else {
                1.0
            };
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
                *size = if size.is_finite() {
                    size.clamp(6.0, 96.0)
                } else {
                    14.0
                };
            }
        }
        if !(1.0..=2.5).contains(&self.typography.line_height) {
            self.typography.line_height = self.typography.line_height.clamp(1.0, 2.5);
            warnings.push(ThemeWarning::new(
                "typography.line_height",
                "ajustado para [1.0, 2.5]",
            ));
        }

        // Efeitos: uma janela quase transparente deixa o aplicativo
        // irrecuperavel pelo proprio usuario, entao o piso e alto.
        if !(0.2..=1.0).contains(&self.effects.window_opacity)
            || !self.effects.window_opacity.is_finite()
        {
            warnings.push(ThemeWarning::new(
                "effects.window_opacity",
                format!(
                    "{} fora da faixa [0.2, 1.0], ajustado",
                    self.effects.window_opacity
                ),
            ));
            self.effects.window_opacity = if self.effects.window_opacity.is_finite() {
                self.effects.window_opacity.clamp(0.2, 1.0)
            } else {
                1.0
            };
        }
        self.effects.shadow_strength = self.effects.shadow_strength.clamp(0.0, 1.0);
        self.effects.gloss = if self.effects.gloss.is_finite() {
            self.effects.gloss.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.effects.artwork_tint_strength = self.effects.artwork_tint_strength.clamp(0.0, 1.0);
        if !(0.0..=64.0).contains(&self.effects.backdrop_blur) {
            self.effects.backdrop_blur = self.effects.backdrop_blur.clamp(0.0, 64.0);
            warnings.push(ThemeWarning::new(
                "effects.backdrop_blur",
                "ajustado para [0, 64]",
            ));
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
            (
                "progress_thickness",
                &mut self.shape.progress_thickness,
                32.0,
            ),
            ("scrollbar_width", &mut self.shape.scrollbar_width, 40.0),
        ] {
            if !(0.0..=max).contains(v) || !v.is_finite() {
                warnings.push(ThemeWarning::new(
                    format!("shape.{name}"),
                    format!("{v} fora da faixa [0, {max}], ajustado"),
                ));
                *v = if v.is_finite() {
                    v.clamp(0.0, max)
                } else {
                    0.0
                };
            }
        }

        // Fundo. Um caminho que escapa do diretorio do tema e apagado em vez de
        // recusado: o tema continua valido, so nao tem imagem -- mesma postura
        // do resto do carregamento, que nunca deixa de aplicar um tema.
        if self.background.image.contains("..")
            || self.background.image.starts_with('/')
            || self.background.image.starts_with('\\')
            || self.background.image.contains(':')
        {
            warnings.push(ThemeWarning::new(
                "background.image",
                format!(
                    "{:?} nao e um caminho relativo dentro do tema, ignorado",
                    self.background.image
                ),
            ));
            self.background.image.clear();
        }
        self.background.opacity = if self.background.opacity.is_finite() {
            self.background.opacity.clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.background.tint_strength = if self.background.tint_strength.is_finite() {
            self.background.tint_strength.clamp(0.0, 1.0)
        } else {
            0.0
        };
        if !(0.0..=64.0).contains(&self.background.blur) || !self.background.blur.is_finite() {
            warnings.push(ThemeWarning::new(
                "background.blur",
                format!("{} fora da faixa [0, 64], ajustado", self.background.blur),
            ));
            self.background.blur = if self.background.blur.is_finite() {
                self.background.blur.clamp(0.0, 64.0)
            } else {
                0.0
            };
        }

        // Controles. Os pisos existem para que um tema nao consiga produzir um
        // alvo pequeno demais para acertar com o mouse enquanto se joga.
        for (name, v, min, max) in [
            ("button_size", &mut self.control.button_size, 20.0, 96.0),
            ("button_radius", &mut self.control.button_radius, 0.0, 999.0),
            (
                "primary_button_size",
                &mut self.control.primary_button_size,
                24.0,
                128.0,
            ),
            ("icon_size", &mut self.control.icon_size, 8.0, 64.0),
            (
                "icon_size_primary",
                &mut self.control.icon_size_primary,
                8.0,
                64.0,
            ),
            ("icon_stroke", &mut self.control.icon_stroke, 0.5, 6.0),
            ("slider_knob", &mut self.control.slider_knob, 0.0, 48.0),
            (
                "tooltip_height",
                &mut self.control.tooltip_height,
                16.0,
                64.0,
            ),
        ] {
            if !(min..=max).contains(v) || !v.is_finite() {
                warnings.push(ThemeWarning::new(
                    format!("control.{name}"),
                    format!("{v} fora da faixa [{min}, {max}], ajustado"),
                ));
                *v = if v.is_finite() {
                    v.clamp(min, max)
                } else {
                    min
                };
            }
        }

        warnings.extend(self.contrast_warnings());
        warnings
    }

    /// Avisos de legibilidade. Nao altera nada: so relata.
    pub fn contrast_warnings(&self) -> Vec<ThemeWarning> {
        let c = &self.colors;
        let mut out = Vec::new();

        // Superficies com alfa nao tem contraste proprio: dependem do que esta
        // atras, e o que esta atras e o fundo da janela -- que por sua vez pode
        // ser translucido e deixar passar a area de trabalho, que nao da para
        // conhecer daqui.
        //
        // Entao a conta e feita nos **dois piores casos**, area de trabalho
        // branca e preta, e vale o pior dos dois. Para um tema opaco nada muda
        // (compor sobre qualquer coisa devolve a propria cor); para um tema de
        // vidro e a diferenca entre um aviso falso e a leitura real.
        let branco = Color::rgb(0xff, 0xff, 0xff);
        let preto = Color::rgb(0x00, 0x00, 0x00);

        // `None` = o texto cai direto sobre o fundo da janela; `Some(cor)` = ha
        // uma regiao pintada por cima dele. A distincao existe porque a janela
        // pinta `background` uma vez so: empilhar o fundo sobre ele mesmo daria
        // um alfa efetivo que o aplicativo nao desenha.
        let mut check = |field: &str, fg: Color, regiao: Option<Color>| {
            let pior = [branco, preto]
                .into_iter()
                .map(|desktop| {
                    let base = c.background.over(desktop);
                    let atras = match regiao {
                        Some(cor) => cor.over(base),
                        None => base,
                    };
                    fg.over(atras).contrast_ratio(atras)
                })
                .fold(f32::INFINITY, f32::min);

            if pior < MIN_TEXT_CONTRAST {
                out.push(ThemeWarning::new(
                    field,
                    format!("contraste {pior:.2}:1 abaixo do minimo {MIN_TEXT_CONTRAST}:1"),
                ));
            }
        };
        check("color.text", c.text, None);
        check("color.text_muted", c.text_muted, None);
        check("color.text_on_accent", c.text_on_accent, Some(c.accent));
        check("color.sidebar_text", c.text, Some(c.sidebar_background));
        check("color.player_text", c.text, Some(c.player_background));

        // Borda e barra de rolagem nao sao texto, mas sao o que separa uma
        // regiao da seguinte e o que diz onde a lista esta: a WCAG 1.4.11 pede
        // os mesmos 3:1 para limite grafico e componente de interface. Ficaram
        // de fora da checagem original, que so olhava texto -- e foi por isso
        // que os quatro temas embutidos passaram anos reprovando sem que nada
        // acusasse.
        check("color.border", c.border, None);
        check("color.scrollbar", c.scrollbar, None);
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
        assert!(
            warnings.is_empty(),
            "tema embutido deveria estar limpo: {warnings:?}"
        );
        assert!(t.validate_manifest().is_ok());
    }

    #[test]
    fn um_tema_de_vidro_legivel_nao_e_acusado_de_contraste_ruim() {
        // Regressao: superficies de branco translucido eram lidas como branco
        // puro, e um tema perfeitamente legivel acusava 1.00:1.
        let mut t = ThemeSpec::default();
        t.colors.background = Color::rgba(0x0b, 0x0b, 0x12, 0xb3);
        t.colors.sidebar_background = Color::rgba(0xff, 0xff, 0xff, 0x0d);
        t.colors.player_background = Color::rgba(0xff, 0xff, 0xff, 0x14);
        t.colors.text = Color::rgb(0xff, 0xff, 0xff);
        // Um tema de vidro precisa de secundario mais claro que um tema opaco:
        // sobre vidro fino a area de trabalho clareia o fundo, e o cinza medio
        // que serve num fundo escuro deixa de servir.
        t.colors.text_muted = Color::rgb(0xe0, 0xe0, 0xe6);
        // Pelo mesmo motivo, borda e barra de rolagem tambem sobem: sobre vidro
        // a area de trabalho clareia o fundo, e o alfa que da 3:1 num fundo
        // opaco nao da mais. Sao os valores do Cristal, que e um tema de vidro
        // real.
        t.colors.border = Color::rgba(0xff, 0xff, 0xff, 0x77);
        t.colors.scrollbar = Color::rgba(0xff, 0xff, 0xff, 0x77);
        let avisos = t.contrast_warnings();
        assert!(avisos.is_empty(), "{avisos:?}");
    }

    #[test]
    fn vidro_transparente_demais_continua_sendo_pego() {
        // A outra metade: o verificador nao pode virar carimbo. Com o fundo
        // quase transparente, uma area de trabalho branca apaga o texto branco
        // -- e e exatamente isso que ele tem de dizer.
        let mut t = ThemeSpec::default();
        t.colors.background = Color::rgba(0x0b, 0x0b, 0x12, 0x1a);
        t.colors.text = Color::rgb(0xff, 0xff, 0xff);
        let avisos = t.contrast_warnings();
        assert!(
            avisos.iter().any(|w| w.field == "color.text"),
            "deveria acusar: {avisos:?}"
        );
    }

    #[test]
    fn background_image_escaping_the_theme_is_dropped() {
        for escape in [
            "../../windows/win.ini",
            "/etc/passwd",
            r"C:\Windows\win.ini",
        ] {
            let mut t = ThemeSpec::default();
            t.background.image = escape.into();
            let w = t.sanitize();
            assert!(
                t.background.image.is_empty(),
                "{escape} deveria ter sido descartado"
            );
            assert!(w.iter().any(|w| w.field == "background.image"));
        }
    }

    #[test]
    fn a_relative_background_image_survives_sanitize() {
        let mut t = ThemeSpec::default();
        t.background.image = "assets/backgrounds/fundo.jpg".into();
        let w = t.sanitize();
        assert_eq!(t.background.image, "assets/backgrounds/fundo.jpg");
        assert!(w.is_empty(), "{w:?}");
    }

    #[test]
    fn absurd_background_blur_is_clamped() {
        let mut t = ThemeSpec::default();
        t.background.blur = 4_000.0;
        let w = t.sanitize();
        assert_eq!(t.background.blur, 64.0);
        assert!(w.iter().any(|w| w.field == "background.blur"));
    }

    #[test]
    fn a_control_too_small_to_click_is_pulled_back() {
        let mut t = ThemeSpec::default();
        t.control.button_size = 2.0;
        t.control.icon_stroke = 0.0;
        let w = t.sanitize();
        assert_eq!(t.control.button_size, 20.0);
        assert_eq!(t.control.icon_stroke, 0.5);
        assert!(w.iter().any(|w| w.field == "control.button_size"));
        assert!(w.iter().any(|w| w.field == "control.icon_stroke"));
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
        assert!(
            second.is_empty(),
            "segunda passada ainda reclamou: {second:?}"
        );
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
