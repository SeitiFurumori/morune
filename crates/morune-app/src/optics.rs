//! Lente de borda sobre a cena do tema, sem ler a tela nem o framebuffer.
//!
//! A fonte e o wallpaper: isto NAO refrata listas/controles que passam atras.
//! A amostragem e limitada e cacheada; em repouso nao ha timer nem trabalho.

use std::collections::VecDeque;

use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

use crate::ui::LensRegion;

// Material regular usa uma fonte suave; 320 px limitam o trabalho em resize.
const MAX_SIDE: f32 = 320.0;
const CACHE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Default)]
pub struct LensCache {
    entries: VecDeque<(Image, LensRegion, Image, usize)>,
    bytes: usize,
}

impl LensCache {
    pub fn image(&mut self, source: Image, region: LensRegion) -> Image {
        if let Some(index) = self
            .entries
            .iter()
            .position(|(image, geometry, _, _)| *image == source && *geometry == region)
        {
            let entry = self.entries.remove(index).unwrap();
            let image = entry.2.clone();
            self.entries.push_back(entry);
            return image;
        }
        let Some(pixels) = source.to_rgba8() else {
            return Image::default();
        };
        let Some(rendered) = render(&pixels, &region) else {
            return Image::default();
        };
        let bytes = rendered.as_bytes().len();
        let image = Image::from_rgba8(rendered);
        while self.bytes + bytes > CACHE_BYTES || self.entries.len() >= 12 {
            let Some((_, _, _, size)) = self.entries.pop_front() else {
                break;
            };
            self.bytes -= size;
        }
        self.bytes += bytes;
        self.entries
            .push_back((source, region, image.clone(), bytes));
        image
    }
}

/// Densidade minima do suporte de leitura para a cena conhecida.
/// Avaliada quando a imagem muda; nao cria trabalho por quadro em repouso.
pub fn readable_tint(
    source: &Image,
    tint: morune_theme::Color,
    spec: &morune_theme::ThemeSpec,
) -> morune_theme::Color {
    readable_tint_inside(source, tint, spec, 0)
}

/// A orla especular nao fica atras do texto e nao deve opacificar o painel.
pub fn readable_glass_tint(
    source: &Image,
    tint: morune_theme::Color,
    spec: &morune_theme::ThemeSpec,
) -> morune_theme::Color {
    readable_tint_inside(source, tint, spec, 4)
}

fn readable_tint_inside(
    source: &Image,
    tint: morune_theme::Color,
    spec: &morune_theme::ThemeSpec,
    inset: u32,
) -> morune_theme::Color {
    use morune_theme::Color;
    let light_text = spec.colors.text.relative_luminance() > 0.4;
    let mut channels = if light_text { [0u8; 3] } else { [255u8; 3] };
    let mut found = false;
    if let Some(pixels) = source.to_rgba8() {
        let inset = inset.min(pixels.width() / 4).min(pixels.height() / 4);
        for (index, pixel) in pixels.as_slice().iter().enumerate() {
            let x = index as u32 % pixels.width();
            let y = index as u32 / pixels.width();
            if pixel.a <= 250
                || x < inset
                || y < inset
                || x >= pixels.width() - inset
                || y >= pixels.height() - inset
            {
                continue;
            }
            found = true;
            for (bound, value) in channels.iter_mut().zip([pixel.r, pixel.g, pixel.b]) {
                *bound = if light_text {
                    (*bound).max(value)
                } else {
                    (*bound).min(value)
                };
            }
        }
    }
    let scene = if found {
        Color::rgb(channels[0], channels[1], channels[2])
    } else {
        spec.colors.background
    };
    let white = Color::rgb(255, 255, 255);
    for alpha in tint.a..=255 {
        let candidate = Color::rgba(tint.r, tint.g, tint.b, alpha);
        let panel = candidate.over(scene);
        let panel = if light_text {
            let panel = white
                .scale_alpha(spec.effects.artwork_tint_strength)
                .over(panel);
            white.scale_alpha(0.1 * spec.effects.gloss).over(panel)
        } else {
            panel
        };
        let readable = [Color::TRANSPARENT, spec.colors.hover, spec.colors.selected]
            .iter()
            .all(|state| {
                let background = state.over(panel);
                [spec.colors.text, spec.colors.text_muted]
                    .iter()
                    .all(|ink| ink.over(background).contrast_ratio(background) >= 4.5)
            });
        if readable {
            return candidate;
        }
    }
    Color::rgba(tint.r, tint.g, tint.b, 255)
}

fn valid(r: &LensRegion) -> bool {
    [
        r.x,
        r.y,
        r.width,
        r.height,
        r.radius,
        r.window_width,
        r.window_height,
    ]
    .iter()
    .all(|v| v.is_finite())
        && r.width > 0.0
        && r.height > 0.0
        && r.window_width > 0.0
        && r.window_height > 0.0
}

/// Mapeia a janela na mesma imagem usada pelo ImageFit da UI.
fn source_point(x: f32, y: f32, iw: f32, ih: f32, r: &LensRegion) -> (f32, f32) {
    let sx = r.window_width / iw;
    let sy = r.window_height / ih;
    let scale = match r.fit {
        1 => sx.min(sy),
        2 => 1.0,
        _ => sx.max(sy),
    };
    if r.fit == 3 {
        return (x / sx, y / sy);
    }
    (
        (x - (r.window_width - iw * scale) * 0.5) / scale,
        (y - (r.window_height - ih * scale) * 0.5) / scale,
    )
}

/// Distancia assinada e normal do retangulo arredondado, em pixels logicos.
fn edge(x: f32, y: f32, width: f32, height: f32, radius: f32) -> (f32, f32, f32) {
    let px = x - width * 0.5;
    let py = y - height * 0.5;
    let radius = radius.max(0.0).min(width.min(height) * 0.5);
    let qx = px.abs() - (width * 0.5 - radius);
    let qy = py.abs() - (height * 0.5 - radius);
    let vx = qx.max(0.0);
    let vy = qy.max(0.0);
    let length = vx.hypot(vy);
    let distance = length + qx.max(qy).min(0.0) - radius;
    let (nx, ny) = if length > 0.0001 {
        (vx / length * px.signum(), vy / length * py.signum())
    } else if qx > qy {
        (px.signum(), 0.0)
    } else {
        (0.0, py.signum())
    };
    (distance, nx, ny)
}

fn sample(source: &SharedPixelBuffer<Rgba8Pixel>, x: f32, y: f32) -> Rgba8Pixel {
    // Fora de contain/center e transparente, nao uma faixa esticada da borda.
    if x < 0.0 || y < 0.0 || x >= source.width() as f32 || y >= source.height() as f32 {
        return Rgba8Pixel::default();
    }
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(source.width() - 1);
    let y1 = (y0 + 1).min(source.height() - 1);
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let pixels = source.as_slice();
    let at = |x, y| pixels[(y * source.width() + x) as usize];
    let samples = [at(x0, y0), at(x1, y0), at(x0, y1), at(x1, y1)];
    let weights = [
        (1.0 - fx) * (1.0 - fy),
        fx * (1.0 - fy),
        (1.0 - fx) * fy,
        fx * fy,
    ];
    let mut sums = [0.0f32; 4];
    for (p, weight) in samples.iter().zip(weights) {
        let alpha = p.a as f32 / 255.0;
        sums[0] += p.r as f32 * alpha * weight;
        sums[1] += p.g as f32 * alpha * weight;
        sums[2] += p.b as f32 * alpha * weight;
        sums[3] += alpha * weight;
    }
    if sums[3] < 0.0001 {
        return Rgba8Pixel::default();
    }
    Rgba8Pixel {
        r: (sums[0] / sums[3]).round() as u8,
        g: (sums[1] / sums[3]).round() as u8,
        b: (sums[2] / sums[3]).round() as u8,
        a: (sums[3] * 255.0).round() as u8,
    }
}

fn render(
    source: &SharedPixelBuffer<Rgba8Pixel>,
    r: &LensRegion,
) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    if !valid(r) || source.width() == 0 || source.height() == 0 {
        return None;
    }
    let scale = (MAX_SIDE / r.width.max(r.height)).min(1.0);
    let width = (r.width * scale).ceil().max(1.0) as u32;
    let height = (r.height * scale).ceil().max(1.0) as u32;
    let mut output = SharedPixelBuffer::<Rgba8Pixel>::new(width, height);
    let band = r.radius.clamp(4.0, 18.0).min(r.width.min(r.height) * 0.5);
    for (index, pixel) in output.make_mut_slice().iter_mut().enumerate() {
        let x = (index as u32 % width) as f32 / width as f32 * r.width + 0.5 / scale;
        let y = (index as u32 / width) as f32 / height as f32 * r.height + 0.5 / scale;
        let (distance, nx, ny) = edge(x, y, r.width, r.height, r.radius);
        if distance > 0.5 / scale {
            continue;
        }
        // Perfil de lente com centro plano: so a borda desloca a fonte.
        // Modelo perceptivo, nao uma simulacao de trajetoria volumetrica.
        let curve = (1.0 + distance / band).clamp(0.0, 1.0);
        let displacement = curve * curve * band * 0.48;
        let (sx, sy) = source_point(
            r.x + x - nx * displacement,
            r.y + y - ny * displacement,
            source.width() as f32,
            source.height() as f32,
            r,
        );
        *pixel = sample(source, sx, sy);
        // Reflexo direcional localizado; centro nao recebe veu branco.
        let facing = (-nx * 0.6 - ny * 0.8).max(0.0);
        let rim = (-distance.abs() * scale * 0.8).exp();
        let light = rim * (0.06 + facing * 0.30);
        pixel.r = (pixel.r as f32 * (1.0 - light) + 255.0 * light) as u8;
        pixel.g = (pixel.g as f32 * (1.0 - light) + 255.0 * light) as u8;
        pixel.b = (pixel.b as f32 * (1.0 - light) + 255.0 * light) as u8;
        let coverage = (0.5 - distance * scale).clamp(0.0, 1.0);
        pixel.a = (pixel.a as f32 * coverage).round() as u8;
    }
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region() -> LensRegion {
        LensRegion {
            width: 80.0,
            height: 60.0,
            radius: 12.0,
            window_width: 160.0,
            window_height: 120.0,
            ..Default::default()
        }
    }

    #[test]
    fn fit_modes_match_window_geometry() {
        let mut r = region();
        r.fit = 0;
        assert_eq!(source_point(80.0, 60.0, 100.0, 100.0, &r), (50.0, 50.0));
        r.fit = 1;
        let (x, _) = source_point(0.0, 60.0, 100.0, 100.0, &r);
        assert!(x < 0.0, "contain preserves side letterboxing");
        r.fit = 2;
        assert_eq!(source_point(30.0, 10.0, 100.0, 100.0, &r), (0.0, 0.0));
        r.fit = 3;
        let (x, y) = source_point(160.0, 120.0, 100.0, 100.0, &r);
        assert!((x - 100.0).abs() < 0.0001 && (y - 100.0).abs() < 0.0001);
    }

    #[test]
    fn geometry_and_allocations_are_bounded() {
        let input = SharedPixelBuffer::new(8, 8);
        let mut r = region();
        r.width = f32::NAN;
        assert!(render(&input, &r).is_none());
        r.width = 50000.0;
        r.height = 20000.0;
        let output = render(&input, &r).unwrap();
        assert!(output.width() <= MAX_SIDE as u32);
        assert!(output.height() <= MAX_SIDE as u32);
    }

    #[test]
    fn lens_displaces_grid_at_edge_but_keeps_center_and_rounded_mask() {
        let mut input = SharedPixelBuffer::<Rgba8Pixel>::new(160, 120);
        for (i, pixel) in input.make_mut_slice().iter_mut().enumerate() {
            *pixel = Rgba8Pixel {
                r: (i % 160) as u8,
                g: (i / 160) as u8,
                b: 0,
                a: 255,
            };
        }
        let output = render(&input, &region()).unwrap();
        let at = |x: usize, y: usize| output.as_slice()[y * 80 + x];
        assert_eq!(at(0, 0).a, 0, "rounded corner is transparent");
        assert_eq!(at(40, 30), sample(&input, 40.5, 30.5));
        // Desconta o reflexo branco usando B (a fonte tem B=0). Assim o
        // teste exige deslocamento de G, nao apenas clareamento da borda.
        let p = at(40, 3);
        let original_green = (f32::from(p.g) - f32::from(p.b)) / (1.0 - f32::from(p.b) / 255.0);
        assert!(original_green > 5.0, "edge must displace the source");
    }

    #[test]
    fn transparent_samples_do_not_add_black_fringes() {
        let mut input = SharedPixelBuffer::<Rgba8Pixel>::new(2, 1);
        input.make_mut_slice()[0] = Rgba8Pixel {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        };
        assert_eq!(
            sample(&input, 0.5, 0.0),
            Rgba8Pixel {
                r: 255,
                g: 255,
                b: 255,
                a: 128
            }
        );
    }

    #[test]
    fn repeated_scene_reuses_image_and_cache_evicts_old_geometry() {
        let source = Image::from_rgba8(SharedPixelBuffer::<Rgba8Pixel>::new(160, 120));
        let mut cache = LensCache::default();
        let first = cache.image(source.clone(), region());
        assert_eq!(first, cache.image(source.clone(), region()));
        assert_eq!(cache.entries.len(), 1);
        for x in 1..30 {
            cache.image(
                source.clone(),
                LensRegion {
                    x: x as f32,
                    ..region()
                },
            );
        }
        assert!(cache.entries.len() <= 12);
        assert!(cache.bytes <= CACHE_BYTES);
    }
}
