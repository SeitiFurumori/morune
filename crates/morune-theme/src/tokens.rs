use serde::{Deserialize, Serialize};

use crate::color::Color;

/// Paleta do tema.
///
/// Os nomes sao semanticos, nao literais: um tema claro define `surface` como
/// quase branco e `text` como quase preto, e nada acima precisa saber disso.
/// Todo campo tem valor padrao para que um tema incompleto continue utilizavel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ColorTokens {
    /// Fundo da janela.
    pub background: Color,
    /// Fundo de paineis sobrepostos ao fundo (sidebar, cards).
    pub surface: Color,
    /// Segundo nivel de elevacao (menus, popovers, modais).
    pub surface_raised: Color,
    /// Fundo do player.
    pub player_background: Color,
    /// Fundo da sidebar.
    pub sidebar_background: Color,

    /// Texto principal.
    pub text: Color,
    /// Texto secundario (artista, metadados).
    pub text_muted: Color,
    /// Texto sobre a cor de destaque.
    pub text_on_accent: Color,

    /// Cor de marca, usada em botao primario e progresso.
    pub accent: Color,
    /// Variacao do destaque em hover.
    pub accent_hover: Color,

    /// Separadores e contornos.
    pub border: Color,
    /// Realce de item sob o cursor.
    pub hover: Color,
    /// Realce de item selecionado.
    pub selected: Color,
    /// Anel de foco de teclado. Nunca deve ficar invisivel.
    pub focus_ring: Color,

    pub success: Color,
    pub warning: Color,
    pub danger: Color,

    /// Sombra projetada por superficies elevadas.
    pub shadow: Color,
    /// Cor do trilho de scroll.
    pub scrollbar: Color,
}

impl Default for ColorTokens {
    fn default() -> Self {
        // Padrao = tema escuro embutido. Serve tambem como fallback quando um
        // tema do usuario falha ao carregar.
        Self {
            background: Color::rgb(0x0b, 0x0d, 0x12),
            surface: Color::rgb(0x12, 0x15, 0x1d),
            surface_raised: Color::rgb(0x1a, 0x1e, 0x29),
            player_background: Color::rgb(0x0f, 0x12, 0x19),
            sidebar_background: Color::rgb(0x0d, 0x10, 0x16),
            text: Color::rgb(0xec, 0xef, 0xf4),
            text_muted: Color::rgb(0x93, 0x9c, 0xb0),
            text_on_accent: Color::rgb(0x08, 0x0a, 0x0e),
            accent: Color::rgb(0x6d, 0xd4, 0x9e),
            accent_hover: Color::rgb(0x87, 0xe4, 0xb2),
            border: Color::rgba(0xff, 0xff, 0xff, 0x14),
            hover: Color::rgba(0xff, 0xff, 0xff, 0x0d),
            selected: Color::rgba(0xff, 0xff, 0xff, 0x1a),
            focus_ring: Color::rgb(0x6d, 0xd4, 0x9e),
            success: Color::rgb(0x5c, 0xc9, 0x8a),
            warning: Color::rgb(0xe4, 0xb3, 0x5c),
            danger: Color::rgb(0xe4, 0x6d, 0x6d),
            shadow: Color::rgba(0x00, 0x00, 0x00, 0x80),
            scrollbar: Color::rgba(0xff, 0xff, 0xff, 0x24),
        }
    }
}

/// Familia e escala tipografica.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TypographyTokens {
    /// Familia principal. Vazio = fonte padrao do sistema.
    pub family: String,
    /// Familia de titulos. Vazio = usa `family`.
    pub display_family: String,
    /// Nome de arquivo dentro de `fonts/` no pacote, se o tema traz a fonte.
    pub bundled_font: Option<String>,

    /// Multiplicador global aplicado a todos os tamanhos.
    ///
    /// Existe para que o usuario aumente a UI inteira sem editar 8 valores; a
    /// escala de DPI do Windows e aplicada por cima disto, nao no lugar dele.
    pub scale: f32,

    pub size_xs: f32,
    pub size_sm: f32,
    pub size_md: f32,
    pub size_lg: f32,
    pub size_xl: f32,
    pub size_display: f32,

    pub weight_normal: i32,
    pub weight_medium: i32,
    pub weight_bold: i32,

    /// Altura de linha como multiplo do tamanho da fonte.
    pub line_height: f32,
    /// Espacamento entre letras em pixels de design.
    pub letter_spacing: f32,
}

impl Default for TypographyTokens {
    fn default() -> Self {
        Self {
            family: String::new(),
            display_family: String::new(),
            bundled_font: None,
            scale: 1.0,
            size_xs: 11.0,
            size_sm: 12.0,
            size_md: 14.0,
            size_lg: 17.0,
            size_xl: 22.0,
            size_display: 32.0,
            weight_normal: 400,
            weight_medium: 500,
            weight_bold: 700,
            line_height: 1.4,
            letter_spacing: 0.0,
        }
    }
}

impl TypographyTokens {
    /// Tamanho ja multiplicado pela escala, arredondado para meio pixel.
    ///
    /// Arredondar evita baseline tremida quando a escala produz fracoes longas.
    pub fn scaled(&self, size: f32) -> f32 {
        ((size * self.scale) * 2.0).round() / 2.0
    }
}

/// Raios, espacamentos e espessuras.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ShapeTokens {
    pub radius_sm: f32,
    pub radius_md: f32,
    pub radius_lg: f32,
    /// Raio das capas de album. `0` deixa quadrado.
    pub radius_artwork: f32,
    /// Raio dos avatares de artista. Valor alto vira circulo.
    pub radius_avatar: f32,

    pub spacing_xs: f32,
    pub spacing_sm: f32,
    pub spacing_md: f32,
    pub spacing_lg: f32,
    pub spacing_xl: f32,

    pub border_width: f32,
    /// Espessura da barra de progresso do player.
    pub progress_thickness: f32,
    pub scrollbar_width: f32,
}

impl Default for ShapeTokens {
    fn default() -> Self {
        Self {
            radius_sm: 4.0,
            radius_md: 8.0,
            radius_lg: 14.0,
            radius_artwork: 6.0,
            radius_avatar: 999.0,
            spacing_xs: 4.0,
            spacing_sm: 8.0,
            spacing_md: 12.0,
            spacing_lg: 20.0,
            spacing_xl: 32.0,
            border_width: 1.0,
            progress_thickness: 4.0,
            scrollbar_width: 10.0,
        }
    }
}

/// Curva de aceleracao das animacoes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    #[default]
    EaseInOut,
}

/// Parametros de animacao.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MotionTokens {
    /// Desliga toda animacao. Respeitado tambem quando o Windows sinaliza
    /// preferencia por movimento reduzido.
    pub enabled: bool,
    /// Multiplicador global de duracao. `0.5` deixa tudo duas vezes mais rapido.
    pub speed: f32,
    /// Transicoes curtas (hover, foco), em milissegundos.
    pub duration_fast: i32,
    /// Transicoes de conteudo (troca de pagina).
    pub duration_normal: i32,
    /// Transicoes grandes (abrir "tocando agora").
    pub duration_slow: i32,
    pub easing: Easing,
}

impl Default for MotionTokens {
    fn default() -> Self {
        Self {
            enabled: true,
            speed: 1.0,
            duration_fast: 120,
            duration_normal: 200,
            duration_slow: 320,
            easing: Easing::EaseInOut,
        }
    }
}

impl MotionTokens {
    /// Duracao efetiva em milissegundos, ja considerando `enabled` e `speed`.
    pub fn effective(&self, base: i32) -> i32 {
        if !self.enabled {
            return 0;
        }
        ((base as f32) * self.speed.clamp(0.0, 5.0)).round() as i32
    }
}

/// Efeitos visuais que afetam custo de renderizacao.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EffectTokens {
    /// Opacidade da janela em `[0.2, 1.0]`.
    pub window_opacity: f32,
    /// Fundo translucido no estilo Mica/Acrylic do Windows, quando disponivel.
    pub acrylic: bool,
    /// Intensidade da sombra de superficies elevadas, em `[0.0, 1.0]`.
    pub shadow_strength: f32,
    /// Desfoque de fundo atras de modais, em pixels. `0` desliga.
    pub backdrop_blur: f32,
    /// Usa a capa do album para tingir o fundo da tela de reproducao.
    pub artwork_tint: bool,
    /// Intensidade do tingimento, em `[0.0, 1.0]`.
    pub artwork_tint_strength: f32,
}

impl Default for EffectTokens {
    fn default() -> Self {
        Self {
            window_opacity: 1.0,
            acrylic: false,
            shadow_strength: 0.5,
            backdrop_blur: 0.0,
            artwork_tint: true,
            artwork_tint_strength: 0.35,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typography_scale_rounds_to_half_pixel() {
        let t = TypographyTokens { scale: 1.1, ..Default::default() };
        assert_eq!(t.scaled(14.0), 15.5);

        let t = TypographyTokens { scale: 1.0, ..Default::default() };
        assert_eq!(t.scaled(14.0), 14.0);
    }

    #[test]
    fn motion_disabled_collapses_every_duration() {
        let mut m = MotionTokens::default();
        assert_eq!(m.effective(200), 200);
        m.enabled = false;
        assert_eq!(m.effective(200), 0);
    }

    #[test]
    fn motion_speed_is_clamped_to_a_sane_range() {
        let fast = MotionTokens { speed: 100.0, ..Default::default() };
        assert_eq!(fast.effective(100), 500);

        let negative = MotionTokens { speed: -1.0, ..Default::default() };
        assert_eq!(negative.effective(100), 0);
    }

    #[test]
    fn default_dark_palette_is_readable() {
        let c = ColorTokens::default();
        assert!(
            c.text.contrast_ratio(c.background) >= 7.0,
            "texto padrao deveria passar em AAA, deu {}",
            c.text.contrast_ratio(c.background)
        );
        assert!(c.text_muted.contrast_ratio(c.background) >= 4.5);
        assert!(c.text_on_accent.contrast_ratio(c.accent) >= 4.5);
    }

    #[test]
    fn tokens_round_trip_through_toml() {
        let c = ColorTokens::default();
        let text = toml::to_string(&c).unwrap();
        assert_eq!(toml::from_str::<ColorTokens>(&text).unwrap(), c);
    }

    #[test]
    fn partial_token_table_fills_in_defaults() {
        let c: ColorTokens = toml::from_str(r##"accent = "#ff0066""##).unwrap();
        assert_eq!(c.accent, Color::rgb(0xff, 0x00, 0x66));
        assert_eq!(c.background, ColorTokens::default().background);
    }

    #[test]
    fn unknown_token_is_rejected_so_typos_are_visible() {
        let err = toml::from_str::<ColorTokens>(r##"acccent = "#ff0066""##).unwrap_err();
        assert!(err.to_string().contains("acccent"), "erro pouco util: {err}");
    }
}
