//! Imagem de fundo da janela.
//!
//! O trabalho todo acontece **uma vez**, no carregamento: decodificar, reduzir
//! e desfocar. Depois disso o Slint so desenha um bitmap pronto, e o custo por
//! quadro e o mesmo de qualquer outra imagem na tela.
//!
//! Isso nao e uma otimizacao: e o requisito. O Morune toca enquanto a pessoa
//! joga, e um fundo que recalculasse desfoque a cada repaint disputaria GPU com
//! o jogo pelo resto da sessao.

use std::path::{Path, PathBuf};

use morune_theme::{BackgroundFit, BackgroundTokens};
use slint::SharedPixelBuffer;

/// Maior lado aceito depois da reducao.
///
/// Um JPEG de 8K decodificado ocupa ~130 MB de RAM para preencher uma janela
/// que raramente passa de 1600 px de largura. O teto e generoso o bastante para
/// telas 4K e ainda assim corta o caso patologico.
const MAX_DIMENSION: u32 = 3840;

/// Quanto o fundo e borrado para servir de vidro.
///
/// Em pixels da imagem ja reduzida. Vinte e quatro e o que some com a forma sem
/// virar um borrao de cor unica: o olho continua reconhecendo onde estava o
/// horizonte e onde estava o sol, que e o que faz o painel parecer transparente
/// em vez de pintado.
const RAIO_VIDRO: f32 = 24.0;

/// A imagem de fundo pronta para a interface, ja com o que a UI precisa saber.
#[derive(Debug, Clone, Default)]
pub struct Wallpaper {
    pub image: slint::Image,
    /// A mesma imagem, borrada, para os paineis usarem como vidro.
    ///
    /// **Existe porque o Slint nao desfoca o que esta atras de um elemento.**
    /// Nao ha filtro de fundo, e nao vai haver. Mas ha uma saida que da o mesmo
    /// resultado no caso que importa: se o que esta atras do painel e o fundo
    /// da janela, basta o painel mostrar o pedaco de uma copia JA borrada desse
    /// fundo, recortado exatamente na posicao dele. O olho nao distingue.
    ///
    /// **Custo por quadro: zero.** O borrao acontece uma vez, aqui, junto com a
    /// decodificacao -- pelo mesmo motivo escrito no topo deste arquivo. O que
    /// sobra para a interface e desenhar um pedaco de bitmap, que e a operacao
    /// mais barata que existe.
    ///
    /// Vazia quando o tema nao tem imagem de fundo. Nesse caso nao ha o que
    /// mostrar atraves, e o painel cai na cor de superficie do tema.
    pub blurred: slint::Image,
    /// `0` cover, `1` contain, `2` center, `3` stretch.
    pub fit: i32,
    pub opacity: f32,
    pub tint_strength: f32,
}

/// Traduz o modo de encaixe para o codigo que a interface espera.
///
/// A interface fala em inteiro pelo mesmo motivo que ja faz isso com
/// `sidebar-position` e `player-position`: o Slint nao tem enum vindo do Rust.
fn fit_code(fit: BackgroundFit) -> i32 {
    match fit {
        BackgroundFit::Cover => 0,
        BackgroundFit::Contain => 1,
        BackgroundFit::Center => 2,
        BackgroundFit::Stretch => 3,
    }
}

/// Resolve o caminho da imagem declarada por um tema.
///
/// `sanitize` ja recusou caminhos absolutos e `..`, mas isto e a segunda
/// tranca: um `.musicpack` importado pode ter escrito o `theme.toml` por fora
/// do caminho que a crate de tema valida, e o custo de conferir de novo aqui e
/// uma comparacao de prefixo.
fn resolve_in_theme(theme_dir: &Path, relative: &str) -> Option<PathBuf> {
    if relative.is_empty() {
        return None;
    }
    let candidate = theme_dir.join(relative);
    let (Ok(root), Ok(full)) = (theme_dir.canonicalize(), candidate.canonicalize()) else {
        tracing::debug!(path = %candidate.display(), "fundo do tema nao existe");
        return None;
    };
    if !full.starts_with(&root) {
        tracing::warn!(path = %full.display(), "fundo do tema aponta para fora do tema, ignorado");
        return None;
    }
    Some(full)
}

/// Carrega o fundo de um tema, ou de um arquivo escolhido pelo usuario.
///
/// `user_image`, quando presente, **substitui** a imagem do tema -- a mesma
/// regra da escala tipografica: preferencia de pessoa sobrevive a troca de
/// tema. O resto dos ajustes (encaixe, opacidade, veu) continua vindo de onde
/// o chamador mandar.
pub fn load(
    theme_dir: Option<&Path>,
    tokens: &BackgroundTokens,
    user_image: Option<&Path>,
) -> Wallpaper {
    let empty = Wallpaper {
        fit: fit_code(tokens.fit),
        opacity: tokens.opacity,
        tint_strength: tokens.tint_strength,
        ..Default::default()
    };

    let path = match user_image {
        Some(p) if p.is_file() => p.to_path_buf(),
        Some(p) => {
            // A imagem do usuario sumiu do disco. A janela volta a ser cor
            // solida em vez de ficar sem fundo nenhum, e o aviso fica no log.
            tracing::warn!(path = %p.display(), "imagem de fundo do usuario nao existe mais");
            return empty;
        }
        None => match theme_dir.and_then(|d| resolve_in_theme(d, &tokens.image)) {
            Some(p) => p,
            None => return empty,
        },
    };

    let Ok(decoded) = slint::Image::load_from_path(&path) else {
        tracing::warn!(path = %path.display(), "fundo nao decodificou");
        return empty;
    };

    let (image, blurred) = match decoded.to_rgba8() {
        // Sem acesso aos pixels nao da para reduzir nem desfocar, mas a imagem
        // continua desenhavel: entregar como veio e melhor que descartar. Sem
        // pixels tambem nao ha copia borrada, e o vidro cai na cor do tema.
        None => (decoded, slint::Image::default()),
        Some(buffer) => {
            let buffer = downscale(buffer, MAX_DIMENSION);

            // A copia do vidro sai da imagem JA reduzida e ANTES do borrao que
            // o tema pede. Se o tema ja borra o fundo, borrar de novo por cima
            // daria um segundo borrao somado -- e o vidro deixaria de mostrar o
            // que esta atras dele para mostrar so uma mancha.
            let blurred = slint::Image::from_rgba8(blur(buffer.clone(), RAIO_VIDRO));

            let buffer = if tokens.blur > 0.0 {
                blur(buffer, tokens.blur)
            } else {
                buffer
            };
            (slint::Image::from_rgba8(buffer), blurred)
        }
    };

    Wallpaper {
        image,
        blurred,
        ..empty
    }
}

type Rgba = SharedPixelBuffer<slint::Rgba8Pixel>;

/// Reduz por fator inteiro ate caber em `max`, com media de area.
///
/// Fator inteiro em vez de escala arbitraria porque a media de area fica exata
/// e barata: cada pixel de saida le um bloco `f x f` e nada mais. Uma imagem
/// que ja cabe volta intacta, sem copia.
fn downscale(buffer: Rgba, max: u32) -> Rgba {
    let (w, h) = (buffer.width(), buffer.height());
    let factor = (w.div_ceil(max)).max(h.div_ceil(max)).max(1);
    if factor == 1 {
        return buffer;
    }

    let (nw, nh) = ((w / factor).max(1), (h / factor).max(1));
    let src = buffer.as_slice();
    let mut out = SharedPixelBuffer::<slint::Rgba8Pixel>::new(nw, nh);
    let dst = out.make_mut_slice();

    for y in 0..nh {
        for x in 0..nw {
            let (mut r, mut g, mut b, mut a, mut n) = (0u32, 0u32, 0u32, 0u32, 0u32);
            for sy in y * factor..((y + 1) * factor).min(h) {
                for sx in x * factor..((x + 1) * factor).min(w) {
                    let p = src[(sy * w + sx) as usize];
                    r += p.r as u32;
                    g += p.g as u32;
                    b += p.b as u32;
                    a += p.a as u32;
                    n += 1;
                }
            }
            let n = n.max(1);
            dst[(y * nw + x) as usize] = slint::Rgba8Pixel {
                r: (r / n) as u8,
                g: (g / n) as u8,
                b: (b / n) as u8,
                a: (a / n) as u8,
            };
        }
    }
    out
}

/// Desfoque de caixa em tres passadas.
///
/// Tres caixas seguidas aproximam uma gaussiana bem o bastante para um fundo, a
/// um custo linear no numero de pixels -- uma gaussiana de verdade nao mudaria
/// nada visivel aqui e custaria varias vezes mais.
fn blur(mut buffer: Rgba, radius: f32) -> Rgba {
    let r = (radius.round() as i32).clamp(1, 64);
    for _ in 0..3 {
        buffer = box_pass(buffer, r, true);
        buffer = box_pass(buffer, r, false);
    }
    buffer
}

/// Uma passada de media em uma das direcoes, com soma corrente.
///
/// A janela nao e resomada a cada pixel: entra o pixel que passa a fazer parte
/// dela e sai o que deixou de fazer. Isso torna o custo **independente do
/// raio** -- a versao ingenua era O(pixels x raio) e um desfoque de 18 px numa
/// imagem de 1600x900 segurava a abertura da janela por quase quatro segundos.
fn box_pass(buffer: Rgba, radius: i32, horizontal: bool) -> Rgba {
    let (w, h) = (buffer.width() as i32, buffer.height() as i32);
    let src = buffer.as_slice().to_vec();
    let mut out = SharedPixelBuffer::<slint::Rgba8Pixel>::new(w as u32, h as u32);
    let dst = out.make_mut_slice();

    let (outer, inner) = if horizontal { (h, w) } else { (w, h) };
    let idx = |o: i32, i: i32| (if horizontal { o * w + i } else { i * w + o }) as usize;
    let n = (2 * radius + 1) as u32;

    for o in 0..outer {
        // A janela comeca centrada no primeiro pixel. Tudo que cai antes do
        // inicio da linha repete o pixel da ponta, em vez de somar zeros: somar
        // zeros deixaria uma moldura escura em volta da imagem.
        let first = src[idx(o, 0)];
        let (mut r, mut g, mut b, mut a) = (
            first.r as u32 * (radius as u32 + 1),
            first.g as u32 * (radius as u32 + 1),
            first.b as u32 * (radius as u32 + 1),
            first.a as u32 * (radius as u32 + 1),
        );
        for k in 1..=radius.min(inner - 1) {
            let p = src[idx(o, k)];
            r += p.r as u32;
            g += p.g as u32;
            b += p.b as u32;
            a += p.a as u32;
        }
        // Linha mais curta que o raio: o que faltou tambem repete a ponta.
        if radius > inner - 1 {
            let last = src[idx(o, inner - 1)];
            let missing = (radius - (inner - 1)) as u32;
            r += last.r as u32 * missing;
            g += last.g as u32 * missing;
            b += last.b as u32 * missing;
            a += last.a as u32 * missing;
        }

        for i in 0..inner {
            dst[idx(o, i)] = slint::Rgba8Pixel {
                r: (r / n) as u8,
                g: (g / n) as u8,
                b: (b / n) as u8,
                a: (a / n) as u8,
            };

            let entra = src[idx(o, (i + radius + 1).clamp(0, inner - 1))];
            let sai = src[idx(o, (i - radius).clamp(0, inner - 1))];
            r = r + entra.r as u32 - sai.r as u32;
            g = g + entra.g as u32 - sai.g as u32;
            b = b + entra.b as u32 - sai.b as u32;
            a = a + entra.a as u32 - sai.a as u32;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PNG 8x8 valido, embutido em bytes.
    ///
    /// Escrito a mao em vez de gerado: o codificador de PNG so existe na
    /// feature `snapshot`, e este teste tem de rodar na suite normal -- e
    /// justamente ele que cobre a carga do fundo de ponta a ponta.
    const PNG_8X8: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08, 0x08, 0x06, 0x00, 0x00, 0x00, 0xc4,
        0x0f, 0xbe, 0x8b, 0x00, 0x00, 0x00, 0x13, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0x38,
        0x71, 0xe2, 0xc4, 0x7f, 0x7c, 0x98, 0x61, 0x64, 0x28, 0x00, 0x00, 0x29, 0xda, 0xd5, 0xc1,
        0x23, 0x3c, 0xdd, 0x70, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60,
        0x82,
    ];

    fn solid(w: u32, h: u32, v: u8) -> Rgba {
        let mut b = SharedPixelBuffer::<slint::Rgba8Pixel>::new(w, h);
        for p in b.make_mut_slice() {
            *p = slint::Rgba8Pixel {
                r: v,
                g: v,
                b: v,
                a: 255,
            };
        }
        b
    }

    #[test]
    fn an_image_that_already_fits_is_not_resampled() {
        let b = downscale(solid(100, 100, 7), 3840);
        assert_eq!((b.width(), b.height()), (100, 100));
    }

    #[test]
    fn an_oversized_image_is_pulled_under_the_cap() {
        let b = downscale(solid(8000, 4000, 7), 3840);
        assert!(b.width() <= 3840 && b.height() <= 3840, "{}", b.width());
        // Cor chapada sobrevive a media de area: se nao sobreviver, a conta
        // esta somando pixels que nao existem.
        assert_eq!(b.as_slice()[0].r, 7);
    }

    #[test]
    fn blurring_a_flat_image_changes_nothing() {
        // Inclui a borda: se o clamp das pontas estivesse errado, os pixels da
        // moldura escureceriam e este teste pegaria.
        let b = blur(solid(32, 32, 200), 8.0);
        assert!(b.as_slice().iter().all(|p| p.r == 200 && p.a == 255));
    }

    #[test]
    fn slint_decodes_svg_at_runtime() {
        // Sonda de dependencia: os icones por tema so podem ser SVG se o
        // decodificador estiver ligado nas features do `slint`. Se este teste
        // falhar, o caminho de icones custom precisa de PNG.
        let dir = std::env::temp_dir().join("morune-svg-probe");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("probe.svg");
        std::fs::write(
            &file,
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><rect width="24" height="24"/></svg>"#,
        )
        .unwrap();
        let image = slint::Image::load_from_path(&file);
        assert!(image.is_ok(), "slint nao decodificou SVG: {image:?}");
        assert_eq!(image.unwrap().size().width, 24);
    }

    #[test]
    fn blurring_preserves_the_average_of_a_gradient() {
        // Um degrade horizontal borrado continua sendo o mesmo degrade no
        // miolo: media de vizinhos simetricos nao desloca o valor central.
        // E o teste que pega erro de sinal na soma corrente, que um teste com
        // cor chapada nao pegaria.
        let (w, h) = (64u32, 4u32);
        let mut b = SharedPixelBuffer::<slint::Rgba8Pixel>::new(w, h);
        for (i, p) in b.make_mut_slice().iter_mut().enumerate() {
            let v = ((i as u32 % w) * 4) as u8;
            *p = slint::Rgba8Pixel {
                r: v,
                g: v,
                b: v,
                a: 255,
            };
        }
        let out = blur(b, 5.0);
        let px = out.as_slice();
        for x in 20..44u32 {
            let esperado = (x * 4) as i32;
            let obtido = px[(2 * w + x) as usize].r as i32;
            assert!(
                (obtido - esperado).abs() <= 2,
                "x={x}: esperado ~{esperado}, obtido {obtido}"
            );
        }
    }

    /// O custo do desfoque nao pode crescer com o raio.
    ///
    /// Medir milissegundos absolutos daria um teste que passa ou falha
    /// conforme a maquina. O que importa e a **forma** do custo: com soma
    /// corrente, dobrar o raio nao muda nada. A versao ingenua era
    /// O(pixels x raio) e um raio de 18 px segurava a abertura da janela por
    /// quase quatro segundos.
    #[test]
    fn blur_cost_does_not_grow_with_the_radius() {
        let medir = |raio: f32| {
            // Uma passada de aquecimento tira o custo da primeira alocacao da
            // conta, que senao apareceria toda no primeiro raio medido.
            let _ = blur(solid(400, 400, 128), raio);
            let t = std::time::Instant::now();
            let _ = blur(solid(400, 400, 128), raio);
            t.elapsed().as_secs_f64()
        };

        let estreito = medir(2.0);
        let largo = medir(48.0);
        println!("raio 2: {estreito:.4}s | raio 48: {largo:.4}s");
        assert!(
            largo < estreito * 4.0,
            "raio 48 custou {largo:.4}s contra {estreito:.4}s do raio 2:              o desfoque voltou a depender do raio"
        );
    }

    #[test]
    fn a_theme_image_is_actually_loaded() {
        // Teste de ponta a ponta do caminho de carga: pasta de tema real,
        // arquivo real, caminho relativo real. Os testes de unidade acima
        // cobrem a matematica; este cobre o que de fato quebrou.
        let dir = std::env::temp_dir().join("morune-wallpaper-e2e");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("assets/backgrounds")).unwrap();

        std::fs::write(dir.join("assets/backgrounds/f.png"), PNG_8X8).unwrap();

        let tokens = BackgroundTokens {
            image: "assets/backgrounds/f.png".into(),
            ..Default::default()
        };
        let paper = load(Some(&dir), &tokens, None);
        assert!(
            paper.image.size().width > 0,
            "imagem do tema nao chegou na interface"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_background_path_escaping_the_theme_is_refused() {
        let dir = std::env::temp_dir();
        assert!(resolve_in_theme(&dir, "../../../windows/win.ini").is_none());
        assert!(resolve_in_theme(&dir, "").is_none());
    }
}
