use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Cor RGBA de 8 bits por canal.
///
/// Guardada como quatro bytes em vez de floats porque e o que a UI consome e o
/// que os temas escrevem; converter uma unica vez na fronteira evita divergencia
/// de arredondamento entre o que o autor do tema escreveu e o que aparece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ColorParseError {
    #[error("cor vazia")]
    Empty,
    #[error("formato de cor nao reconhecido: {0:?}")]
    Unrecognized(String),
    #[error("digito hexadecimal invalido em {0:?}")]
    InvalidHex(String),
    #[error("componente fora de faixa em {0:?}")]
    OutOfRange(String),
}

impl Color {
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Empacota como `0xAARRGGBB`, formato aceito diretamente pela UI.
    pub const fn to_argb_u32(self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    pub fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }

    /// Multiplica o alfa por um fator em `[0.0, 1.0]`.
    ///
    /// Usado pelos temas para derivar estados (hover, disabled) a partir de uma
    /// cor base, em vez de obrigar o autor a declarar cada variacao.
    pub fn scale_alpha(self, factor: f32) -> Self {
        let a = (self.a as f32 * factor.clamp(0.0, 1.0))
            .round()
            .clamp(0.0, 255.0);
        Self { a: a as u8, ..self }
    }

    /// Mistura linear entre duas cores. `t = 0.0` devolve `self`.
    pub fn mix(self, other: Color, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        Self {
            r: lerp(self.r, other.r),
            g: lerp(self.g, other.g),
            b: lerp(self.b, other.b),
            a: lerp(self.a, other.a),
        }
    }

    /// Luminancia relativa segundo WCAG 2.1.
    pub fn relative_luminance(self) -> f32 {
        fn channel(v: u8) -> f32 {
            let c = v as f32 / 255.0;
            if c <= 0.039_285_71 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * channel(self.r) + 0.7152 * channel(self.g) + 0.0722 * channel(self.b)
    }

    /// Razao de contraste WCAG entre duas cores, de 1.0 a 21.0.
    ///
    /// O validador de temas usa isto para avisar quando texto e fundo ficam
    /// ilegiveis -- um tema pode ser feio de proposito, mas nao deve ser
    /// acidentalmente impossivel de ler.
    pub fn contrast_ratio(self, other: Color) -> f32 {
        let (a, b) = (self.relative_luminance(), other.relative_luminance());
        let (lighter, darker) = if a >= b { (a, b) } else { (b, a) };
        (lighter + 0.05) / (darker + 0.05)
    }

    pub fn is_light(self) -> bool {
        self.relative_luminance() > 0.5
    }
}

impl FromStr for Color {
    type Err = ColorParseError;

    /// Aceita `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa` e `rgba(r, g, b, a)`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ColorParseError::Empty);
        }

        if let Some(hex) = s.strip_prefix('#') {
            return parse_hex(hex).ok_or_else(|| {
                if hex.chars().all(|c| c.is_ascii_hexdigit()) {
                    ColorParseError::Unrecognized(s.to_string())
                } else {
                    ColorParseError::InvalidHex(s.to_string())
                }
            });
        }

        if let Some(inner) = s.strip_prefix("rgba(").and_then(|r| r.strip_suffix(')')) {
            return parse_rgba(inner, s);
        }
        if let Some(inner) = s.strip_prefix("rgb(").and_then(|r| r.strip_suffix(')')) {
            return parse_rgba(inner, s);
        }

        Err(ColorParseError::Unrecognized(s.to_string()))
    }
}

fn parse_hex(hex: &str) -> Option<Color> {
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let d = |i: usize| -> Option<u8> { hex.as_bytes().get(i)?.to_ascii_lowercase().checked_sub(0) };
    let nib = |i: usize| -> Option<u8> {
        let c = *hex.as_bytes().get(i)? as char;
        c.to_digit(16).map(|v| v as u8)
    };
    let _ = d;

    match hex.len() {
        3 | 4 => {
            let expand = |v: u8| v * 17;
            Some(Color {
                r: expand(nib(0)?),
                g: expand(nib(1)?),
                b: expand(nib(2)?),
                a: if hex.len() == 4 { expand(nib(3)?) } else { 255 },
            })
        }
        6 | 8 => {
            let byte = |i: usize| -> Option<u8> { Some(nib(i)? * 16 + nib(i + 1)?) };
            Some(Color {
                r: byte(0)?,
                g: byte(2)?,
                b: byte(4)?,
                a: if hex.len() == 8 { byte(6)? } else { 255 },
            })
        }
        _ => None,
    }
}

fn parse_rgba(inner: &str, original: &str) -> Result<Color, ColorParseError> {
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    if parts.len() != 3 && parts.len() != 4 {
        return Err(ColorParseError::Unrecognized(original.to_string()));
    }

    let channel = |s: &str| -> Result<u8, ColorParseError> {
        let v: i64 = s
            .parse()
            .map_err(|_| ColorParseError::Unrecognized(original.to_string()))?;
        u8::try_from(v).map_err(|_| ColorParseError::OutOfRange(original.to_string()))
    };

    let alpha = match parts.get(3) {
        None => 255u8,
        Some(s) => {
            // Alfa aceita tanto 0-255 quanto 0.0-1.0, porque os dois formatos
            // aparecem naturalmente em temas escritos a mao.
            let f: f32 = s
                .parse()
                .map_err(|_| ColorParseError::Unrecognized(original.to_string()))?;
            if s.contains('.') {
                if !(0.0..=1.0).contains(&f) {
                    return Err(ColorParseError::OutOfRange(original.to_string()));
                }
                (f * 255.0).round() as u8
            } else {
                channel(s)?
            }
        }
    };

    Ok(Color {
        r: channel(parts[0])?,
        g: channel(parts[1])?,
        b: channel(parts[2])?,
        a: alpha,
    })
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.a == 255 {
            write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            write!(
                f,
                "#{:02x}{:02x}{:02x}{:02x}",
                self.r, self.g, self.b, self.a
            )
        }
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Color::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_supported_hex_form() {
        assert_eq!("#fff".parse::<Color>().unwrap(), Color::rgb(255, 255, 255));
        assert_eq!("#000".parse::<Color>().unwrap(), Color::rgb(0, 0, 0));
        assert_eq!(
            "#1a2b3c".parse::<Color>().unwrap(),
            Color::rgb(0x1a, 0x2b, 0x3c)
        );
        assert_eq!(
            "#1a2b3c80".parse::<Color>().unwrap(),
            Color::rgba(0x1a, 0x2b, 0x3c, 0x80)
        );
        assert_eq!(
            "#f00f".parse::<Color>().unwrap(),
            Color::rgba(255, 0, 0, 255)
        );
    }

    #[test]
    fn parses_rgb_and_rgba_functions() {
        assert_eq!(
            "rgb(12, 34, 56)".parse::<Color>().unwrap(),
            Color::rgb(12, 34, 56)
        );
        assert_eq!(
            "rgba(12,34,56,128)".parse::<Color>().unwrap(),
            Color::rgba(12, 34, 56, 128)
        );
        assert_eq!(
            "rgba(0, 0, 0, 0.5)".parse::<Color>().unwrap(),
            Color::rgba(0, 0, 0, 128)
        );
    }

    #[test]
    fn rejects_malformed_colors_with_a_useful_error() {
        assert_eq!("".parse::<Color>(), Err(ColorParseError::Empty));
        assert!(matches!(
            "#gg0000".parse::<Color>(),
            Err(ColorParseError::InvalidHex(_))
        ));
        assert!(matches!(
            "#12345".parse::<Color>(),
            Err(ColorParseError::Unrecognized(_))
        ));
        assert!(matches!(
            "periwinkle".parse::<Color>(),
            Err(ColorParseError::Unrecognized(_))
        ));
        assert!(matches!(
            "rgb(300, 0, 0)".parse::<Color>(),
            Err(ColorParseError::OutOfRange(_))
        ));
        assert!(matches!(
            "rgba(0,0,0,2.0)".parse::<Color>(),
            Err(ColorParseError::OutOfRange(_))
        ));
    }

    #[test]
    fn display_round_trips_through_parse() {
        for s in ["#1a2b3c", "#1a2b3c80"] {
            let c: Color = s.parse().unwrap();
            assert_eq!(c.to_string(), s);
            assert_eq!(c.to_string().parse::<Color>().unwrap(), c);
        }
    }

    #[test]
    fn argb_packing_matches_channel_order() {
        assert_eq!(
            Color::rgba(0x11, 0x22, 0x33, 0x44).to_argb_u32(),
            0x4411_2233
        );
    }

    #[test]
    fn contrast_ratio_matches_wcag_extremes() {
        let w = Color::rgb(255, 255, 255);
        let b = Color::rgb(0, 0, 0);
        assert!((w.contrast_ratio(b) - 21.0).abs() < 0.05);
        assert!((w.contrast_ratio(w) - 1.0).abs() < 0.001);
        // A razao nao depende da ordem dos argumentos.
        assert!((w.contrast_ratio(b) - b.contrast_ratio(w)).abs() < 0.001);
    }

    #[test]
    fn mix_interpolates_between_endpoints() {
        let a = Color::rgb(0, 0, 0);
        let b = Color::rgb(255, 255, 255);
        assert_eq!(a.mix(b, 0.0), a);
        assert_eq!(a.mix(b, 1.0), b);
        assert_eq!(a.mix(b, 0.5), Color::rgb(128, 128, 128));
    }

    #[test]
    fn scale_alpha_is_clamped() {
        let c = Color::rgba(1, 2, 3, 200);
        assert_eq!(c.scale_alpha(0.5).a, 100);
        assert_eq!(c.scale_alpha(5.0).a, 200);
        assert_eq!(c.scale_alpha(-1.0).a, 0);
    }

    #[test]
    fn serde_uses_the_textual_form() {
        let c = Color::rgba(0x1a, 0x2b, 0x3c, 0x80);
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(json, "\"#1a2b3c80\"");
        assert_eq!(serde_json::from_str::<Color>(&json).unwrap(), c);
    }
}
