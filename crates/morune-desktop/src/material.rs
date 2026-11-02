//! Material da janela: acrilico do Windows atras da tela, quando o tema pede.
//!
//! A janela nasce transparente sempre; quem decide se aparece o que esta atras
//! e o tema. Tema opaco pinta o fundo inteiro e o acrilico fica desligado.

use tauri::utils::config::WindowEffectsConfig;
use tauri::window::Effect as WindowEffect;

/// Liga ou desliga o acrilico. Devolve se ele ficou ligado de fato.
///
/// Com "Efeitos de transparencia" desligado no Windows o acrilico nao aparece,
/// e a tela precisa saber disso para pintar um fundo solido no lugar.
pub fn aplicar(janela: &tauri::WebviewWindow, acrilico: bool) -> bool {
    let ligar = acrilico && transparencia_do_windows();
    let efeitos = ligar.then(|| WindowEffectsConfig {
        effects: vec![WindowEffect::Acrylic],
        ..Default::default()
    });
    if let Err(error) = janela.set_effects(efeitos) {
        tracing::warn!(%error, "acrilico da janela indisponivel");
        return false;
    }
    ligar
}

/// "Efeitos de transparencia" em Configuracoes > Personalizacao > Cores.
#[cfg(windows)]
fn transparencia_do_windows() -> bool {
    use windows::core::w;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};

    let mut valor: u32 = 1;
    let mut tamanho = std::mem::size_of::<u32>() as u32;
    // SAFETY: buffer e tamanho apontam para um u32 vivo durante a chamada.
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
            w!("EnableTransparency"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut valor as *mut u32 as *mut core::ffi::c_void),
            Some(&mut tamanho),
        )
    };
    // Sem a chave, o padrao do Windows e ligado.
    r.is_err() || valor != 0
}

#[cfg(not(windows))]
fn transparencia_do_windows() -> bool {
    false
}

/// Cantos arredondados do Windows 11 numa janela sem barra de titulo (o
/// menu da bandeja e o mini player). Sem isto elas ficam quadradas; no
/// Windows 10 a chamada falha e nada muda.
#[cfg(windows)]
pub fn cantos_redondos(janela: &tauri::WebviewWindow) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    };

    let Ok(h) = janela.hwnd() else { return };
    let preferencia = DWMWCP_ROUND;
    // SAFETY: HWND vivo da propria janela; o valor e um i32 lido na chamada.
    let _ = unsafe {
        DwmSetWindowAttribute(
            HWND(h.0),
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &preferencia as *const _ as *const core::ffi::c_void,
            std::mem::size_of_val(&preferencia) as u32,
        )
    };
}

#[cfg(not(windows))]
pub fn cantos_redondos(_janela: &tauri::WebviewWindow) {}
