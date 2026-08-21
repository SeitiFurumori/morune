//! Menu proprio da bandeja.
//!
//! O menu nativo do Windows e um `HMENU`: o sistema o desenha, e nao ha onde
//! encaixar cor, tipografia ou forma. Era o unico pedaco do Morune que nao
//! parecia o Morune. Aqui ele vira uma janela Slint comum, com os mesmos
//! componentes da janela principal -- inclusive os botoes de transporte.
//!
//! Trocar o menu do sistema por um proprio traz de volta, na mao, tres coisas
//! que o `HMENU` dava de graca:
//!
//! - **fechar ao clicar fora.** Nao ha evento de perda de foco no Slint, entao
//!   a janela e subclassada e responde a `WM_ACTIVATE`, como em [`crate::taskbar`];
//! - **fechar com Esc**, que o proprio componente trata;
//! - **aparecer onde cabe.** A bandeja fica embaixo a direita na configuracao
//!   padrao, mas nao em todas: a posicao e calculada contra a area util do
//!   monitor onde o icone esta, e o menu cai para baixo do icone quando nao ha
//!   espaco acima.
//!
//! A janela existe apenas enquanto o menu esta aberto. Manter uma segunda
//! janela viva custaria outro contexto OpenGL em repouso, e o orcamento do
//! aplicativo e nao pesar na maquina de quem esta fazendo outra coisa.

use std::sync::mpsc::{self, Receiver, Sender};

use slint::ComponentHandle;

use crate::tray::IconAnchor;
use crate::ui;

/// Distancia entre o menu e o icone, e entre o menu e a borda da tela.
const MARGIN: i32 = 8;

/// Menu da bandeja aberto.
pub struct TrayMenu {
    window: ui::TrayMenuWindow,
    anchor: IconAnchor,
    sender: Sender<()>,
    dismissed: Receiver<()>,
    #[cfg(windows)]
    subclassed: Option<windows::Win32::Foundation::HWND>,
    /// A janela chegou a ser a de primeiro plano.
    ///
    /// Sem esta marca, o menu se fecharia no primeiro giro do laco: entre pedir
    /// o foco e recebe-lo passam alguns milissegundos, e nesse intervalo ele
    /// nao e o primeiro plano.
    #[cfg(windows)]
    had_focus: std::cell::Cell<bool>,
}

impl std::fmt::Debug for TrayMenu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrayMenu").finish_non_exhaustive()
    }
}

impl TrayMenu {
    /// Cria e mostra o menu ancorado no icone da bandeja.
    ///
    /// A janela nasce sem posicao, sem canto arredondado e sem foco: nada disso
    /// pode ser feito aqui. `show` pede a janela ao backend, mas o HWND so
    /// existe depois que o laco de eventos gira -- ate la nao ha o que
    /// posicionar nem o que subclassar. Quem completa e [`Self::attach`].
    pub fn open(anchor: IconAnchor) -> Result<Self, slint::PlatformError> {
        let window = ui::TrayMenuWindow::new()?;
        let (sender, dismissed) = mpsc::channel();

        window.show()?;

        Ok(Self {
            window,
            anchor,
            sender,
            dismissed,
            #[cfg(windows)]
            subclassed: None,
            #[cfg(windows)]
            had_focus: std::cell::Cell::new(false),
        })
    }

    /// Termina de montar o menu assim que a janela do sistema existir.
    ///
    /// Chamada a cada giro do laco enquanto nao der certo. Devolve `true`
    /// quando nao ha mais nada a fazer.
    pub fn attach(&mut self) -> bool {
        #[cfg(windows)]
        {
            if self.subclassed.is_some() {
                return true;
            }
            if native_handle(self.window.window()).is_none() {
                return false;
            }

            place(self.window.window(), self.anchor);

            // Mesmo canto arredondado da janela principal. Sem isto o menu
            // seria a unica superficie do aplicativo com canto vivo, ao lado
            // dos menus do proprio Windows, que sao arredondados.
            crate::ensure_rounded_corners(self.window.window());

            self.subclassed = focus_out_closes(self.window.window(), self.sender.clone());
            self.subclassed.is_some()
        }

        #[cfg(not(windows))]
        {
            place(self.window.window(), self.anchor);
            true
        }
    }

    /// A janela do menu, para preencher o estado e ligar os callbacks.
    pub fn window(&self) -> &ui::TrayMenuWindow {
        &self.window
    }

    /// Houve pedido de fechamento desde a ultima leitura?
    ///
    /// Esc e os itens do menu passam pelos callbacks do componente; aqui chega
    /// o clique fora, por dois caminhos que se cobrem:
    ///
    /// - `WM_ACTIVATE` avisando que a janela deixou de ser a ativa;
    /// - a propria janela deixar de ser a de primeiro plano.
    ///
    /// O segundo existe porque o primeiro depende de a janela ter sido ativada
    /// alguma vez, e o Windows nem sempre concede foco a quem pede. Enquanto o
    /// menu nunca tiver recebido o foco, perder o primeiro plano nao significa
    /// nada -- por isso a espera pelo primeiro `true` de `had_focus`.
    pub fn dismiss_requested(&self) -> bool {
        self.dismissed.try_iter().count() > 0 || self.lost_foreground()
    }

    /// A janela teve o primeiro plano e o perdeu?
    ///
    /// Enquanto ela nunca o tiver tido, nao ha o que perder: responder `true`
    /// nesse estado fecharia o menu no primeiro giro do laco, antes mesmo de
    /// ele aparecer.
    #[cfg(windows)]
    fn lost_foreground(&self) -> bool {
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

        let Some(hwnd) = self.subclassed else {
            return false;
        };
        if unsafe { GetForegroundWindow() } == hwnd {
            self.had_focus.set(true);
            return false;
        }
        self.had_focus.get()
    }

    #[cfg(not(windows))]
    fn lost_foreground(&self) -> bool {
        false
    }
}

impl Drop for TrayMenu {
    fn drop(&mut self) {
        // Remover o subclass antes de a janela morrer: o procedimento aponta
        // para uma caixa alocada aqui, e o Windows nao a devolveria sozinho.
        #[cfg(windows)]
        if let Some(hwnd) = self.subclassed.take() {
            let _ = unsafe {
                windows::Win32::UI::Shell::RemoveWindowSubclass(hwnd, Some(wnd_proc), SUBCLASS_ID)
            };
        }
        let _ = self.window.hide();
    }
}

/// Coloca o menu contra o icone, dentro da area util do monitor.
fn place(window: &slint::Window, anchor: IconAnchor) {
    let size = window.size();
    let (x, y) = position_in(
        work_area(anchor),
        anchor,
        size.width as i32,
        size.height as i32,
    );
    window.set_position(slint::PhysicalPosition::new(x, y));
}

/// Onde o menu cabe, dado o icone e a area util.
///
/// Separado de [`place`] porque e a unica parte com decisao de verdade -- e a
/// unica que da para exercitar sem um monitor.
fn position_in(area: WorkArea, anchor: IconAnchor, width: i32, height: i32) -> (i32, i32) {
    // Centrado no icone na horizontal, acima dele na vertical: e onde os menus
    // de bandeja do Windows aparecem quando a barra esta embaixo.
    let mut x = anchor.x + anchor.width / 2 - width / 2;
    let mut y = anchor.y - height - MARGIN;

    let max_x = (area.right - width - MARGIN).max(area.left + MARGIN);
    x = x.clamp(area.left + MARGIN, max_x);

    // Barra de tarefas no topo (ou icone colado na borda de cima): o menu vai
    // para baixo do icone em vez de sair da tela.
    if y < area.top + MARGIN {
        y = anchor.y + anchor.height + MARGIN;
    }
    let max_y = (area.bottom - height - MARGIN).max(area.top + MARGIN);
    y = y.clamp(area.top + MARGIN, max_y);

    (x, y)
}

/// Area util (sem a barra de tarefas) do monitor onde o icone esta.
#[derive(Debug, Clone, Copy)]
struct WorkArea {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[cfg(windows)]
fn work_area(anchor: IconAnchor) -> WorkArea {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };

    let point = POINT {
        x: anchor.x,
        y: anchor.y,
    };
    let monitor = unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST) };

    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        WorkArea {
            left: info.rcWork.left,
            top: info.rcWork.top,
            right: info.rcWork.right,
            bottom: info.rcWork.bottom,
        }
    } else {
        // Sem informacao do monitor, ancorar no proprio icone e melhor que
        // recusar a abrir: o menu ainda nasce ao lado dele.
        fallback_area(anchor)
    }
}

#[cfg(not(windows))]
fn work_area(anchor: IconAnchor) -> WorkArea {
    fallback_area(anchor)
}

fn fallback_area(anchor: IconAnchor) -> WorkArea {
    WorkArea {
        left: i32::MIN / 2,
        top: i32::MIN / 2,
        right: anchor.x + anchor.width,
        bottom: anchor.y + anchor.height,
    }
}

// --- fechar ao clicar fora ---

#[cfg(windows)]
const SUBCLASS_ID: usize = 0x4d4f_5255 + 1;

/// A janela do sistema por tras da janela do Slint, se ela ja existir.
///
/// `None` antes do primeiro giro do laco de eventos: `show` pede a janela, e o
/// backend so a cria depois. Foi exatamente isso que fez a primeira versao
/// deste modulo falhar inteira e em silencio -- posicao, canto e fechamento
/// automatico dependiam de um HWND que ainda nao existia.
#[cfg(windows)]
fn native_handle(window: &slint::Window) -> Option<windows::Win32::Foundation::HWND> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::HWND;

    let owner = window.window_handle();
    let handle = owner.window_handle().ok()?;
    let RawWindowHandle::Win32(win32) = handle.as_ref() else {
        return None;
    };
    Some(HWND(win32.hwnd.get() as *mut std::ffi::c_void))
}

/// Faz a janela avisar quando deixa de ser a ativa.
///
/// Trazer para a frente vem antes de proposito: sem ser ativada, a janela nunca
/// e desativada -- e o menu ficaria aberto para sempre.
#[cfg(windows)]
fn focus_out_closes(
    window: &slint::Window,
    sender: Sender<()>,
) -> Option<windows::Win32::Foundation::HWND> {
    use windows::Win32::UI::Shell::SetWindowSubclass;

    let hwnd = native_handle(window)?;

    let boxed = Box::into_raw(Box::new(sender)) as usize;
    let installed =
        unsafe { SetWindowSubclass(hwnd, Some(wnd_proc), SUBCLASS_ID, boxed) }.as_bool();
    if !installed {
        // Recuperar a caixa evita vazar o canal quando o Windows recusa.
        drop(unsafe { Box::from_raw(boxed as *mut Sender<()>) });
        tracing::warn!("menu da bandeja sem fechamento automatico: subclass recusado");
        return None;
    }

    bring_to_front(hwnd);
    Some(hwnd)
}

/// Traz a janela do menu para o primeiro plano, insistindo quando preciso.
///
/// `SetForegroundWindow` sozinho costuma falhar aqui: o Windows so o concede a
/// quem tem a entrada do usuario, e o clique aconteceu na bandeja, que pertence
/// ao Explorer. Ligar a fila de entrada deste thread a do thread em primeiro
/// plano -- o mesmo recurso que o Windows documenta para menus de bandeja --
/// devolve a permissao pelo tempo da chamada.
#[cfg(windows)]
fn bring_to_front(hwnd: windows::Win32::Foundation::HWND) {
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
    };

    let this = unsafe { GetCurrentThreadId() };
    let front = unsafe { GetForegroundWindow() };
    let front_thread = unsafe { GetWindowThreadProcessId(front, None) };

    let attached = front_thread != 0
        && front_thread != this
        && unsafe { AttachThreadInput(front_thread, this, true) }.as_bool();

    let brought = unsafe { SetForegroundWindow(hwnd) }.as_bool();
    let _ = unsafe { SetFocus(Some(hwnd)) };

    if attached {
        let _ = unsafe { AttachThreadInput(front_thread, this, false) };
    }

    if !brought {
        // Nao e fatal: `dismiss_requested` so passa a vigiar o primeiro plano
        // depois de o menu te-lo tido, entao o menu fica aberto em vez de
        // fechar sozinho. Mas explica um menu que nao fecha ao clicar fora.
        tracing::warn!("menu da bandeja: o Windows recusou o primeiro plano");
    }
}

#[cfg(windows)]
unsafe extern "system" fn wnd_proc(
    hwnd: windows::Win32::Foundation::HWND,
    message: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    _id: usize,
    data: usize,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::Shell::DefSubclassProc;
    use windows::Win32::UI::WindowsAndMessaging::{WA_INACTIVE, WM_ACTIVATE, WM_NCDESTROY};

    if message == WM_ACTIVATE && (wparam.0 as u32 & 0xffff) == WA_INACTIVE {
        // A caixa foi criada em `focus_out_closes` e so e devolvida em
        // `WM_NCDESTROY`, entao o ponteiro e valido aqui.
        let sender = unsafe { &*(data as *const Sender<()>) };
        let _ = sender.send(());
    }

    // O Windows garante esta mensagem antes de destruir a janela, e e a ultima
    // chance de devolver a caixa do canal.
    if message == WM_NCDESTROY {
        drop(unsafe { Box::from_raw(data as *mut Sender<()>) });
    }

    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchor() -> IconAnchor {
        IconAnchor {
            x: 1800,
            y: 1040,
            width: 24,
            height: 24,
        }
    }

    #[test]
    fn menu_fits_above_the_icon() {
        let area = WorkArea {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        };
        let (x, y) = position_in(area, anchor(), 272, 214);

        assert!(y + 214 <= anchor().y, "menu deveria ficar acima do icone");
        assert!(x >= 0 && x + 272 <= 1920, "menu saiu da tela: x = {x}");
    }

    #[test]
    fn menu_falls_below_when_the_bar_is_on_top() {
        let area = WorkArea {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        };
        let top_anchor = IconAnchor {
            x: 1800,
            y: 4,
            width: 24,
            height: 24,
        };
        let (_, y) = position_in(area, top_anchor, 272, 214);

        assert!(
            y >= top_anchor.y + top_anchor.height,
            "menu deveria cair abaixo do icone, e nao sair pelo topo"
        );
    }

    #[test]
    fn menu_stays_inside_a_narrow_screen() {
        let area = WorkArea {
            left: 0,
            top: 0,
            right: 320,
            bottom: 480,
        };
        let narrow = IconAnchor {
            x: 300,
            y: 460,
            width: 20,
            height: 20,
        };
        let (x, y) = position_in(area, narrow, 272, 214);

        assert!(x >= area.left, "menu vazou pela esquerda: x = {x}");
        assert!(y >= area.top, "menu vazou pelo topo: y = {y}");
        assert!(y + 214 <= area.bottom, "menu vazou por baixo: y = {y}");
    }
}
