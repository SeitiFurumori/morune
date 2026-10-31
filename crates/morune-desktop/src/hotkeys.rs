//! Atalhos globais: funcionam com qualquer janela na frente, inclusive um jogo.
//!
//! **Por que existe.** As teclas de midia do teclado ja chegavam (ver
//! [`crate::smtc`]), mas muito teclado nao tem tecla de midia, e o cliente
//! oficial do Spotify nunca teve atalho global -- e o pedido mais antigo do
//! forum deles. Quem esta num jogo quer pular a faixa sem alt-tab.
//!
//! **Como.** `RegisterHotKey` numa thread propria, com fila de mensagens
//! propria. A thread passa a vida dentro de `GetMessageW`, dormindo: nao ha
//! temporizador nem varredura de teclado, e o custo com ninguem apertando nada
//! e zero -- o criterio do projeto. Quem le os comandos e o temporizador que ja
//! existe para a barra de tarefas e o painel de midia.
//!
//! **As combinacoes sao Ctrl+Alt+tecla.** Jogo quase nunca usa Ctrl+Alt, e
//! `RegisterHotKey` **toma** a combinacao do sistema inteiro enquanto o Morune
//! esta aberto -- por isso nada de letra solta ou F-tecla. Se outro programa ja
//! registrou uma delas, o Windows recusa so aquela, e o resto continua valendo.

use std::sync::mpsc::{self, Receiver};
use std::thread;

/// O que um atalho global pediu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyCommand {
    TogglePlay,
    Next,
    Previous,
    VolumeUp,
    VolumeDown,
    ToggleLike,
}

/// Combinacoes registradas, na ordem dos ids.
///
/// Espelhadas no painel de atalhos (`Ctrl+/`) em `app.slint`: mudar aqui e mudar
/// la.
#[cfg(windows)]
const ATALHOS: &[(u16, HotkeyCommand, &str)] = {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        VK_DOWN, VK_LEFT, VK_RIGHT, VK_SPACE, VK_UP,
    };
    &[
        (VK_SPACE.0, HotkeyCommand::TogglePlay, "Ctrl+Alt+Espaço"),
        (VK_RIGHT.0, HotkeyCommand::Next, "Ctrl+Alt+→"),
        (VK_LEFT.0, HotkeyCommand::Previous, "Ctrl+Alt+←"),
        (VK_UP.0, HotkeyCommand::VolumeUp, "Ctrl+Alt+↑"),
        (VK_DOWN.0, HotkeyCommand::VolumeDown, "Ctrl+Alt+↓"),
        (b'L' as u16, HotkeyCommand::ToggleLike, "Ctrl+Alt+L"),
    ]
};

/// Reserva do tocar/pausar, tentada so se Ctrl+Alt+Espaco estiver ocupado.
///
/// Espaco e o melhor para uma mao so (a esquerda alcanca Ctrl+Alt+Espaco sem
/// sair do WASD), mas o app do Claude no Windows usa a mesma combinacao para a
/// pergunta rapida, e o Windows entrega a combinacao a quem pediu primeiro. Z
/// fica na mesma mao e quase nenhum jogo usa com Ctrl+Alt.
#[cfg(windows)]
const RESERVA_TOCAR: (u16, &str) = (b'Z' as u16, "Ctrl+Alt+Z");

/// Os atalhos vivos. Soltar isto encerra a thread e devolve as combinacoes ao
/// sistema.
pub struct GlobalHotkeys {
    comandos: Receiver<HotkeyCommand>,
    #[cfg(windows)]
    thread_id: u32,
}

impl GlobalHotkeys {
    /// Registra o que der. Devolve `None` so se a thread nem subir.
    #[cfg(windows)]
    pub fn start() -> Option<Self> {
        let (tx, comandos) = mpsc::channel();
        let (pronto_tx, pronto_rx) = mpsc::channel();

        thread::Builder::new()
            .name("morune-atalhos".into())
            .spawn(move || laco(tx, pronto_tx))
            .ok()?;

        let thread_id = pronto_rx.recv().ok()?;
        Some(Self {
            comandos,
            thread_id,
        })
    }

    #[cfg(not(windows))]
    pub fn start() -> Option<Self> {
        None
    }

    /// Comandos pendentes, sem bloquear.
    pub fn poll(&self) -> Vec<HotkeyCommand> {
        self.comandos.try_iter().collect()
    }
}

#[cfg(windows)]
impl Drop for GlobalHotkeys {
    fn drop(&mut self) {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};
        // SAFETY: id de thread devolvido pela propria thread; WM_QUIT sem
        // ponteiros.
        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
}

#[cfg(windows)]
fn comando_de(indice: usize) -> &'static HotkeyCommand {
    &ATALHOS[indice].1
}

#[cfg(windows)]
fn laco(tx: mpsc::Sender<HotkeyCommand>, pronto: mpsc::Sender<u32>) {
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

    // SAFETY: chamadas Win32 sem ponteiros alem de `msg`, local e vivo.
    unsafe {
        let _ = pronto.send(GetCurrentThreadId());

        // Sem repeticao: segurar a tecla nao pula dez faixas.
        let modificadores = HOT_KEY_MODIFIERS(MOD_CONTROL.0 | MOD_ALT.0 | MOD_NOREPEAT.0);
        let mut registrados = Vec::new();
        for (i, (tecla, _, nome)) in ATALHOS.iter().enumerate() {
            let id = i as i32 + 1;
            match RegisterHotKey(None, id, modificadores, *tecla as u32) {
                Ok(()) => registrados.push(id),
                Err(e) => {
                    tracing::warn!(atalho = nome, error = %e, "atalho global ocupado por outro programa");
                    // Mesmo id, outra tecla: o WM_HOTKEY continua caindo no
                    // mesmo comando.
                    if *comando_de(i) == HotkeyCommand::TogglePlay {
                        let (reserva, nome_reserva) = RESERVA_TOCAR;
                        match RegisterHotKey(None, id, modificadores, reserva as u32) {
                            Ok(()) => {
                                registrados.push(id);
                                tracing::info!(atalho = nome_reserva, "tocar/pausar na reserva");
                            }
                            Err(e) => {
                                tracing::warn!(atalho = nome_reserva, error = %e, "reserva tambem ocupada")
                            }
                        }
                    }
                }
            }
        }
        tracing::info!(
            registrados = registrados.len(),
            total = ATALHOS.len(),
            "atalhos globais"
        );

        let mut msg = MSG::default();
        // `GetMessageW` devolve 0 no WM_QUIT e -1 em erro: os dois encerram.
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            if msg.message == WM_HOTKEY {
                let indice = msg.wParam.0.wrapping_sub(1);
                if let Some((_, comando, _)) = ATALHOS.get(indice) {
                    if tx.send(*comando).is_err() {
                        break;
                    }
                }
            }
        }

        for id in registrados {
            let _ = UnregisterHotKey(None, id);
        }
    }
}
