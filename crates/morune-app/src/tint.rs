//! Cor dominante da capa do album.
//!
//! Serve ao `artwork_tint` do tema: a interface pega emprestada uma cor do que
//! esta tocando. E a unica customizacao que um tocador de musica tem e um papel
//! de parede nao -- ela muda sozinha, faixa a faixa.
//!
//! O calculo acontece **uma vez por capa** e o resultado fica guardado no
//! estado. Nada aqui roda por quadro.

use std::path::Path;

use slint::Color;

/// Lado da miniatura usada para o calculo.
///
/// A conta nao precisa da capa inteira: 32x32 ja separa as cores que importam,
/// e reduzir primeiro deixa o histograma barato e estavel -- duas capas
/// parecidas nao dao cores diferentes so por causa de ruido de JPEG.
const THUMB: u32 = 32;

/// Quantos niveis por canal o histograma usa.
///
/// 5 bits por canal (32 niveis) e o suficiente para agrupar tons vizinhos sem
/// juntar cores que a pessoa veria como diferentes.
const LEVELS: u32 = 32;

/// Cor dominante de uma capa, ou `None` se a imagem nao abrir.
///
/// "Dominante" nao e "media": a media de uma foto e sempre um cinza barrento.
/// O criterio aqui e **quantidade vezes saturacao**, que e o que faz um vinil
/// vermelho num fundo preto devolver o vermelho, e nao o preto.
pub fn dominant(path: &Path) -> Option<Color> {
    let image = slint::Image::load_from_path(path).ok()?;
    let buffer = image.to_rgba8()?;
    dominant_pixels(buffer.as_slice(), buffer.width(), buffer.height())
}

/// A conta em si, separada do carregamento.
///
/// Separada para poder ser testada sem gravar imagem em disco: o que pode dar
/// errado aqui e o criterio de escolha, nao o decodificador do Slint.
fn dominant_pixels(src: &[slint::Rgba8Pixel], w: u32, h: u32) -> Option<Color> {
    if w == 0 || h == 0 || src.len() < (w * h) as usize {
        return None;
    }
    let step_x = (w / THUMB).max(1);
    let step_y = (h / THUMB).max(1);

    // Cada balde guarda soma de r, g, b e a contagem, para devolver a media
    // dentro do balde vencedor em vez do centro dele -- o centro do balde daria
    // uma cor levemente errada, sempre a mesma, em qualquer capa.
    let mut buckets = vec![(0u32, 0u32, 0u32, 0u32); (LEVELS * LEVELS * LEVELS) as usize];

    for y in (0..h).step_by(step_y as usize) {
        for x in (0..w).step_by(step_x as usize) {
            let p = src[(y * w + x) as usize];
            // Pixel transparente nao e cor da capa, e uma capa com canto
            // recortado daria peso a um transparente que ninguem ve.
            if p.a < 128 {
                continue;
            }
            let shift = 8 - LEVELS.trailing_zeros();
            let key = ((p.r as u32 >> shift) * LEVELS * LEVELS)
                + ((p.g as u32 >> shift) * LEVELS)
                + (p.b as u32 >> shift);
            let b = &mut buckets[key as usize];
            b.0 += p.r as u32;
            b.1 += p.g as u32;
            b.2 += p.b as u32;
            b.3 += 1;
        }
    }

    let medias: Vec<(u8, u8, u8, u32)> = buckets
        .into_iter()
        .filter(|b| b.3 > 0)
        .map(|(r, g, b, n)| ((r / n) as u8, (g / n) as u8, (b / n) as u8, n))
        .collect();

    // Duas passadas. A primeira ignora quase-preto e quase-branco: sao as cores
    // que mais aparecem numa capa e as piores para tingir qualquer coisa -- um
    // encarte com fundo preto devolveria preto, que nao diz nada sobre o disco.
    // Se a capa for genuinamente monocromatica e nada sobreviver ao filtro, a
    // segunda passada aceita tudo, porque devolver uma cor ruim ainda e melhor
    // que devolver nenhuma.
    escolher(&medias, true)
        .or_else(|| escolher(&medias, false))
        .map(|(r, g, b)| Color::from_rgb_u8(r, g, b))
}

/// Faixa de luminancia aceita na primeira passada.
const LUZ_MIN: f32 = 0.10;
const LUZ_MAX: f32 = 0.92;

fn escolher(medias: &[(u8, u8, u8, u32)], filtrar: bool) -> Option<(u8, u8, u8)> {
    let mut melhor: Option<(f32, u8, u8, u8)> = None;
    for &(r, g, b, n) in medias {
        if filtrar {
            let luz = luminancia(r, g, b);
            if !(LUZ_MIN..=LUZ_MAX).contains(&luz) {
                continue;
            }
        }
        // O piso somado a saturacao evita que uma capa cinza zere todas as
        // notas e o desempate vire sorteio.
        let nota = n as f32 * (0.10 + saturacao(r, g, b));
        if melhor.is_none_or(|(atual, ..)| nota > atual) {
            melhor = Some((nota, r, g, b));
        }
    }
    melhor.map(|(_, r, g, b)| (r, g, b))
}

/// Luminancia percebida em `[0, 1]`.
fn luminancia(r: u8, g: u8, b: u8) -> f32 {
    (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) / 255.0
}

/// Saturacao em `[0, 1]`, no sentido de HSL.
///
/// Usada junto com a contagem para decidir qual cor representa a capa.
fn saturacao(r: u8, g: u8, b: u8) -> f32 {
    let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    if max <= f32::EPSILON {
        return 0.0;
    }
    (max - min) / max
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(r: u8, g: u8, b: u8) -> slint::Rgba8Pixel {
        slint::Rgba8Pixel { r, g, b, a: 255 }
    }

    #[test]
    fn a_capa_de_uma_cor_devolve_essa_cor() {
        let pixels = vec![px(200, 40, 60); 64 * 64];
        let c = dominant_pixels(&pixels, 64, 64).unwrap();
        assert!(
            (c.red() as i32 - 200).abs() <= 8 && (c.green() as i32 - 40).abs() <= 8,
            "{c:?}"
        );
    }

    #[test]
    fn a_cor_saturada_vence_o_fundo_preto_mesmo_sendo_minoria() {
        // O caso que motiva o criterio: um vinil vermelho pequeno sobre muito
        // preto. A media daria quase preto; o esperado e o vermelho.
        let (w, h) = (64u32, 64u32);
        let mut pixels = vec![px(8, 8, 10); (w * h) as usize];
        for y in 20..44 {
            for x in 20..44 {
                pixels[(y * w + x) as usize] = px(220, 30, 40);
            }
        }
        let c = dominant_pixels(&pixels, w, h).unwrap();
        assert!(
            c.red() > 150 && c.green() < 90,
            "esperava o vermelho do vinil, veio {c:?}"
        );
    }

    #[test]
    fn pixel_transparente_nao_conta_como_cor() {
        let (w, h) = (32u32, 32u32);
        let mut pixels = vec![
            slint::Rgba8Pixel {
                r: 255,
                g: 255,
                b: 255,
                a: 0,
            };
            (w * h) as usize
        ];
        for p in pixels.iter_mut().take(64) {
            *p = px(30, 180, 90);
        }
        let c = dominant_pixels(&pixels, w, h).unwrap();
        assert!(c.green() > 120 && c.red() < 90, "{c:?}");
    }

    #[test]
    fn uma_capa_inteira_escura_ainda_devolve_alguma_cor() {
        // Exercita a segunda passada: se o filtro de luminancia descartar tudo,
        // devolver `None` deixaria a interface sem tint e sem explicacao.
        let pixels = vec![px(6, 6, 9); 32 * 32];
        let c = dominant_pixels(&pixels, 32, 32).unwrap();
        assert!(c.red() < 20 && c.blue() < 20, "{c:?}");
    }

    #[test]
    fn um_arquivo_que_nao_abre_nao_derruba_nada() {
        assert!(dominant(std::path::Path::new("nao-existe.png")).is_none());
    }
}
