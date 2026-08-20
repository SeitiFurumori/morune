//! Controles nativos no preview da barra de tarefas do Windows.
//!
//! O Windows chama esse recurso de *thumbnail toolbar*. O conjunto de botoes e
//! registrado uma vez e depois apenas atualizado; cliques chegam como
//! `WM_COMMAND`. Toda a fronteira insegura Win32 fica confinada neste modulo.

use std::cell::Cell;
use std::ffi::c_void;
use std::sync::mpsc::{self, Receiver, Sender};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RPC_E_CHANGED_MODE, WPARAM};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    CoUninitialize,
};
use windows::Win32::UI::Shell::{
    DefSubclassProc, ITaskbarList3, RemoveWindowSubclass, SetWindowSubclass, THB_FLAGS, THB_ICON,
    THB_TOOLTIP, THBF_DISABLED, THBF_ENABLED, THBN_CLICKED, THUMBBUTTON, TaskbarList,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconFromResourceEx, DestroyIcon, HICON, LR_DEFAULTCOLOR, RegisterWindowMessageW,
    WM_APPCOMMAND, WM_COMMAND,
};
use windows::core::w;

const BUTTON_PREVIOUS: u32 = 0x4d01;
const BUTTON_TOGGLE: u32 = 0x4d02;
const BUTTON_NEXT: u32 = 0x4d03;
const APPCOMMAND_MEDIA_NEXTTRACK: u32 = 11;
const APPCOMMAND_MEDIA_PREVIOUSTRACK: u32 = 12;
const APPCOMMAND_MEDIA_PLAY_PAUSE: u32 = 14;
const SUBCLASS_ID: usize = 0x4d4f_5255;
const ICON_SIZE: usize = 32;
const MASK_BYTES: usize = ICON_SIZE * ICON_SIZE / 8;
const ICON_RESOURCE_HEADER_BYTES: usize = 40;
const ICON_COLOR_BYTES: usize = ICON_SIZE * ICON_SIZE * 4;
const ICON_RESOURCE_BYTES: usize = ICON_RESOURCE_HEADER_BYTES + ICON_COLOR_BYTES + MASK_BYTES;
// Violeta da marca com luminosidade intermediaria: continua legivel tanto no
// flyout claro quanto no escuro do Windows.
const GLYPH_BGRA: [u8; 4] = [0xff, 0x5c, 0xc0, 0xff];

/// Acao pedida pelo usuario no preview da janela.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskbarCommand {
    Previous,
    TogglePlay,
    Next,
}

enum NativeEvent {
    Command(TaskbarCommand),
    TaskbarRecreated,
}

struct SubclassState {
    sender: Sender<NativeEvent>,
    taskbar_button_created: u32,
}

/// Inicializacao COM balanceada na mesma thread da interface.
struct ComApartment(bool);

impl ComApartment {
    fn initialize() -> Result<Self, TaskbarError> {
        // SAFETY: inicializa COM para a thread atual, com ponteiro reservado nulo.
        let result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        if result.is_ok() {
            Ok(Self(true))
        } else if result == RPC_E_CHANGED_MODE {
            // Outro componente ja escolheu o modelo COM desta thread. COM esta
            // disponivel; apenas nao devemos balancear uma inicializacao nossa.
            Ok(Self(false))
        } else {
            Err(TaskbarError::Windows(result.into()))
        }
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.0 {
            // SAFETY: balanceia exatamente o CoInitializeEx bem-sucedido acima.
            unsafe { CoUninitialize() };
        }
    }
}

struct OwnedIcon(HICON);

impl Drop for OwnedIcon {
    fn drop(&mut self) {
        // SAFETY: o HICON foi criado por CreateIconFromResourceEx e pertence a
        // este objeto.
        let _ = unsafe { DestroyIcon(self.0) };
    }
}

struct PlayerIcons {
    previous: OwnedIcon,
    play: OwnedIcon,
    pause: OwnedIcon,
    next: OwnedIcon,
}

impl PlayerIcons {
    fn new() -> Result<Self, TaskbarError> {
        Ok(Self {
            previous: create_glyph_icon(Glyph::Previous)?,
            play: create_glyph_icon(Glyph::Play)?,
            pause: create_glyph_icon(Glyph::Pause)?,
            next: create_glyph_icon(Glyph::Next)?,
        })
    }
}

/// Mantem vivos a interface COM, os icones e o callback enquanto a janela existe.
pub struct TaskbarControls {
    // A ordem e intencional: a interface COM deve cair antes de `_apartment`.
    taskbar: Option<ITaskbarList3>,
    _apartment: ComApartment,
    hwnd: HWND,
    icons: PlayerIcons,
    receiver: Receiver<NativeEvent>,
    subclass_state: *mut SubclassState,
    registered: Cell<bool>,
    last_shown: Cell<Option<(bool, bool)>>,
}

impl TaskbarControls {
    pub fn new(window: &slint::Window) -> Result<Self, TaskbarError> {
        let handle = window.window_handle();
        let raw = handle
            .window_handle()
            .map_err(TaskbarError::WindowHandle)?
            .as_raw();
        let RawWindowHandle::Win32(raw) = raw else {
            return Err(TaskbarError::NotWin32);
        };
        let hwnd = HWND(raw.hwnd.get() as *mut c_void);

        let apartment = ComApartment::initialize()?;
        // SAFETY: COM esta inicializado na thread atual e TaskbarList e uma
        // coclasse do sistema usada sem agregacao.
        let taskbar: ITaskbarList3 =
            unsafe { CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER) }?;
        // SAFETY: chamada de inicializacao obrigatoria da interface recem-criada.
        unsafe { taskbar.HrInit() }?;

        let icons = PlayerIcons::new()?;
        // SAFETY: a string e terminada em NUL pelo macro `w!`.
        let taskbar_button_created = unsafe { RegisterWindowMessageW(w!("TaskbarButtonCreated")) };
        if taskbar_button_created == 0 {
            return Err(TaskbarError::RegisterMessage);
        }

        let (sender, receiver) = mpsc::channel();
        let subclass_state = Box::into_raw(Box::new(SubclassState {
            sender,
            taskbar_button_created,
        }));
        // SAFETY: `subclass_state` permanece alocado ate Drop, quando o callback
        // e removido antes de o Box ser reconstruido.
        let subclassed = unsafe {
            SetWindowSubclass(
                hwnd,
                Some(window_subclass),
                SUBCLASS_ID,
                subclass_state as usize,
            )
        };
        if !subclassed.as_bool() {
            // SAFETY: SetWindowSubclass falhou, portanto nenhum callback reteve
            // o ponteiro e a alocacao ainda pertence exclusivamente a este ramo.
            unsafe { drop(Box::from_raw(subclass_state)) };
            return Err(TaskbarError::Subclass);
        }

        let mut controls = Self {
            taskbar: Some(taskbar),
            _apartment: apartment,
            hwnd,
            icons,
            receiver,
            subclass_state,
            registered: Cell::new(false),
            last_shown: Cell::new(None),
        };
        controls.update(false, false);
        Ok(controls)
    }

    /// Le cliques sem bloquear a thread da interface.
    pub fn poll(&mut self) -> Vec<TaskbarCommand> {
        let mut commands = Vec::new();
        while let Ok(event) = self.receiver.try_recv() {
            match event {
                NativeEvent::Command(command) => commands.push(command),
                NativeEvent::TaskbarRecreated => {
                    self.registered.set(false);
                    self.last_shown.set(None);
                }
            }
        }
        commands
    }

    /// Sincroniza habilitacao e o icone central sem repetir chamadas Win32.
    pub fn update(&mut self, has_track: bool, playing: bool) {
        let shown = (has_track, playing);
        if self.registered.get() && self.last_shown.get() == Some(shown) {
            return;
        }

        let buttons = self.buttons(has_track, playing);
        let Some(taskbar) = self.taskbar.as_ref() else {
            return;
        };
        // SAFETY: HWND, interface e HICONs continuam validos durante a chamada.
        let result = unsafe {
            if self.registered.get() {
                taskbar.ThumbBarUpdateButtons(self.hwnd, &buttons)
            } else {
                taskbar.ThumbBarAddButtons(self.hwnd, &buttons)
            }
        };

        match result {
            Ok(()) => {
                self.registered.set(true);
                self.last_shown.set(Some(shown));
            }
            Err(error) => tracing::debug!(%error, "thumbnail toolbar ainda nao disponivel"),
        }
    }

    fn buttons(&self, has_track: bool, playing: bool) -> [THUMBBUTTON; 3] {
        let flags = if has_track {
            THBF_ENABLED
        } else {
            THBF_DISABLED
        };
        [
            button(
                BUTTON_PREVIOUS,
                self.icons.previous.0,
                "Faixa anterior",
                flags,
            ),
            button(
                BUTTON_TOGGLE,
                if playing {
                    self.icons.pause.0
                } else {
                    self.icons.play.0
                },
                if playing { "Pausar" } else { "Tocar" },
                flags,
            ),
            button(BUTTON_NEXT, self.icons.next.0, "Proxima faixa", flags),
        ]
    }
}

impl Drop for TaskbarControls {
    fn drop(&mut self) {
        // SAFETY: a janela ainda pertence ao componente Slint, e removemos o
        // mesmo callback/id que foram instalados em `new`.
        let _ = unsafe { RemoveWindowSubclass(self.hwnd, Some(window_subclass), SUBCLASS_ID) };
        if !self.subclass_state.is_null() {
            // SAFETY: o callback ja foi removido; este e o ponteiro unico criado
            // por Box::into_raw em `new`.
            unsafe { drop(Box::from_raw(self.subclass_state)) };
            self.subclass_state = std::ptr::null_mut();
        }
        // Libera a interface antes de ComApartment::drop chamar CoUninitialize.
        drop(self.taskbar.take());
    }
}

unsafe extern "system" fn window_subclass(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _subclass_id: usize,
    ref_data: usize,
) -> LRESULT {
    // SAFETY: ref_data aponta para SubclassState durante toda a instalacao do
    // callback; Drop remove o callback antes de liberar a alocacao.
    let state = unsafe { &*(ref_data as *const SubclassState) };

    if message == WM_APPCOMMAND {
        // GET_APPCOMMAND_LPARAM: os 11 bits baixos da palavra alta guardam o
        // comando, independentemente de ele vir do teclado ou de outro HID.
        let app_command = ((lparam.0 >> 16) & 0x7ff) as u32;
        let command = match app_command {
            APPCOMMAND_MEDIA_PREVIOUSTRACK => Some(TaskbarCommand::Previous),
            APPCOMMAND_MEDIA_PLAY_PAUSE => Some(TaskbarCommand::TogglePlay),
            APPCOMMAND_MEDIA_NEXTTRACK => Some(TaskbarCommand::Next),
            _ => None,
        };
        if let Some(command) = command {
            let _ = state.sender.send(NativeEvent::Command(command));
            return LRESULT(1);
        }
    } else if message == WM_COMMAND {
        let notification = ((wparam.0 >> 16) & 0xffff) as u32;
        let button_id = (wparam.0 & 0xffff) as u32;
        if notification == THBN_CLICKED {
            let command = match button_id {
                BUTTON_PREVIOUS => Some(TaskbarCommand::Previous),
                BUTTON_TOGGLE => Some(TaskbarCommand::TogglePlay),
                BUTTON_NEXT => Some(TaskbarCommand::Next),
                _ => None,
            };
            if let Some(command) = command {
                let _ = state.sender.send(NativeEvent::Command(command));
            }
        }
    } else if message == state.taskbar_button_created {
        // O Explorer pode reiniciar; nesse caso o conjunto inteiro precisa ser
        // adicionado novamente, com os mesmos ids.
        let _ = state.sender.send(NativeEvent::TaskbarRecreated);
    }

    // SAFETY: encaminha toda mensagem para a proxima funcao da cadeia Win32.
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

fn button(
    id: u32,
    icon: HICON,
    tooltip: &str,
    flags: windows::Win32::UI::Shell::THUMBBUTTONFLAGS,
) -> THUMBBUTTON {
    let mut tip = [0u16; 260];
    for (target, source) in tip.iter_mut().zip(tooltip.encode_utf16()) {
        *target = source;
    }
    THUMBBUTTON {
        dwMask: THB_ICON | THB_TOOLTIP | THB_FLAGS,
        iId: id,
        hIcon: icon,
        szTip: tip,
        dwFlags: flags,
        ..Default::default()
    }
}

#[derive(Clone, Copy)]
enum Glyph {
    Previous,
    Play,
    Pause,
    Next,
}

fn create_glyph_icon(glyph: Glyph) -> Result<OwnedIcon, TaskbarError> {
    let resource = glyph_icon_resource(glyph);
    // SAFETY: o buffer contem um BITMAPINFOHEADER, pixels BGRA 32-bit e mascara
    // AND, exatamente no formato RT_ICON. A API copia os dados antes de voltar.
    let icon = unsafe {
        CreateIconFromResourceEx(
            &resource,
            true,
            0x0003_0000,
            ICON_SIZE as i32,
            ICON_SIZE as i32,
            LR_DEFAULTCOLOR,
        )
    }?;
    Ok(OwnedIcon(icon))
}

fn glyph_icon_resource(glyph: Glyph) -> [u8; ICON_RESOURCE_BYTES] {
    let mut alpha = [0u8; ICON_SIZE * ICON_SIZE];

    let mut pixel = |x: usize, y: usize| {
        if x < ICON_SIZE && y < ICON_SIZE {
            alpha[y * ICON_SIZE + x] = 0xff;
        }
    };

    match glyph {
        Glyph::Play => triangle_right(&mut pixel, 11, 8, 20),
        Glyph::Pause => {
            rect(&mut pixel, 10, 8, 14, 24);
            rect(&mut pixel, 18, 8, 22, 24);
        }
        Glyph::Previous => {
            rect(&mut pixel, 8, 9, 11, 23);
            triangle_left(&mut pixel, 11, 8, 22);
        }
        Glyph::Next => {
            triangle_right(&mut pixel, 9, 8, 20);
            rect(&mut pixel, 21, 9, 24, 23);
        }
    }

    let mut resource = [0u8; ICON_RESOURCE_BYTES];
    // BITMAPINFOHEADER. A altura e dobrada porque um RT_ICON guarda o bitmap
    // de cor e, logo depois, sua mascara AND.
    write_u32(&mut resource, 0, ICON_RESOURCE_HEADER_BYTES as u32);
    write_i32(&mut resource, 4, ICON_SIZE as i32);
    write_i32(&mut resource, 8, (ICON_SIZE * 2) as i32);
    write_u16(&mut resource, 12, 1);
    write_u16(&mut resource, 14, 32);
    write_u32(&mut resource, 20, ICON_COLOR_BYTES as u32);

    let color_start = ICON_RESOURCE_HEADER_BYTES;
    let mask_start = color_start + ICON_COLOR_BYTES;
    resource[mask_start..].fill(0xff);

    // DIBs sao armazenados de baixo para cima. O canal alpha explicito corrige
    // o HICON monocromatico que virava um botao clicavel, porem invisivel, no
    // compositor da thumbnail toolbar.
    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            if alpha[y * ICON_SIZE + x] == 0 {
                continue;
            }
            let dib_y = ICON_SIZE - 1 - y;
            let color = color_start + (dib_y * ICON_SIZE + x) * 4;
            resource[color..color + 4].copy_from_slice(&GLYPH_BGRA);
            resource[mask_start + dib_y * 4 + x / 8] &= !(0x80 >> (x % 8));
        }
    }

    resource
}

fn write_u16(target: &mut [u8], offset: usize, value: u16) {
    target[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(target: &mut [u8], offset: usize, value: u32) {
    target[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_i32(target: &mut [u8], offset: usize, value: i32) {
    target[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn rect(
    pixel: &mut impl FnMut(usize, usize),
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
) {
    for y in top..bottom {
        for x in left..right {
            pixel(x, y);
        }
    }
}

fn triangle_right(pixel: &mut impl FnMut(usize, usize), left: usize, top: usize, width: usize) {
    let height = 16usize;
    for y in 0..height {
        let half = if y < height / 2 { y } else { height - 1 - y };
        let row_width = 2 + half * width / (height / 2);
        for x in 0..row_width {
            pixel(left + x, top + y);
        }
    }
}

fn triangle_left(pixel: &mut impl FnMut(usize, usize), right: usize, top: usize, width: usize) {
    let height = 16usize;
    for y in 0..height {
        let half = if y < height / 2 { y } else { height - 1 - y };
        let row_width = 2 + half * width / (height / 2);
        for x in 0..row_width {
            pixel(right.saturating_sub(x), top + y);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TaskbarError {
    #[error("janela nao expoe um handle Win32")]
    NotWin32,
    #[error("handle da janela: {0}")]
    WindowHandle(raw_window_handle::HandleError),
    #[error("Windows: {0}")]
    Windows(#[from] windows::core::Error),
    #[error("Windows nao registrou TaskbarButtonCreated")]
    RegisterMessage,
    #[error("Windows recusou o callback da janela")]
    Subclass,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_glyph_is_a_valid_argb_icon_resource() {
        for glyph in [Glyph::Previous, Glyph::Play, Glyph::Pause, Glyph::Next] {
            let resource = glyph_icon_resource(glyph);
            assert_eq!(u32::from_le_bytes(resource[0..4].try_into().unwrap()), 40);
            assert_eq!(u16::from_le_bytes(resource[14..16].try_into().unwrap()), 32);
            let pixels = &resource[ICON_RESOURCE_HEADER_BYTES..][..ICON_COLOR_BYTES];
            let visible = pixels
                .chunks_exact(4)
                .filter(|pixel| pixel[3] == 0xff)
                .count();
            assert!(visible > 30, "icone quase vazio: {visible} pixels");
            assert!(visible < 400, "icone virou um bloco: {visible} pixels");
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] == 0));
        }
    }

    #[test]
    fn play_and_pause_are_distinct() {
        assert_ne!(
            glyph_icon_resource(Glyph::Play),
            glyph_icon_resource(Glyph::Pause)
        );
    }
}
