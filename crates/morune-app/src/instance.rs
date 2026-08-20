//! Garante uma unica instancia do aplicativo no Windows.
//!
//! Uma segunda abertura traz a janela existente para frente em vez de criar
//! outro player, outra bandeja e outro escritor do arquivo de configuracao.

use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SW_RESTORE, SetForegroundWindow, ShowWindow,
};
use windows::core::w;

pub struct SingleInstance(HANDLE);

impl SingleInstance {
    pub fn acquire() -> windows::core::Result<Option<Self>> {
        // SAFETY: nome constante, atributos de seguranca padrao e posse inicial
        // desnecessaria. O HANDLE valido fica vivo no guard retornado.
        let handle = unsafe { CreateMutexW(None, false, w!("Local\\MoruneDesktopAppInstance"))? };
        // SAFETY: GetLastError nao recebe ponteiros nem depende de invariantes.
        let already_running = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        if !already_running {
            return Ok(Some(Self(handle)));
        }

        // SAFETY: o handle foi devolvido por CreateMutexW e nao sera reutilizado.
        let _ = unsafe { CloseHandle(handle) };
        for _ in 0..10 {
            // SAFETY: procura uma janela de topo pelo titulo constante do Morune.
            let window = unsafe { FindWindowW(None, w!("Morune")) };
            if let Ok(window) = window {
                if !window.is_invalid() {
                    // SAFETY: HWND encontrado pelo sistema; restaurar e focar sao
                    // operacoes idempotentes mesmo se ele ja estiver visivel.
                    unsafe {
                        let _ = ShowWindow(window, SW_RESTORE);
                        let _ = SetForegroundWindow(window);
                    }
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Ok(None)
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        // SAFETY: este guard possui o unico HANDLE e o fecha exatamente uma vez.
        let _ = unsafe { CloseHandle(self.0) };
    }
}
