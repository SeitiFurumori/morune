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

/// Como os icones da barra de miniaturas sao desenhados.
///
/// `orbe` e o material Aero: o glifo escuro sobre uma esfera de vidro clara
/// em cima e azul embaixo, como os botoes do Windows 7. Sem orbe, o glifo
/// sozinho na cor pedida -- o desenho de todos os outros temas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconStyle {
    /// Cor do glifo.
    pub tint: [u8; 3],
    /// Esfera atras do glifo, ou nada.
    pub orbe: Option<Orbe>,
}

/// As cores da esfera Aero, tiradas do tema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Orbe {
    /// Metade de baixo da esfera (a de cima e quase branca).
    pub base: [u8; 3],
    /// Contorno de 1 px.
    pub borda: [u8; 3],
}

struct PlayerIcons {
    previous: OwnedIcon,
    play: OwnedIcon,
    pause: OwnedIcon,
    next: OwnedIcon,
}

impl PlayerIcons {
    fn new(style: IconStyle) -> Result<Self, TaskbarError> {
        Ok(Self {
            previous: create_glyph_icon(Glyph::Previous, style)?,
            play: create_glyph_icon(Glyph::Play, style)?,
            pause: create_glyph_icon(Glyph::Pause, style)?,
            next: create_glyph_icon(Glyph::Next, style)?,
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
    tint: Cell<IconStyle>,
    receiver: Receiver<NativeEvent>,
    subclass_state: *mut SubclassState,
    registered: Cell<bool>,
    last_shown: Cell<Option<(bool, bool)>>,
}

impl TaskbarControls {
    pub fn new(window: &slint::Window, tint: IconStyle) -> Result<Self, TaskbarError> {
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
    pub fn set_tint(&mut self, tint: IconStyle) {
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

fn create_glyph_icon(glyph: Glyph, style: IconStyle) -> Result<OwnedIcon, TaskbarError> {
    let resource = glyph_icon_resource(glyph, style);
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
fn glyph_shapes(glyph: Glyph, margem: f32) -> Vec<Shape> {
    let lado = ICON_SIZE as f32;
    let esq = margem;
    let dir = lado - margem;
    let topo = margem;
    let base = lado - margem;
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
fn glyph_coverage(glyph: Glyph, margem: f32) -> [u8; ICON_SIZE * ICON_SIZE] {
    let shapes = glyph_shapes(glyph, margem);
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
fn glyph_icon_resource(glyph: Glyph, style: IconStyle) -> [u8; ICON_RESOURCE_BYTES] {
    let tint = style.tint;
    // Dentro da esfera o glifo encolhe: a esfera e a peca, o glifo e o rotulo.
    let alpha = glyph_coverage(
        glyph,
        if style.orbe.is_some() {
            ORB_GLYPH_MARGIN
        } else {
            GLYPH_MARGIN
        },
    );
    let orbe = style.orbe.map(orb_pixels);

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
            // Primeiro a esfera, se houver; o glifo entra por cima dela.
            let fundo = orbe
                .as_ref()
                .map(|o| o[y * ICON_SIZE + x])
                .unwrap_or([0, 0, 0, 0]);
            let (pixel, a) = over(tint, cobertura, fundo);
            if a == 0 {
                continue;
            }
            let dib_y = ICON_SIZE - 1 - y;
            let color = color_start + (dib_y * ICON_SIZE + x) * 4;
            // BGRA, com o alfa da cobertura: o pixel de borda entra parcial.
            resource[color] = pixel[2];
            resource[color + 1] = pixel[1];
            resource[color + 2] = pixel[0];
            resource[color + 3] = a;
            // Qualquer cobertura torna o pixel visivel na mascara; a
            // transparencia parcial quem resolve e o alfa acima.
            resource[mask_start + dib_y * 4 + x / 8] &= !(0x80 >> (x % 8));
        }
    }

    resource
}

/// Margem do glifo quando ha esfera atras: o glifo ocupa metade da esfera.
const ORB_GLYPH_MARGIN: f32 = 9.5;

/// `tinta` com cobertura `a` por cima de `fundo` (RGBA, alfa reto).
fn over(tinta: [u8; 3], a: u8, fundo: [u8; 4]) -> ([u8; 3], u8) {
    let sa = a as f32 / 255.0;
    let da = fundo[3] as f32 / 255.0;
    let oa = sa + da * (1.0 - sa);
    if oa <= 0.0 {
        return ([0, 0, 0], 0);
    }
    let mut out = [0u8; 3];
    for c in 0..3 {
        let v = (tinta[c] as f32 * sa + fundo[c] as f32 * da * (1.0 - sa)) / oa;
        out[c] = v.round().clamp(0.0, 255.0) as u8;
    }
    (out, (oa * 255.0).round() as u8)
}

/// A esfera Aero, pixel a pixel: clara em cima, `base` embaixo, com a troca
/// seca no meio como a capsula do Windows 7, e um contorno de 1 px. A borda
/// externa e suavizada pela distancia ao centro.
fn orb_pixels(orbe: Orbe) -> Vec<[u8; 4]> {
    let lado = ICON_SIZE as f32;
    let centro = lado / 2.0;
    let raio = centro - 1.0;
    let mut out = vec![[0u8; 4]; ICON_SIZE * ICON_SIZE];
    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let dx = x as f32 + 0.5 - centro;
            let dy = y as f32 + 0.5 - centro;
            let d = (dx * dx + dy * dy).sqrt();
            // Cobertura da borda externa: 1 px de transicao.
            let cobertura = (raio + 0.5 - d).clamp(0.0, 1.0);
            if cobertura <= 0.0 {
                continue;
            }
            let t = (y as f32 + 0.5) / lado;
            let cor = if t < 0.48 {
                // Metade de cima: quase branco indo a azul bem claro.
                mistura([243, 251, 255], [223, 241, 251], t / 0.48)
            } else {
                mistura([169, 214, 240], orbe.base, (t - 0.48) / 0.52)
            };
            // Contorno: o ultimo pixel do raio.
            let contorno = (d - (raio - 1.0)).clamp(0.0, 1.0);
            let cor = mistura(cor, orbe.borda, contorno);
            out[y * ICON_SIZE + x] = [cor[0], cor[1], cor[2], (cobertura * 255.0).round() as u8];
        }
    }
    out
}

fn mistura(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    let mut out = [0u8; 3];
    for c in 0..3 {
        out[c] = (a[c] as f32 + (b[c] as f32 - a[c] as f32) * t).round() as u8;
    }
    out
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
    const TINTA: IconStyle = IconStyle {
        tint: [0x6d, 0xd4, 0x9e],
        orbe: None,
    };

    /// Com esfera, o icone e redondo e preenchido: o centro e o canto contam
    /// historias diferentes.
    #[test]
    fn a_esfera_aero_preenche_o_circulo_e_deixa_os_cantos_vazios() {
        let estilo = IconStyle {
            tint: [0x0f, 0x4a, 0x75],
            orbe: Some(Orbe {
                base: [0x98, 0xd1, 0xef],
                borda: [0x0f, 0x4a, 0x75],
            }),
        };
        let resource = glyph_icon_resource(Glyph::Next, estilo);
        let color_start = ICON_RESOURCE_HEADER_BYTES;
        let alfa = |x: usize, y: usize| {
            resource[color_start + ((ICON_SIZE - 1 - y) * ICON_SIZE + x) * 4 + 3]
        };
        assert_eq!(alfa(0, 0), 0, "canto fora da esfera");
        assert_eq!(alfa(16, 4), 255, "topo da esfera, sem glifo, e opaco");
        assert_eq!(alfa(16, 27), 255, "base da esfera e opaca");
        // Metade de cima quase branca, metade de baixo azul.
        let pixel = |x: usize, y: usize| {
            let i = color_start + ((ICON_SIZE - 1 - y) * ICON_SIZE + x) * 4;
            [resource[i + 2], resource[i + 1], resource[i]]
        };
        assert!(pixel(16, 4)[0] > 220, "topo claro: {:?}", pixel(16, 4));
        assert!(pixel(16, 26)[0] < 200, "base azul: {:?}", pixel(16, 26));
    }

    /// Extremos ocupados pelo desenho, em pixels: (esquerda, direita).
    ///
    /// Um pixel conta quando tem alguma cobertura; a borda suavizada entra.
    fn extremos(glyph: Glyph) -> (usize, usize) {
        let alpha = glyph_coverage(glyph, GLYPH_MARGIN);
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
        let alpha = glyph_coverage(Glyph::Play, GLYPH_MARGIN);
        let parciais = alpha.iter().filter(|&&a| a > 0 && a < 255).count();
        assert!(parciais > 10, "borda sem meio-tom: {parciais} pixels");
    }

    /// O pause e feito de retangulos alinhados ao pixel: nao ha o que suavizar,
    /// e um meio-tom ali seria borrao, nao curva.
    #[test]
    fn o_pause_nao_precisa_de_meio_tom_nas_verticais() {
        let alpha = glyph_coverage(Glyph::Pause, GLYPH_MARGIN);
        assert!(alpha.contains(&255));
    }

    #[test]
    fn a_cor_pedida_e_a_que_vai_para_o_icone() {
        let resource = glyph_icon_resource(
            Glyph::Play,
            IconStyle {
                tint: [0x11, 0x22, 0x33],
                orbe: None,
            },
        );
        let pixels = &resource[ICON_RESOURCE_HEADER_BYTES..][..ICON_COLOR_BYTES];
        // BGRA: o azul vem primeiro.
        let cheio = pixels
            .chunks_exact(4)
            .find(|p| p[3] == 0xff)
            .expect("algum pixel opaco");
        assert_eq!([cheio[2], cheio[1], cheio[0]], [0x11, 0x22, 0x33]);
    }
}
