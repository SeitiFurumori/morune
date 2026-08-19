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
