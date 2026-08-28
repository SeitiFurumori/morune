//! Controles nativos no preview da barra de tarefas do Windows.
//!
//! O Windows chama esse recurso de *thumbnail toolbar*. O conjunto de botoes e
//! registrado uma vez e depois apenas atualizado; cliques chegam como
//! `WM_COMMAND`. Toda a fronteira insegura Win32 fica confinada neste modulo.

use std::cell::Cell;
use std::ffi::c_void;
use std::sync::mpsc::{self, Receiver, Sender};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RPC_E_CHANGED_MODE, WPARAM};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{
    DefSubclassProc, ITaskbarList3, RemoveWindowSubclass, SetWindowSubclass, TaskbarList,
    THBF_DISABLED, THBF_ENABLED, THBN_CLICKED, THB_FLAGS, THB_ICON, THB_TOOLTIP, THUMBBUTTON,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconFromResourceEx, DestroyIcon, RegisterWindowMessageW, HICON, LR_DEFAULTCOLOR,
    WM_APPCOMMAND, WM_COMMAND,
};

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
/// Amostras por eixo na suavizacao dos glifos.
///
/// 4x4 = 16 amostras por pixel. O olho nao distingue mais que isso num icone de
/// 32 px, e o custo e irrelevante: os quatro icones sao desenhados uma vez por
/// troca de tema.
const SUPERSAMPLE: usize = 4;

/// Margem do desenho dentro do icone, em pixels.
///
/// O Windows encolhe o icone para caber no botao da barra de miniaturas. Sem
/// margem, a suavizacao da borda e a primeira coisa que ele corta.
const GLYPH_MARGIN: f32 = 5.0;

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
    fn new(tint: [u8; 3]) -> Result<Self, TaskbarError> {
        Ok(Self {
            previous: create_glyph_icon(Glyph::Previous, tint)?,
            play: create_glyph_icon(Glyph::Play, tint)?,
            pause: create_glyph_icon(Glyph::Pause, tint)?,
            next: create_glyph_icon(Glyph::Next, tint)?,
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
    /// Cor com que os icones foram desenhados, para saber quando redesenhar.
    tint: Cell<[u8; 3]>,
    receiver: Receiver<NativeEvent>,
    subclass_state: *mut SubclassState,
    registered: Cell<bool>,
    last_shown: Cell<Option<(bool, bool)>>,
}

impl TaskbarControls {
    pub fn new(window: &slint::Window, tint: [u8; 3]) -> Result<Self, TaskbarError> {
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

        let icons = PlayerIcons::new(tint)?;
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
            tint: Cell::new(tint),
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

    /// Redesenha os icones quando o tema muda de cor.
    ///
    /// Sai no primeiro `if` no caso comum -- e chamada no mesmo tique de meio
    /// segundo da bandeja, e a cor so muda quando alguem troca de tema.
    ///
    /// Falhar em criar os icones novos **mantem os antigos**: um botao com a
    /// cor do tema anterior e melhor que um botao invisivel.
    pub fn set_tint(&mut self, tint: [u8; 3]) {
        if self.tint.get() == tint {
            return;
        }

        match PlayerIcons::new(tint) {
            Ok(icons) => {
                self.icons = icons;
                self.tint.set(tint);
                // Forca o proximo `update` a reenviar os botoes: os HICON
                // antigos foram destruidos junto com o `PlayerIcons` anterior,
                // e a barra ainda aponta para eles.
                self.last_shown.set(None);
            }
            Err(error) => tracing::debug!(%error, "icones da barra de tarefas nao redesenharam"),
        }
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

#[derive(Debug, Clone, Copy)]
enum Glyph {
    Previous,
    Play,
    Pause,
    Next,
}

fn create_glyph_icon(glyph: Glyph, tint: [u8; 3]) -> Result<OwnedIcon, TaskbarError> {
    let resource = glyph_icon_resource(glyph, tint);
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

/// Formas que compoem um glifo, em coordenadas de ponto flutuante.
///
/// Coordenadas reais, e nao inteiros de pixel: e o que permite centralizar e
/// suavizar. A versao anterior desenhava com aritmetica inteira e limites
/// escritos a mao -- o triangulo de tocar comecava em x=11 e terminava em x=33
/// num icone de 32, entao saia cortado a direita **e** deslocado.
#[derive(Debug, Clone, Copy)]
enum Shape {
    /// Retangulo: esquerda, topo, direita, base.
    Rect(f32, f32, f32, f32),
    /// Triangulo por tres vertices.
    Tri([(f32, f32); 3]),
}

impl Shape {
    fn contains(&self, x: f32, y: f32) -> bool {
        match self {
            Shape::Rect(l, t, r, b) => x >= *l && x < *r && y >= *t && y < *b,
            Shape::Tri(v) => {
                // Sinal do produto vetorial em relacao a cada aresta. Igual nos
                // tres lados significa dentro, e funciona para qualquer ordem
                // de vertices.
                let lado = |a: (f32, f32), b: (f32, f32)| {
                    (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0)
                };
                let d0 = lado(v[0], v[1]);
                let d1 = lado(v[1], v[2]);
                let d2 = lado(v[2], v[0]);
                let neg = d0 < 0.0 || d1 < 0.0 || d2 < 0.0;
                let pos = d0 > 0.0 || d1 > 0.0 || d2 > 0.0;
                !(neg && pos)
            }
        }
    }
}

/// As formas de cada glifo, ja centradas na area util do icone.
///
/// Todas ocupam a mesma caixa vertical e a mesma largura total, para que os
/// tres botoes tenham peso visual igual quando ficam lado a lado.
fn glyph_shapes(glyph: Glyph) -> Vec<Shape> {
    let lado = ICON_SIZE as f32;
    let esq = GLYPH_MARGIN;
    let dir = lado - GLYPH_MARGIN;
    let topo = GLYPH_MARGIN;
    let base = lado - GLYPH_MARGIN;
    let meio = lado / 2.0;
    // Espessura da barra vertical de "anterior" e "proxima".
    let barra = 3.5;

    match glyph {
        // O triangulo ocupa a largura toda: sozinho no botao, ele e o unico
        // elemento e nao divide espaco com barra nenhuma.
        Glyph::Play => vec![Shape::Tri([(esq, topo), (esq, base), (dir, meio)])],
        Glyph::Pause => {
            let largura = 4.5;
            let vao = 4.0;
            vec![
                Shape::Rect(meio - vao / 2.0 - largura, topo, meio - vao / 2.0, base),
                Shape::Rect(meio + vao / 2.0, topo, meio + vao / 2.0 + largura, base),
            ]
        }
        Glyph::Previous => vec![
            Shape::Rect(esq, topo, esq + barra, base),
            Shape::Tri([(dir, topo), (dir, base), (esq + barra + 1.0, meio)]),
        ],
        Glyph::Next => vec![
            Shape::Tri([(esq, topo), (esq, base), (dir - barra - 1.0, meio)]),
            Shape::Rect(dir - barra, topo, dir, base),
        ],
    }
}

/// Cobertura de cada pixel do glifo, de 0 a 255.
///
/// Amostragem em grade: a fracao de sub-amostras dentro de alguma forma vira o
/// alfa. E o que troca a escada de pixels da versao anterior por uma borda
/// lisa, que e o que o usuario ve como "feio" ou "limpo".
fn glyph_coverage(glyph: Glyph) -> [u8; ICON_SIZE * ICON_SIZE] {
    let shapes = glyph_shapes(glyph);
    let mut alpha = [0u8; ICON_SIZE * ICON_SIZE];
    let passo = 1.0 / SUPERSAMPLE as f32;

    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let mut dentro = 0u32;
            for sy in 0..SUPERSAMPLE {
                for sx in 0..SUPERSAMPLE {
                    let px = x as f32 + (sx as f32 + 0.5) * passo;
                    let py = y as f32 + (sy as f32 + 0.5) * passo;
                    if shapes.iter().any(|s| s.contains(px, py)) {
                        dentro += 1;
                    }
                }
            }
            let total = (SUPERSAMPLE * SUPERSAMPLE) as u32;
            alpha[y * ICON_SIZE + x] = (dentro * 255 / total) as u8;
        }
    }

    alpha
}

/// Monta o recurso `RT_ICON` de um glifo, na cor pedida.
///
/// `tint` e RGB do tema. Antes era uma constante violeta: o icone era o unico
/// pedaco do produto que nao obedecia ao tema escolhido.
fn glyph_icon_resource(glyph: Glyph, tint: [u8; 3]) -> [u8; ICON_RESOURCE_BYTES] {
    let alpha = glyph_coverage(glyph);

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

    // DIBs sao armazenados de baixo para cima. O canal alfa explicito corrige
    // o HICON monocromatico que virava um botao clicavel, porem invisivel, no
    // compositor da thumbnail toolbar.
    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let cobertura = alpha[y * ICON_SIZE + x];
            if cobertura == 0 {
                continue;
            }
            let dib_y = ICON_SIZE - 1 - y;
            let color = color_start + (dib_y * ICON_SIZE + x) * 4;
            // BGRA, com o alfa da cobertura: o pixel de borda entra parcial.
            resource[color] = tint[2];
            resource[color + 1] = tint[1];
            resource[color + 2] = tint[0];
            resource[color + 3] = cobertura;
            // Qualquer cobertura torna o pixel visivel na mascara; a
            // transparencia parcial quem resolve e o alfa acima.
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

    /// Cor qualquer, so para o recurso ficar completo nos testes.
    const TINTA: [u8; 3] = [0x6d, 0xd4, 0x9e];

    /// Extremos ocupados pelo desenho, em pixels: (esquerda, direita).
    ///
    /// Um pixel conta quando tem alguma cobertura; a borda suavizada entra.
    fn extremos(glyph: Glyph) -> (usize, usize) {
        let alpha = glyph_coverage(glyph);
        let mut esq = ICON_SIZE;
        let mut dir = 0;
        for y in 0..ICON_SIZE {
            for x in 0..ICON_SIZE {
                if alpha[y * ICON_SIZE + x] > 0 {
                    esq = esq.min(x);
                    dir = dir.max(x);
                }
            }
        }
        (esq, dir)
    }

    #[test]
    fn every_glyph_is_a_valid_argb_icon_resource() {
        for glyph in [Glyph::Previous, Glyph::Play, Glyph::Pause, Glyph::Next] {
            let resource = glyph_icon_resource(glyph, TINTA);
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
            glyph_icon_resource(Glyph::Play, TINTA),
            glyph_icon_resource(Glyph::Pause, TINTA)
        );
    }

    /// O defeito relatado: o triangulo de tocar saia deslocado e cortado na
    /// borda direita, porque era desenhado de x=11 ate x=33 num icone de 32.
    #[test]
    fn nenhum_glifo_encosta_na_borda() {
        for glyph in [Glyph::Previous, Glyph::Play, Glyph::Pause, Glyph::Next] {
            let (esq, dir) = extremos(glyph);
            assert!(esq > 0, "{glyph:?} encosta na borda esquerda");
            assert!(
                dir < ICON_SIZE - 1,
                "{glyph:?} encosta na borda direita (x={dir})"
            );
        }
    }

    #[test]
    fn todo_glifo_fica_centrado() {
        for glyph in [Glyph::Previous, Glyph::Play, Glyph::Pause, Glyph::Next] {
            let (esq, dir) = extremos(glyph);
            let folga_esquerda = esq;
            let folga_direita = ICON_SIZE - 1 - dir;
            let diferenca = folga_esquerda.abs_diff(folga_direita);
            assert!(
                diferenca <= 1,
                "{glyph:?} descentrado: {folga_esquerda} a esquerda, {folga_direita} a direita"
            );
        }
    }

    /// Sem suavizacao todo pixel seria 0 ou 255, e a borda vira escada.
    #[test]
    fn as_bordas_sao_suavizadas() {
        let alpha = glyph_coverage(Glyph::Play);
        let parciais = alpha.iter().filter(|&&a| a > 0 && a < 255).count();
        assert!(parciais > 10, "borda sem meio-tom: {parciais} pixels");
    }

    /// O pause e feito de retangulos alinhados ao pixel: nao ha o que suavizar,
    /// e um meio-tom ali seria borrao, nao curva.
    #[test]
    fn o_pause_nao_precisa_de_meio_tom_nas_verticais() {
        let alpha = glyph_coverage(Glyph::Pause);
        assert!(alpha.contains(&255));
    }

    #[test]
    fn a_cor_pedida_e_a_que_vai_para_o_icone() {
        let resource = glyph_icon_resource(Glyph::Play, [0x11, 0x22, 0x33]);
        let pixels = &resource[ICON_RESOURCE_HEADER_BYTES..][..ICON_COLOR_BYTES];
        // BGRA: o azul vem primeiro.
        let cheio = pixels
            .chunks_exact(4)
            .find(|p| p[3] == 0xff)
            .expect("algum pixel opaco");
        assert_eq!([cheio[2], cheio[1], cheio[0]], [0x11, 0x22, 0x33]);
    }
}
