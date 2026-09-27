//! Captura da janela em PNG, a partir do renderizador do Slint.
//!
//! Usada para verificacao visual de temas. Capturar pelo renderizador, e nao
//! pela tela, tem duas vantagens que importam: funciona com a janela em segundo
//! plano, e nunca grava nada que esteja atras dela.

use std::path::Path;

use slint::ComponentHandle;

use crate::ui;

/// Salva a janela em `path`.
pub fn save(window: &ui::AppWindow, path: &Path) -> anyhow::Result<(u32, u32)> {
    let buffer = window.window().take_snapshot()?;
    let (width, height) = (buffer.width(), buffer.height());

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = std::fs::File::create(path)?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header()?;
    // `SharedPixelBuffer<Rgba8Pixel>` ja esta em RGBA de 8 bits sem padding de
    // linha, que e exatamente o formato que o codificador espera.
    let bytes: &[u8] = bytemuck_cast(buffer.as_slice());
    writer.write_image_data(bytes)?;
    writer.finish()?;

    Ok((width, height))
}

/// Reinterpreta os pixels como bytes.
///
/// `Rgba8Pixel` e `#[repr(C)]` com quatro `u8`, entao a fatia de pixels e, byte
/// a byte, a mesma coisa que a fatia de bytes esperada pelo codificador.
fn bytemuck_cast(pixels: &[slint::Rgba8Pixel]) -> &[u8] {
    // SAFETY: `Rgba8Pixel` e repr(C) com exatamente 4 campos u8 e alinhamento 1,
    // entao a conversao preserva tamanho, alinhamento e validade.
    #[allow(unsafe_code)]
    unsafe {
        std::slice::from_raw_parts(pixels.as_ptr() as *const u8, std::mem::size_of_val(pixels))
    }
}

/// Troca os dados da conta por dados de exemplo, para as fotos do README.
///
/// A foto publica nao pode mostrar a biblioteca de quem a tirou -- nomes de
/// playlist, nome de usuario, o que a pessoa ouve. O tema, o vidro e a cena
/// continuam os de verdade: so o conteudo e trocado, um instante antes da
/// captura. As capas vem de `pasta` (`c1.jpg` ... `c12.jpg`); capa que falta vira
/// a ficha com a inicial, como no aplicativo.
pub fn dados_de_exemplo(w: &ui::AppWindow, pasta: &Path) {
    use slint::{Image, ModelRc, SharedString, VecModel};

    let capa =
        |n: usize| Image::load_from_path(&pasta.join(format!("c{n}.jpg"))).unwrap_or_default();
    let faixa =
        |id: &str, titulo: &str, artista: &str, album: &str, n: usize, dur: &str| ui::TrackRow {
            id: id.into(),
            title: titulo.into(),
            artist: artista.into(),
            album: album.into(),
            cover: capa(n),
            initial: titulo
                .chars()
                .next()
                .map(String::from)
                .unwrap_or_default()
                .into(),
            hue: n as f32 / 12.0,
            cover_pending: false,
            duration: dur.into(),
            playable: true,
            playing: id == "4",
            favorite: true,
        };
    let cartao = |id: &str, titulo: &str, sub: &str, n: Option<usize>| ui::CardItem {
        id: id.into(),
        title: titulo.into(),
        subtitle: sub.into(),
        cover: n.map(capa).unwrap_or_default(),
        initial: titulo
            .chars()
            .next()
            .map(String::from)
            .unwrap_or_default()
            .into(),
        hue: titulo.len() as f32 / 20.0 % 1.0,
        cover_pending: false,
        pinned: false,
    };
    let lista = |v: Vec<ui::CardItem>| ModelRc::new(VecModel::from(v));

    let faixas = vec![
        faixa(
            "1",
            "Just the Two of Us",
            "Grover Washington, Jr., Bill Withers",
            "Winelight",
            1,
            "7:23",
        ),
        faixa(
            "2",
            "Ain't No Sunshine",
            "Bill Withers",
            "Just As I Am",
            2,
            "2:06",
        ),
        faixa("4", "Nights", "Frank Ocean", "Blonde", 3, "5:07"),
        faixa(
            "5",
            "Circle of Life",
            "Elton John",
            "The Lion King",
            4,
            "4:51",
        ),
        faixa(
            "6",
            "Slide",
            "Calvin Harris, Frank Ocean, Migos",
            "Funk Wav Bounces Vol.1",
            5,
            "3:50",
        ),
        faixa("7", "Ivy", "Frank Ocean", "Blonde", 6, "4:09"),
    ];
    let recentes: Vec<_> = [2, 4, 0, 5].iter().map(|&i| faixas[i].clone()).collect();
    let playlists = vec![
        cartao("p1", "Para estudar", "Playlist", Some(7)),
        cartao("p2", "Noite de jogo", "Playlist", None),
        cartao("p3", "Chuva lá fora", "Playlist", Some(8)),
        cartao("p4", "Clássicos", "Playlist", None),
        cartao("p5", "Viagem", "Playlist", Some(9)),
        cartao("p6", "Para dormir", "Playlist", None),
    ];
    let feitos = vec![
        cartao("m1", "Daily Mix 1", "Feito para você", Some(10)),
        cartao("m2", "Daily Mix 2", "Feito para você", Some(11)),
        cartao("m3", "Discover Weekly", "Feito para você", Some(12)),
        cartao("m4", "Radar de Novidades", "Feito para você", Some(1)),
        cartao("m5", "Daily Mix 3", "Feito para você", Some(5)),
        cartao("m6", "Repeat Rewind", "Feito para você", Some(2)),
        cartao("m7", "Descobertas", "Feito para você", Some(6)),
    ];

    w.set_logged_in(true);
    w.set_home_loaded(true);
    w.set_account_name(SharedString::from("você"));
    w.set_account_initial(SharedString::from("V"));
    w.set_account_avatar(Image::default());
    w.set_home_liked(ModelRc::new(VecModel::from(faixas.clone())));
    w.set_home_recent(ModelRc::new(VecModel::from(recentes)));
    w.set_home_playlists(lista(playlists.clone()));
    w.set_home_made_for_you(lista(feitos.clone()));
    w.set_sidebar_playlists(lista(playlists.clone()));
    w.set_library_playlists(lista(playlists));
    w.set_home_feed(ModelRc::new(VecModel::from(vec![ui::FeedShelf {
        title: "Feito para você".into(),
        items: lista(feitos),
    }])));
    w.set_has_track(true);
    w.set_playing(true);
    w.set_now_title("Nights".into());
    w.set_now_artist("Frank Ocean".into());
    w.set_now_id("4".into());
    w.set_now_cover(capa(3));
    w.set_now_cover_pending(false);
    w.set_now_favorite(true);
    w.set_progress(0.42);
    w.set_elapsed("2:11".into());
    w.set_total("5:07".into());
    // Avisos como "Conectado como <usuario>" levariam o nome real para a foto.
    w.set_status_message(SharedString::default());
    // A cena do Travessia segue o relogio; de noite a foto sai quase preta.
    // A hora azul e o momento que mostra as tres camadas de ceu.
    w.global::<ui::Theme>().set_cena_hora(0.55);
}
