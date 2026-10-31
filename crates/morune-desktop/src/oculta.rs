//! Janela nativa invisivel, dona do painel de midia do Windows (SMTC).
//!
//! A tela web e destruida quando o Morune vai para a bandeja, mas as teclas de
//! midia e o painel de volume precisam continuar funcionando. O SMTC pede uma
//! janela; esta nunca aparece e nao desenha nada.

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawWindowHandle, Win32WindowHandle,
    WindowHandle,
};
use std::num::NonZeroIsize;
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, RegisterClassW, WINDOW_EX_STYLE, WNDCLASSW,
    WS_OVERLAPPED,
};

pub struct JanelaOculta {
    hwnd: HWND,
}

unsafe extern "system" fn proc_janela(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: repassa a mensagem ao tratamento padrao do sistema.
    unsafe { DefWindowProcW(h, m, w, l) }
}

impl JanelaOculta {
    pub fn criar() -> Option<Self> {
        // SAFETY: classe e janela registradas com ponteiros estaticos validos; a
        // janela nunca recebe `ShowWindow`, entao continua invisivel.
        unsafe {
            let instancia = GetModuleHandleW(None).ok()?;
            let classe = WNDCLASSW {
                lpfnWndProc: Some(proc_janela),
                hInstance: instancia.into(),
                lpszClassName: w!("MoruneMotor"),
                ..Default::default()
            };
            RegisterClassW(&classe);
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("MoruneMotor"),
                w!("Morune"),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(instancia.into()),
                None,
            )
            .ok()?;
            Some(Self { hwnd })
        }
    }
}

impl Drop for JanelaOculta {
    fn drop(&mut self) {
        // SAFETY: janela criada por nos e ainda valida.
        let _ = unsafe { DestroyWindow(self.hwnd) };
    }
}

impl HasWindowHandle for JanelaOculta {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let raw = NonZeroIsize::new(self.hwnd.0 as isize).ok_or(HandleError::Unavailable)?;
        let handle = Win32WindowHandle::new(raw);
        // SAFETY: o HWND vive enquanto `self` viver.
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(handle)) })
    }
}

impl HasDisplayHandle for JanelaOculta {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::windows())
    }
}
