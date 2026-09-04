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

use std::path::PathBuf;

use windows::core::{w, PWSTR};
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, FALSE, HANDLE, HWND, LPARAM, TRUE,
};
use windows::Win32::System::Threading::{
    CreateMutexW, GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, TerminateProcess,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsHungAppWindow, MessageBoxW,
    SetForegroundWindow, ShowWindow, IDYES, MB_ICONWARNING, MB_YESNO, SW_RESTORE,
};

/// Janela encontrada da outra instancia, junto do processo que a criou.
///
/// O pid anda junto do HWND de proposito: entre achar a janela e encerrar o
/// processo nao pode haver uma segunda consulta ao sistema que possa devolver
/// outro processo.
#[derive(Clone, Copy)]
struct Instancia {
    window: HWND,
    pid: u32,
}

pub struct SingleInstance(HANDLE);

impl SingleInstance {
    pub fn acquire() -> windows::core::Result<Option<Self>> {
        if let Some(guard) = Self::try_acquire()? {
            return Ok(Some(guard));
        }

        // Ja ha uma instancia. A janela dela pode levar um instante para
        // existir, quando as duas aberturas quase coincidem.
        let Some(instancia) = Self::find_window() else {
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
        if !unsafe { IsHungAppWindow(instancia.window) }.as_bool() {
            unsafe {
                let _ = ShowWindow(instancia.window, SW_RESTORE);
                let _ = SetForegroundWindow(instancia.window);
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

        if !Self::terminate(instancia) {
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
    ///
    /// **Titulo nao identifica processo.** A versao anterior usava
    /// `FindWindowW` pelo titulo "Morune" e oferecia encerrar o dono daquela
    /// janela: qualquer programa com uma janela de mesmo titulo virava
    /// candidato a `TerminateProcess`. Aqui o titulo e so o filtro barato, e o
    /// que decide e o executavel do processo dono se chamar `morune.exe`.
    ///
    /// **Por que o nome do arquivo, e nao o caminho inteiro.** Um Morune
    /// instalado e um Morune recem-compilado sao o mesmo aplicativo em pastas
    /// diferentes, e disputam o mesmo mutex de instancia unica. Exigir caminho
    /// identico faria o segundo nao reconhecer o primeiro e cair no aviso de
    /// "aberto sem janela" -- que e um dialogo, ou seja, um aplicativo que trava
    /// esperando um clique. Quem ja provou muita coisa aqui e o mutex: chegar
    /// neste ponto significa que o outro processo tomou um nome que so o Morune
    /// usa.
    fn find_window() -> Option<Instancia> {
        for _ in 0..10 {
            if let Some(encontrada) = enumerate_matching() {
                return Some(encontrada);
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        None
    }

    /// Encerra o processo dono da janela. `false` quando nao foi possivel.
    ///
    /// Reconfere o caminho do executavel com o handle ja aberto, e nao pelo pid:
    /// entre a busca e aqui houve uma pergunta ao usuario, e nesse intervalo o
    /// processo pode ter morrido e o pid ter sido reciclado por outro programa.
    /// O handle prende a identidade; o pid, nao.
    fn terminate(alvo: Instancia) -> bool {
        // SAFETY: pid vindo do proprio sistema; o handle e fechado abaixo.
        let Ok(process) = (unsafe {
            OpenProcess(
                PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION,
                false,
                alvo.pid,
            )
        }) else {
            return false;
        };

        let encerrado = if e_morune(process) {
            // SAFETY: handle valido, aberto com o direito de encerrar.
            unsafe { TerminateProcess(process, 1) }.is_ok()
        } else {
            tracing::warn!(pid = alvo.pid, "pid reciclado por outro programa; nao encerrado");
            false
        };

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

/// O processo aberto e um Morune?
///
/// Pergunta so o nome do arquivo -- ver `find_window` para o porque. Comparado
/// sem distinguir maiusculas, que e como o Windows trata caminho.
fn e_morune(process: HANDLE) -> bool {
    image_path_of(process).is_some_and(|caminho| {
        caminho
            .file_name()
            .is_some_and(|nome| nome.eq_ignore_ascii_case("morune.exe"))
    })
}

/// Caminho do executavel de um processo ja aberto.
fn image_path_of(process: HANDLE) -> Option<PathBuf> {
    let mut buffer = [0u16; 32768];
    let mut tamanho = buffer.len() as u32;
    // SAFETY: handle valido com direito de consulta; o ponteiro e o tamanho
    // descrevem o mesmo buffer vivo nesta funcao.
    unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut tamanho,
        )
    }
    .ok()?;

    let texto = String::from_utf16_lossy(&buffer[..tamanho as usize]);
    if texto.is_empty() {
        return None;
    }
    Some(PathBuf::from(texto))
}

/// Estado que a enumeracao de janelas preenche.
struct Busca {
    pid_proprio: u32,
    achado: Option<Instancia>,
}

/// Percorre as janelas de topo procurando uma que seja **nossa**.
///
/// Substitui `FindWindowW`: aquela devolve a primeira janela com o titulo
/// pedido, seja ela de quem for, e nao ha como pedir a segunda.
fn enumerate_matching() -> Option<Instancia> {
    // SAFETY: GetCurrentProcessId nao recebe ponteiros.
    let pid_proprio = unsafe { GetCurrentProcessId() };
    let mut busca = Busca {
        pid_proprio,
        achado: None,
    };

    // SAFETY: o ponteiro aponta para `busca`, que vive por toda a chamada, e o
    // callback so e invocado de dentro dela. `EnumWindows` devolve erro quando
    // o callback interrompe a varredura, o que aqui e sucesso.
    let _ = unsafe {
        EnumWindows(
            Some(visitar_janela),
            LPARAM(&mut busca as *mut Busca as isize),
        )
    };
    busca.achado
}

/// Callback de `EnumWindows`. `TRUE` continua a varredura, `FALSE` para.
unsafe extern "system" fn visitar_janela(window: HWND, lparam: LPARAM) -> windows::core::BOOL {
    // SAFETY: o ponteiro foi montado em `enumerate_matching` a partir de uma
    // referencia exclusiva viva, e a enumeracao e de thread unica.
    let busca = unsafe { &mut *(lparam.0 as *mut Busca) };

    let mut pid = 0u32;
    // SAFETY: HWND vindo da enumeracao e ponteiro para uma variavel local viva.
    unsafe { GetWindowThreadProcessId(window, Some(&mut pid)) };
    if pid == 0 || pid == busca.pid_proprio {
        return TRUE;
    }

    // Filtro barato antes de abrir um handle por janela: a maquina tem dezenas
    // de janelas de topo e so uma nos interessa.
    let mut titulo = [0u16; 32];
    // SAFETY: HWND valido; o buffer e o limite descrevem a mesma variavel local.
    let escritos = unsafe { GetWindowTextW(window, &mut titulo) };
    if String::from_utf16_lossy(&titulo[..escritos.max(0) as usize]) != "Morune" {
        return TRUE;
    }

    // SAFETY: pid vindo do sistema; o handle e fechado logo abaixo.
    let Ok(process) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }) else {
        return TRUE;
    };
    let morune = e_morune(process);
    // SAFETY: unico dono do handle, fechado uma vez so.
    let _ = unsafe { CloseHandle(process) };

    if morune {
        busca.achado = Some(Instancia { window, pid });
        return FALSE;
    }
    TRUE
}
