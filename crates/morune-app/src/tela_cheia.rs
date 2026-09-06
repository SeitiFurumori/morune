//! O vigia de tela cheia.
//!
//! Responde a uma pergunta so: **ha um aplicativo ocupando a tela inteira na
//! frente, e ele nao e o Morune?**
//!
//! Existe por causa da decisao de 21/08/2026 -- movimento e efeito caro sao
//! permitidos, desde que tudo va a zero quando alguem esta em tela cheia. E a
//! regra que o Wallpaper Engine usa, e e o que torna aceitavel gastar placa de
//! video com vidro: enquanto o jogo esta na frente, o gasto nao existe.
//!
//! **O que o aplicativo ja tinha e nao bastava.** Os relogios internos param
//! quando `window().is_visible()` e falso, mas isso cobre janela minimizada e
//! janela na bandeja -- nao cobre janela COBERTA. Para o Slint, uma janela
//! atras de um jogo em tela cheia continua visivel.
//!
//! **Custo.** Duas chamadas ao sistema por leitura, sem alocar e sem varrer
//! janela nenhuma: qual janela esta na frente, e qual o tamanho do monitor dela.
//! O orcamento de desempenho do projeto proibe gasto invisivel, entao a leitura
//! e barata de proposito e quem chama decide de quanto em quanto tempo.

#[cfg(windows)]
pub fn app_em_tela_cheia() -> bool {
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId,
    };

    // SAFETY: todas as chamadas abaixo recebem ponteiros para variaveis locais
    // vivas e HWND devolvido pelo proprio sistema.
    unsafe {
        let janela: HWND = GetForegroundWindow();
        if janela.is_invalid() {
            return false;
        }

        // A propria janela do Morune em tela cheia nao e motivo para desligar
        // nada: quem maximizou o player quer ver o player.
        let mut pid = 0u32;
        GetWindowThreadProcessId(janela, Some(&mut pid));
        if pid == GetCurrentProcessId() {
            return false;
        }

        // A area de trabalho responde por toda a tela quando nao ha nada em
        // primeiro plano -- sem esta excecao, o Morune se acharia coberto por
        // um jogo o tempo todo em que ninguem clicou em nada.
        //
        // `Progman` e `WorkerW` sao as duas janelas do shell que hospedam os
        // icones e o papel de parede; `Shell_TrayWnd` e a barra de tarefas.
        let mut classe = [0u16; 64];
        let n = GetClassNameW(janela, &mut classe);
        if n > 0 {
            let nome = String::from_utf16_lossy(&classe[..n as usize]);
            if matches!(nome.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd") {
                return false;
            }
        }

        let mut janela_rect = RECT::default();
        if GetWindowRect(janela, &mut janela_rect).is_err() {
            return false;
        }

        let monitor = MonitorFromWindow(janela, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: core::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return false;
        }

        // `rcMonitor` e a tela inteira, incluindo a area sob a barra de
        // tarefas -- que e justamente o que um jogo em tela cheia ocupa. Usar
        // `rcWork` (que desconta a barra) faria uma janela apenas maximizada
        // contar como tela cheia, e maximizar o navegador nao e motivo para
        // desligar o visual do player.
        let tela = info.rcMonitor;
        janela_rect.left <= tela.left
            && janela_rect.top <= tela.top
            && janela_rect.right >= tela.right
            && janela_rect.bottom >= tela.bottom
    }
}

#[cfg(not(windows))]
pub fn app_em_tela_cheia() -> bool {
    false
}
