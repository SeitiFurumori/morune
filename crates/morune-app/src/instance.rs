//! Garante uma unica instancia do aplicativo no Windows.
//!
//! Uma segunda abertura traz a janela existente para frente em vez de criar
//! outro player, outra bandeja e outro escritor do arquivo de configuracao.
//!
//! **O caso que quase nao se ve, e que e o pior:** a instancia que ja existe
//! travou. Ela continua com o mutex na mao e com uma janela que nao responde,
//! entao trazer para frente nao faz nada. A versao anterior desistia em
//! silencio -- sem janela, sem mensagem, sem uma linha no log --, e do lado de
//! fora isso e clicar no atalho e nada acontecer, quantas vezes for. Aqui esse
//! caso e detectado e vira uma pergunta, porque so quem esta na frente da tela
//! sabe se aquele processo esta travado de verdade ou apenas ocupado.

use windows::core::w;
use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, HWND};
use windows::Win32::System::Threading::{
    CreateMutexW, OpenProcess, TerminateProcess, PROCESS_TERMINATE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetWindowThreadProcessId, IsHungAppWindow, MessageBoxW, SetForegroundWindow,
    ShowWindow, IDYES, MB_ICONWARNING, MB_YESNO, SW_RESTORE,
};

pub struct SingleInstance(HANDLE);

impl SingleInstance {
    pub fn acquire() -> windows::core::Result<Option<Self>> {
        if let Some(guard) = Self::try_acquire()? {
            return Ok(Some(guard));
        }

        // Ja ha uma instancia. A janela dela pode levar um instante para
        // existir, quando as duas aberturas quase coincidem.
        let Some(window) = Self::find_window() else {
            // Mutex tomado e nenhuma janela em lugar nenhum: um processo que
            // ficou para tras sem interface. Nao ha o que trazer para frente e
            // nem como oferecer fecha-lo, mas dizer isso e melhor que sumir.
            Self::warn(
                w!("O Morune parece estar aberto, mas nao tem janela para mostrar.\r\n\r\nEncerre o processo morune.exe pelo Gerenciador de Tarefas e abra de novo."),
            );
            return Ok(None);
        };

        // SAFETY: HWND devolvido pelo sistema; restaurar e focar sao
        // idempotentes mesmo com a janela ja visivel.
        if !unsafe { IsHungAppWindow(window) }.as_bool() {
            unsafe {
                let _ = ShowWindow(window, SW_RESTORE);
                let _ = SetForegroundWindow(window);
            }
            return Ok(None);
        }

        // Travada. Encerrar por conta propria mataria a reproducao de um
        // aplicativo que pode estar apenas ocupado, entao a escolha e de quem
        // esta na frente da tela.
        if !Self::ask(
            w!("O Morune ja esta aberto, mas nao esta respondendo.\r\n\r\nDeseja encerra-lo e abrir de novo? A reproducao em andamento sera interrompida."),
        ) {
            return Ok(None);
        }

        if !Self::terminate(window) {
            Self::warn(
                w!("Nao foi possivel encerrar o Morune que esta travado.\r\n\r\nFinalize o processo morune.exe pelo Gerenciador de Tarefas e abra de novo."),
            );
            return Ok(None);
        }

        // O mutex e liberado pelo sistema quando o processo morre, mas nao
        // instantaneamente: sem esta espera a nova instancia acharia o nome
        // ainda tomado e desistiria pelo motivo errado.
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if let Some(guard) = Self::try_acquire()? {
                return Ok(Some(guard));
            }
        }

        Self::warn(
            w!("O Morune travado foi encerrado, mas o sistema ainda o considera aberto.\r\n\r\nAguarde alguns segundos e abra de novo."),
        );
        Ok(None)
    }

    /// Tenta tomar o nome global. `None` quando outra instancia ja o tem.
    fn try_acquire() -> windows::core::Result<Option<Self>> {
        // SAFETY: nome constante, atributos de seguranca padrao e posse inicial
        // desnecessaria. O HANDLE valido fica vivo no guard retornado.
        let handle = unsafe { CreateMutexW(None, false, w!("Local\\MoruneDesktopAppInstance"))? };
        // SAFETY: GetLastError nao recebe ponteiros nem depende de invariantes.
        if unsafe { GetLastError() } != ERROR_ALREADY_EXISTS {
            return Ok(Some(Self(handle)));
        }

        // SAFETY: o handle foi devolvido por CreateMutexW e nao sera reutilizado.
        let _ = unsafe { CloseHandle(handle) };
        Ok(None)
    }

    /// Procura a janela da instancia que ja existe.
    ///
    /// Insiste por meio segundo: quando duas aberturas quase coincidem, a
    /// primeira pode ainda nao ter criado a janela.
    fn find_window() -> Option<HWND> {
        for _ in 0..10 {
            // SAFETY: procura uma janela de topo pelo titulo constante do Morune.
            if let Ok(window) = unsafe { FindWindowW(None, w!("Morune")) } {
                if !window.is_invalid() {
                    return Some(window);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        None
    }

    /// Encerra o processo dono da janela. `false` quando nao foi possivel.
    fn terminate(window: HWND) -> bool {
        let mut pid = 0u32;
        // SAFETY: HWND valido e ponteiro para uma variavel local viva.
        unsafe { GetWindowThreadProcessId(window, Some(&mut pid)) };
        if pid == 0 {
            return false;
        }

        // SAFETY: pid vindo do proprio sistema; o handle e fechado abaixo.
        let Ok(process) = (unsafe { OpenProcess(PROCESS_TERMINATE, false, pid) }) else {
            return false;
        };

        // SAFETY: handle valido, aberto com o direito de encerrar.
        let encerrado = unsafe { TerminateProcess(process, 1) }.is_ok();
        // SAFETY: este e o unico dono do handle e o fecha uma vez so.
        let _ = unsafe { CloseHandle(process) };
        encerrado
    }

    /// Pergunta de sim ou nao. `true` quando a resposta e sim.
    fn ask(texto: windows::core::PCWSTR) -> bool {
        // SAFETY: ponteiros para literais estaticos terminados em nulo, sem
        // janela dona -- a nossa ainda nao existe.
        let resposta = unsafe { MessageBoxW(None, texto, w!("Morune"), MB_YESNO | MB_ICONWARNING) };
        resposta == IDYES
    }

    fn warn(texto: windows::core::PCWSTR) {
        // SAFETY: ver `ask`.
        unsafe { MessageBoxW(None, texto, w!("Morune"), MB_ICONWARNING) };
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        // SAFETY: este guard possui o unico HANDLE e o fecha exatamente uma vez.
        let _ = unsafe { CloseHandle(self.0) };
    }
}
