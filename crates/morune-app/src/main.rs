//! Ponto de entrada do Morune.
//!
//! A ordem de inicializacao aqui e deliberada e faz parte do orcamento de
//! startup: nada que dependa de rede acontece antes da janela aparecer.
//!
//! 1. caminhos e log (sem I/O pesado)
//! 2. configuracao
//! 3. tema (com fallback garantido)
//! 4. janela
//! 5. so entao autenticacao, biblioteca, cache

// Sem console preto atras da janela no Windows, inclusive quando o executavel
// e iniciado diretamente pelo Explorer para uma sessao de uso diario.
#![cfg_attr(windows, windows_subsystem = "windows")]
// A unica fronteira `unsafe` do aplicativo fica isolada em `taskbar.rs`: a API
// nativa exige COM, HWND e um callback Win32. Dentro dela, cada operacao ainda
// precisa declarar explicitamente o seu bloco inseguro.
#![deny(unsafe_op_in_unsafe_fn)]

mod artwork;
mod browse;
mod bundled;
#[cfg(windows)]
mod clipboard;
#[cfg(windows)]
mod crash;
mod hotkeys;
#[cfg(windows)]
mod instance;
mod optics;
mod quadros;
mod session;
mod smtc;
#[cfg(feature = "snapshot")]
mod snapshot;
mod startup;
mod state;
#[cfg(windows)]
mod taskbar;
mod tela_cheia;
mod theme_bridge;
mod tint;
mod tray;
mod tray_menu;
mod update;
mod wallpaper;

pub mod ui {
    slint::include_modules!();
}

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use slint::ComponentHandle;

use state::AppState;

fn main() -> anyhow::Result<()> {
    // O empacotador executa o binario com um PATH que contem apenas o Windows.
    // Chegar ate aqui prova que o loader encontrou todas as DLLs importadas.
    // Nao criamos a janela nesse modo porque runners do GitHub executam como
    // servico, sem uma area de trabalho interativa onde o Slint possa abrir UI.
    if let Some(report) = std::env::var_os("MORUNE_BINARY_CHECK_FILE") {
        std::fs::write(report, "binary_loaded=ok\n")?;
        return Ok(());
    }

    #[cfg(windows)]
    let _single_instance =
        if cfg!(feature = "snapshot") && std::env::var_os("MORUNE_SNAPSHOT").is_some() {
            // A ferramenta visual precisa coexistir com o aplicativo que o dono
            // esta usando. Isto nao entra no build normal porque a feature fica
            // desligada em release.
            None
        } else {
            let Some(instance) = instance::SingleInstance::acquire()? else {
                return Ok(());
            };
            Some(instance)
        };

    let started = Instant::now();
    let started_with_windows = std::env::args_os().any(|arg| arg == "--startup");
    // Os caminhos vem antes do log porque e neles que o arquivo de log mora.
    // `ensure` e idempotente; `AppState::load` chama de novo logo abaixo.
    let paths = morune_storage::AppPaths::discover();
    let _ = paths.ensure();
    init_logging(&paths);
    log_panics();
    crash::instalar(paths.data_dir());
    // `MORUNE_RELEASE`, e nao `CARGO_PKG_VERSION`: a versao do crate fica parada
    // em 0.1.0 enquanto as tags avancam, entao a primeira linha do log dizia
    // "0.1.0" tanto para um alpha publicado quanto para um build local de hoje
    // -- justamente a pergunta que se vai ao log para responder. Ver
    // `embed_release_tag` em build.rs.
    tracing::info!(versao = env!("MORUNE_RELEASE"), log = %paths.log_file().display(), "Morune iniciando");

    let mut state = AppState::load();
    state.open_page_from_env();
    let window = ui::AppWindow::new()?;

    state.apply_theme_to(&window);
    state.apply_initial_window_size(&window);
    #[cfg(feature = "snapshot")]
    if let (Ok(width), Ok(height)) = (
        std::env::var("MORUNE_SNAPSHOT_WIDTH")
            .and_then(|v| v.parse::<f32>().map_err(|_| std::env::VarError::NotPresent)),
        std::env::var("MORUNE_SNAPSHOT_HEIGHT")
            .and_then(|v| v.parse::<f32>().map_err(|_| std::env::VarError::NotPresent)),
    ) {
        window.window().set_maximized(false);
        window
            .window()
            .set_size(slint::LogicalSize::new(width, height));
    }
    #[cfg(feature = "snapshot")]
    if std::env::var_os("MORUNE_SNAPSHOT_SHORTCUTS").is_some() {
        window.set_shortcuts_visible(true);
    }
    #[cfg(feature = "snapshot")]
    if std::env::var_os("MORUNE_SNAPSHOT_MINI_PLAYER").is_some() {
        window.set_mini_player(true);
        window
            .window()
            .set_size(slint::LogicalSize::new(620.0, 150.0));
    }
    state.push_to_ui(&window);

    // A bandeja precisa existir antes de interceptar o fechamento: sem ela nao
    // ha como o usuario trazer a janela de volta nem encerrar o aplicativo, e
    // fechar tem de voltar a encerrar.
    let tray = match tray::Tray::new() {
        Ok(tray) => Some(Rc::new(tray)),
        Err(e) => {
            tracing::warn!(error = %e, "bandeja indisponivel; fechar a janela vai encerrar");
            None
        }
    };

    let state = Rc::new(std::cell::RefCell::new(state));
    wire_callbacks(&window, &state);
    wire_window_chrome(&window);
    wire_mini_player(&window);
    let _tray_poll = wire_tray(&window, &state, tray.clone());
    let _backend_poll = wire_backend(&window, &state);
    let _progress_tick = wire_progress_tick(&window, &state);
    let _fullscreen_gate = wire_fullscreen_gate(&window, &state);
    let _window_state_poll = wire_window_state(&window, &state);
    wire_close_behavior(&window, &state, tray.is_some());
    let _quadros = quadros::instalar(&window);

    // Medida real do caminho critico, comparavel entre execucoes. Aparece no
    // log sempre e na sobreposicao de performance no Developer Mode.
    let ready = started.elapsed();
    tracing::info!(ms = ready.as_millis(), "interface pronta");
    window.set_perf_text(
        format!(
            "startup {} ms | tema {}",
            ready.as_millis(),
            state.borrow().theme_id()
        )
        .into(),
    );

    // Modo de medicao: abre a janela de verdade, espera o laco de eventos
    // comecar a rodar (primeiro quadro agendado) e so entao sai. Medir antes de
    // `run()` daria um numero bonito e falso, porque a janela ainda nao existe
    // na tela.
    let measuring = std::env::var_os("MORUNE_EXIT_AFTER_STARTUP").is_some();
    if measuring {
        // O binario de release e do subsistema "windows" e nao tem stdout, por
        // isso o resultado vai para um arquivo: imprimir na tela funcionaria so
        // em depuracao, e a medida que interessa e a do binario de release.
        let report = std::env::var_os("MORUNE_STARTUP_FILE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("morune-startup.txt"));

        slint::Timer::single_shot(std::time::Duration::ZERO, move || {
            let ms = started.elapsed().as_millis();
            println!("startup_ms={ms}");
            let _ = std::fs::write(&report, format!("startup_ms={ms}\n"));
            let _ = slint::quit_event_loop();
        });
    }

    // Verificacao visual: renderiza, salva a janela em PNG e sai. Capturar pelo
    // renderizador e nao pela tela mantem a captura limpa mesmo com a janela em
    // segundo plano.
    #[cfg(feature = "snapshot")]
    if let Some(path) = std::env::var_os("MORUNE_SNAPSHOT") {
        let weak = window.as_weak();
        let path = std::path::PathBuf::from(path);
        let delay = std::env::var("MORUNE_SNAPSHOT_DELAY_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(600);
        slint::Timer::single_shot(std::time::Duration::from_millis(delay), move || {
            if let Some(w) = weak.upgrade() {
                match snapshot::save(&w, &path) {
                    Ok((width, height)) => {
                        println!("snapshot={} {}x{}", path.display(), width, height)
                    }
                    Err(e) => eprintln!("falha ao capturar: {e}"),
                }
            }
            let _ = slint::quit_event_loop();
        });
    }

    // `window.run()` encerraria o laco quando a janela some, que e exatamente o
    // que nao pode acontecer: com a bandeja ativa a janela fecha e o aplicativo
    // continua vivo, tocando. Quem termina o laco e `quit_event_loop`, chamado
    // pelo item "Sair" da bandeja ou quando a bandeja nao existe.
    window.show()?;
    // Primeira tentativa. Pode nao pegar -- o HWND as vezes ainda nao existe
    // aqui --, e nesse caso quem resolve e o temporizador de estado da janela.
    #[cfg(windows)]
    ensure_rounded_corners(window.window());
    #[cfg(windows)]
    {
        let s = state.borrow();
        let (opacity, material) = s.window_effects();
        window.set_native_backdrop_active(ensure_window_effects(
            window.window(),
            opacity,
            backdrop_da_janela(material, s.tela_cheia_ativa()),
        ));
    }
    #[cfg(windows)]
    let _taskbar_poll = wire_taskbar(&window, &state);
    // Recriado a cada troca do interruptor, e nao so aqui: por isso vive numa
    // celula em vez de numa variavel local.
    #[cfg(feature = "hot-reload")]
    let theme_watch = Rc::new(RefCell::new(wire_theme_watch(&window, &state)));
    #[cfg(feature = "hot-reload")]
    {
        let weak = window.as_weak();
        let state = state.clone();
        let cell = theme_watch.clone();
        window.on_set_hot_reload(move |on| {
            let Some(window) = weak.upgrade() else { return };
            state.borrow_mut().set_hot_reload(on);
            *cell.borrow_mut() = wire_theme_watch(&window, &state);
            state.borrow().push_to_ui(&window);
        });
    }
    #[cfg(not(feature = "hot-reload"))]
    window.on_set_hot_reload(|_| {});
    if started_with_windows && tray.is_some() {
        window.hide()?;
        tracing::info!("inicializacao do Windows concluida na bandeja");
    }
    slint::run_event_loop_until_quit()?;

    state.borrow().save_config();
    Ok(())
}

/// Move a janela sem recorrer a APIs inseguras da plataforma.
///
/// A interface envia deltas em pixels logicos; `Window::position` trabalha em
/// pixels fisicos. Multiplicar pelo fator de escala evita que o ponteiro se
/// afaste da title bar em 125%, 150% ou 200% de DPI.
/// Garante que a janela esteja com os cantos arredondados.
///
/// A janela e `no-frame`, entao o Windows nao desenha borda nem canto: sem
/// isto ela e um retangulo de canto vivo, que destoa de tudo no Windows 11.
///
/// Quem arredonda e o gerenciador de janelas, e nao o Slint. A diferenca
/// importa: o DWM recorta a **regiao** da janela, entao o conteudo desenhado
/// tambem sai recortado e a sombra acompanha a curva. Um `border-radius` no
/// retangulo raiz so pintaria o canto de outra cor, com o pixel quadrado da
/// janela continuando ali por baixo.
///
/// Sem efeito no Windows 10, que nao conhece este atributo: a chamada devolve
/// erro, o canto continua vivo e nada mais muda. Por isso a falha so vai para o
/// log -- nao ha o que o usuario possa fazer a respeito.
///
/// **Chamada repetidamente, de proposito.** Logo depois de `show()` o backend
/// do Slint ainda pode nao ter criado o HWND -- o mesmo motivo que faz a
/// integracao com a barra de tarefas nascer num temporizador --, e a primeira
/// tentativa simplesmente nao encontra janela para configurar. Alem disso,
/// esconder e reabrir a janela pode recria-la, e a janela nova volta com o
/// canto padrao. Ler antes de escrever mantem o custo em duas chamadas baratas
/// quando ja esta certo, que e o caso comum.
#[cfg(windows)]
fn ensure_rounded_corners(window: &slint::Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{
        DwmGetWindowAttribute, DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    };

    let handle = window.window_handle();
    let Ok(handle) = handle.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(raw) = handle.as_raw() else {
        return;
    };
    let hwnd = HWND(raw.hwnd.get() as *mut std::ffi::c_void);

    // `DWMWCP_ROUND` e o raio padrao do sistema, o mesmo das outras janelas do
    // Windows 11. Escolher um numero proprio faria o Morune ser o unico
    // aplicativo com canto diferente na tela.
    let preference = DWMWCP_ROUND;

    // Ler primeiro: no caso comum a janela ja esta configurada e nao ha nada a
    // fazer. Isto e o que permite chamar esta funcao a cada meio segundo.
    let mut applied = 0i32;
    // SAFETY: `hwnd` vem da janela viva, e o ponteiro aponta para um inteiro do
    // tamanho declarado, vivo durante a chamada.
    let read = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            std::ptr::addr_of_mut!(applied).cast(),
            std::mem::size_of_val(&applied) as u32,
        )
    };
    if read.is_ok() && applied == preference.0 {
        return;
    }

    // SAFETY: mesmas condicoes da leitura acima.
    let written = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            std::ptr::addr_of!(preference).cast(),
            std::mem::size_of_val(&preference) as u32,
        )
    };
    match written {
        Ok(()) => tracing::info!(pedido = preference.0, "cantos da janela arredondados"),
        Err(error) => {
            tracing::debug!(%error, "cantos arredondados indisponiveis nesta versao do Windows")
        }
    }
}

/// O material que o tema pede, traduzido para a janela do aplicativo.
///
/// **Voltou a ser acrilico, e Mica foi um desvio.** A documentacao do Windows
/// diz "acrylic is used only for transient, light-dismiss surfaces" e reserva
/// Mica para "long-lived windows such as apps and settings", e por isso a janela
/// principal passou a usar Mica. Duas coisas derrubam essa leitura aqui:
///
/// 1. **A frase e conselho de estilo e de bateria, nao impedimento.** O que ela
///    protege -- gasto de video -- e coberto pelo portao de tela cheia, que nao
///    existia quando a troca foi feita.
/// 2. **Mica nao entrega o que o tema de vidro precisa.** Ele amostra o papel de
///    parede, uma vez. Numa maquina onde o papel de parede e desenhado por outro
///    programa (Wallpaper Engine e o caso aqui), o Windows nao tem arquivo
///    nenhum para apontar e o resultado e nada. Acrilico amostra o que esta
///    ATRAS DA JANELA, que e a definicao do efeito.
///
/// O caso ruim conhecido do acrilico -- colapsar em cinza sobre um jogo em tela
/// cheia, porque o desktop nao esta visivel -- deixa de acontecer: o portao
/// desliga o material antes disso.
///
/// `tela_cheia` desliga tudo. E o mesmo portao do movimento.
#[cfg(windows)]
fn backdrop_da_janela(pedido_do_tema: bool, tela_cheia: bool) -> Backdrop {
    if pedido_do_tema && !tela_cheia {
        Backdrop::Acrylic
    } else {
        Backdrop::None
    }
}

/// Material de fundo que o Windows deve pintar atras da janela.
#[cfg(windows)]
#[derive(Clone, Copy, PartialEq)]
enum Backdrop {
    /// Sem material: o tema pinta o fundo inteiro.
    None,
    /// Superficie translucida do aplicativo enquanto ele esta visivel.
    Acrylic,
}

/// Aplica opacidade e fundo acrilico da janela, pedidos pelo tema.
///
/// Os dois vinham sendo validados e documentados sem nunca chegar a janela:
/// `theme_bridge` aplicava metade dos `EffectTokens` e o resto era letra morta.
///
/// **Acrilico e composicao do sistema, nao do aplicativo.** Quem desenha o
/// borrado atras da janela e o DWM, uma vez, e nao o Morune a cada quadro --
/// que e a unica forma aceitavel dado que isto toca enquanto a pessoa joga.
/// Para o efeito aparecer, a cor de fundo do tema precisa ter alfa: um
/// `background` opaco cobre o acrilico e o resultado e uma janela normal.
///
/// **Opacidade usa janela em camada.** Um aviso honesto: janela em camada
/// convive mal com renderizador por OpenGL em alguns drivers. Se a janela
/// ficar preta com `window_opacity < 1`, a causa e essa, e a saida e o
/// renderizador por software (`SLINT_BACKEND=winit-software`).
///
/// Chamada repetidamente pelo mesmo motivo de `ensure_rounded_corners`: o HWND
/// pode nao existir logo depois de `show()`, e reabrir a janela a recria.
#[cfg(windows)]
fn ensure_window_effects(window: &slint::Window, opacity: f32, backdrop: Backdrop) -> bool {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::Foundation::{COLORREF, HWND};
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMSBT_NONE, DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE,
    };
    use windows::Win32::Graphics::Gdi::{
        RedrawWindow, RDW_ALLCHILDREN, RDW_ERASE, RDW_FRAME, RDW_INVALIDATE,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowLongPtrW, GWL_EXSTYLE, LWA_ALPHA,
        WINDOW_EX_STYLE, WS_EX_LAYERED,
    };

    let handle = window.window_handle();
    let Ok(handle) = handle.window_handle() else {
        return false;
    };
    let RawWindowHandle::Win32(raw) = handle.as_raw() else {
        return false;
    };
    let hwnd = HWND(raw.hwnd.get() as *mut std::ffi::c_void);

    let backdrop = match backdrop {
        Backdrop::Acrylic => DWMSBT_TRANSIENTWINDOW,
        Backdrop::None => DWMSBT_NONE,
    };
    // SAFETY: `hwnd` vem da janela viva; o ponteiro aponta para um inteiro do
    // tamanho declarado, vivo durante a chamada.
    let written = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            std::ptr::addr_of!(backdrop).cast(),
            std::mem::size_of_val(&backdrop) as u32,
        )
    };
    let native_backdrop = backdrop == DWMSBT_TRANSIENTWINDOW && written.is_ok();
    if let Err(error) = written {
        // Windows 10 nao conhece este atributo. Nao ha o que o usuario possa
        // fazer, entao fica so no log.
        tracing::debug!(%error, "material de fundo indisponivel nesta versao do Windows");
    }

    // A janela so vira "em camada" quando o tema realmente pede transparencia.
    // Ligar o estilo a toa custaria o caminho de composicao mais lento para
    // todo mundo, inclusive para quem nunca mexeu nisso.
    //
    // Quando o tema **nao** pede, o estilo tem de ser removido, e nao apenas
    // ignorado: sem isso a transparencia de um tema sobrevivia a troca para um
    // tema opaco e contaminava todos os outros ate fechar o aplicativo.
    let alpha = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;

    // SAFETY: `hwnd` e valido; ler e escrever o estilo estendido de uma janela
    // propria e a forma documentada de ligar e desligar `WS_EX_LAYERED`.
    unsafe {
        let atual = WINDOW_EX_STYLE(GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32);
        let em_camada = atual.contains(WS_EX_LAYERED);

        if alpha == 255 {
            if em_camada {
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (atual & !WS_EX_LAYERED).0 as isize);
                // Sem repintar, a janela fica com o ultimo quadro composto pelo
                // caminho em camada e so volta ao normal no proximo redesenho.
                let _ = RedrawWindow(
                    Some(hwnd),
                    None,
                    None,
                    RDW_INVALIDATE | RDW_ERASE | RDW_FRAME | RDW_ALLCHILDREN,
                );
                tracing::info!("transparencia da janela desligada");
            }
            return native_backdrop;
        }

        if !em_camada {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (atual | WS_EX_LAYERED).0 as isize);
        }
        if let Err(error) = SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA) {
            tracing::debug!(%error, "opacidade da janela nao aplicada");
        }
    }
    native_backdrop
}

/// Liga o observador de temas, se a pessoa pediu recarga ao salvar.
///
/// O observador chama exatamente o mesmo caminho do botao **Recarregar temas**
/// -- nao ha um segundo caminho de recarga para manter em pe. Fica atras da
/// feature `hot-reload` porque custa uma thread e alguns handles do sistema, e
/// a maioria das pessoas nunca edita um tema.
///
/// O retorno precisa ficar vivo: soltar o `ThemeWatcher` para de observar.
#[cfg(feature = "hot-reload")]
fn wire_theme_watch(
    window: &ui::AppWindow,
    state: &Rc<RefCell<AppState>>,
) -> Option<morune_theme::watch::ThemeWatcher> {
    let (ligado, dir) = {
        let s = state.borrow();
        (s.hot_reload(), s.themes_dir())
    };
    if !ligado {
        return None;
    }

    // `Weak` atravessa threads; `Rc<RefCell<AppState>>` nao. Por isso o
    // observador nao toca o estado: ele so pede a janela, ja no laco de
    // eventos, para disparar a recarga que ja existe.
    let weak = window.as_weak();
    match morune_theme::watch::ThemeWatcher::new(&dir, move |_| {
        let _ = weak.upgrade_in_event_loop(|window| window.invoke_reload_theme());
    }) {
        Ok(watcher) => {
            tracing::info!(dir = %dir.display(), "recarga de tema ao salvar ligada");
            Some(watcher)
        }
        Err(error) => {
            // Pasta inexistente ou limite de observadores do sistema. Nao ha o
            // que o usuario possa fazer, e o botao Recarregar continua ali.
            tracing::warn!(%error, "observador de temas indisponivel");
            None
        }
    }
}

fn wire_window_chrome(window: &ui::AppWindow) {
    let weak = window.as_weak();
    window.on_move_window(move |delta_x, delta_y| {
        if !delta_x.is_finite() || !delta_y.is_finite() {
            return;
        }

        let Some(window) = weak.upgrade() else {
            return;
        };
        let native = window.window();
        let position = native.position();
        let scale = native.scale_factor();
        native.set_position(slint::PhysicalPosition::new(
            position.x + (delta_x * scale).round() as i32,
            position.y + (delta_y * scale).round() as i32,
        ));
    });
}

/// Quantas voltas de meio segundo com a janela escondida antes de devolver a
/// memoria fisica. Quatro dao dois segundos: tempo de sobra para a janela
/// terminar de sumir e o renderizador soltar o ultimo quadro.
#[cfg(windows)]
const VOLTAS_ATE_DEVOLVER: u32 = 4;

/// Devolve ao Windows a memoria fisica do processo.
///
/// **Por que.** Na bandeja o Morune segurava 167 MB de RAM fisica -- medido em
/// 06/09/2026 --, quase toda textura e contexto do driver de video que ninguem
/// vai olhar com a janela escondida. E exatamente a hora em que alguem esta
/// jogando, e RAM fisica e o que falta num jogo.
///
/// `EmptyWorkingSet` nao apaga nada e nao descarrega o aplicativo: marca as
/// paginas como candidatas a sair da RAM. O que for preciso de novo volta
/// sozinho quando a janela reaparecer. Medido no aplicativo real: 150,8 MB caem
/// para 1,2 MB, e a janela inteira redesenhada volta a 25,6 MB -- ou seja, a
/// maior parte do que estava na RAM nao estava sendo usada.
///
/// So na transicao para a bandeja: repetir com a janela visivel tiraria da RAM
/// justamente as paginas que o proximo quadro pede de volta.
#[cfg(windows)]
fn devolver_memoria() {
    use windows::Win32::System::ProcessStatus::EmptyWorkingSet;
    use windows::Win32::System::Threading::GetCurrentProcess;

    // SAFETY: o pseudo-handle do proprio processo e sempre valido, nao precisa
    // ser fechado, e a chamada nao guarda ponteiro nenhum.
    let resultado = unsafe { EmptyWorkingSet(GetCurrentProcess()) };
    match resultado {
        Ok(()) => tracing::debug!("memoria fisica devolvida ao Windows"),
        Err(erro) => tracing::debug!(%erro, "nao consegui devolver a memoria fisica"),
    }
}

/// Observa tamanho/maximizacao com baixa frequencia. Isso tambem cobre resize
/// pelo teclado, snap layouts e restauracao do Windows, nao apenas arraste.
fn wire_window_state(
    window: &ui::AppWindow,
    state: &Rc<std::cell::RefCell<AppState>>,
) -> slint::Timer {
    let weak = window.as_weak();
    let state = state.clone();
    let timer = slint::Timer::default();
    // Voltas seguidas com a janela escondida. Serve para devolver a memoria uma
    // vez so por ida a bandeja -- ver `devolver_memoria`.
    let mut voltas_escondida: u32 = 0;
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(500),
        move || {
            let _cronometro = quadros::Cronometro::new("janela escondida");
            let Some(window) = weak.upgrade() else { return };
            // Escondida na bandeja nao ha canto para arredondar, efeito para
            // aplicar nem tamanho para lembrar -- e essas sao chamadas ao DWM,
            // que atravessam a fronteira do processo. Duas por segundo a toa
            // enquanto alguem joga e exatamente o custo invisivel que o
            // criterio de desempenho do projeto existe para barrar.
            if !window.window().is_visible() {
                voltas_escondida = voltas_escondida.saturating_add(1);
                if voltas_escondida == 1 {
                    // A aurora do vidro Aero para junto com a janela: escondida
                    // nao ha o que animar, e o relogio dela acordaria vinte
                    // vezes por segundo a toa.
                    window.global::<ui::Theme>().set_aurora_viva(false);
                }
                #[cfg(windows)]
                if voltas_escondida == VOLTAS_ATE_DEVOLVER {
                    devolver_memoria();
                }
                return;
            }
            if voltas_escondida > 0 {
                window.global::<ui::Theme>().set_aurora_viva(true);
            }
            voltas_escondida = 0;
            // Antes do desvio do mini-player: o canto arredondado vale para as
            // duas formas da janela. No caso comum sao duas chamadas baratas
            // que confirmam que ja esta certo.
            #[cfg(windows)]
            ensure_rounded_corners(window.window());
            #[cfg(windows)]
            {
                let s = state.borrow();
                let (opacity, material) = s.window_effects();
                window.set_native_backdrop_active(ensure_window_effects(
                    window.window(),
                    opacity,
                    backdrop_da_janela(material, s.tela_cheia_ativa()),
                ));
            }
            if window.get_mini_player() {
                return;
            }
            let native = window.window();
            let size = native.size();
            let scale = native.scale_factor().max(0.1);
            state.borrow_mut().remember_window_state(
                size.width as f32 / scale,
                size.height as f32 / scale,
                native.is_maximized(),
            );
        },
    );
    timer
}

fn wire_mini_player(window: &ui::AppWindow) {
    let weak = window.as_weak();
    let previous = Rc::new(std::cell::Cell::new(None::<(f32, f32, bool)>));
    window.on_toggle_mini_player(move || {
        let Some(window) = weak.upgrade() else { return };
        let native = window.window();
        if !window.get_mini_player() {
            let size = native.size();
            let scale = native.scale_factor().max(0.1);
            previous.set(Some((
                size.width as f32 / scale,
                size.height as f32 / scale,
                native.is_maximized(),
            )));
            native.set_maximized(false);
            window.set_mini_player(true);
            native.set_size(slint::LogicalSize::new(620.0, 150.0));
        } else {
            window.set_mini_player(false);
            if let Some((width, height, maximized)) = previous.get() {
                native.set_size(slint::LogicalSize::new(width, height));
                native.set_maximized(maximized);
            }
        }
    });
}

/// Faz o fechamento da janela esconder em vez de encerrar, quando o usuario
/// pediu isso e ha bandeja para trazer o aplicativo de volta.
fn wire_close_behavior(
    window: &ui::AppWindow,
    state: &Rc<std::cell::RefCell<AppState>>,
    has_tray: bool,
) {
    let state = state.clone();
    let weak = window.as_weak();
    window.window().on_close_requested(move || {
        let keep_running = has_tray && state.borrow().close_to_tray();
        let show_hint = keep_running && state.borrow_mut().take_tray_hint();
        state.borrow().save_config();
        if keep_running {
            tracing::info!("janela escondida na bandeja; reproducao continua");
            if show_hint {
                if let Some(window) = weak.upgrade() {
                    let _ = window.hide();
                }
                let _ = rfd::MessageDialog::new()
                    .set_title("Morune continua tocando")
                    .set_description(
                        "A janela foi escondida na bandeja do Windows. Use o ícone do Morune para abrir novamente ou sair.",
                    )
                    .set_buttons(rfd::MessageButtons::Ok)
                    .show();
            }
        } else {
            let _ = slint::quit_event_loop();
        }
        // Nos dois casos a janela some. A diferenca esta em o laco de eventos
        // continuar rodando ou nao.
        slint::CloseRequestResponse::HideWindow
    });
}

/// Liga a bandeja ao aplicativo.
///
/// Intervalo de leitura do backend de reproducao.
///
/// A librespot vive noutra thread e conversa por canal. 100 ms e imperceptivel
/// para troca de faixa e mantem o custo em repouso proximo de zero -- o teste
/// de CPU ociosa nao pode regredir por causa disto.
const BACKEND_POLL: std::time::Duration = std::time::Duration::from_millis(100);

/// Liga o backend do Spotify a interface.
///
/// A restauracao de sessao comeca aqui, e nao no `main`, de proposito: quando
/// este temporizador dispara pela primeira vez a janela ja apareceu, e o
/// orcamento de startup fica intacto mesmo com a rede lenta.
///
/// O temporizador devolvido precisa continuar vivo: descartado, o aplicativo
/// para de receber login, fim de faixa e erro de reproducao.
fn wire_backend(window: &ui::AppWindow, state: &Rc<std::cell::RefCell<AppState>>) -> slint::Timer {
    let weak = window.as_weak();
    let state = state.clone();
    let mut started = false;

    // Mudanca que aconteceu com a janela escondida e que a tela ainda nao viu.
    let mut pendente = false;
    let mut estava_visivel = true;

    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, BACKEND_POLL, move || {
        let Some(window) = weak.upgrade() else { return };

        if !started {
            started = true;
            state.borrow_mut().restore_session();
        }

        // O backend continua sendo lido com a janela na bandeja: e o que faz a
        // musica seguir, a fila andar e a bandeja mostrar a faixa certa. O que
        // para e **escrever na interface** -- ver `push_to_ui`.
        let t = std::time::Instant::now();
        pendente |= state.borrow_mut().poll_backend();
        quadros::tarefa("poll_backend", t);

        let visivel = window.window().is_visible();
        // Reaparecer conta como motivo para espelhar tudo, mesmo sem mudanca
        // nova: a tela pode ter ficado minutos sem receber o que se acumulou.
        if visivel && (pendente || !estava_visivel) {
            let t = std::time::Instant::now();
            state.borrow().push_to_ui(&window);
            quadros::tarefa("push_to_ui", t);
            pendente = false;
        }
        estava_visivel = visivel;
    });

    timer
}

/// Intervalo do relogio de progresso.
///
/// 250 ms da quatro atualizacoes por segundo: a barra anda visivelmente e o
/// tempo decorrido nunca fica mais de um quarto de segundo atrasado, sem
/// repintar a tela na taxa do monitor.
const PROGRESS_TICK: std::time::Duration = std::time::Duration::from_millis(250);

/// Faz a barra de progresso andar sozinha.
///
/// **Por que existe:** `wire_backend` so espelha a interface quando
/// `poll_backend` acusa mudanca, e reproducao estavel nao produz evento nenhum
/// -- a posicao vive num relogio interpolado do motor, que ninguem lia. Sem
/// este temporizador a barra e o tempo decorrido ficam parados enquanto a
/// musica toca, e um seek parece nao ter efeito.
///
/// Pausado nao custa nada: o tique sai antes de tocar na interface, entao o
/// numero de CPU em repouso de `docs/PERFORMANCE.md` nao se mexe.
///
/// **Com a janela na bandeja tambem nao custa nada**, e essa e a medida que
/// motivou a checagem: tocando e escondido, a thread da interface gastava 1,93%
/// de um nucleo contra 1,09% da saida de audio -- ou seja, o aplicativo gastava
/// mais desenhando o que ninguem via do que tocando musica. A barra de
/// progresso nao precisa andar numa janela que nao esta na tela; quando ela
/// volta, `wire_backend` espelha tudo de uma vez.
///
/// O temporizador devolvido precisa continuar vivo.
/// De quanto em quanto tempo o vigia de tela cheia olha.
///
/// Um segundo. Entrar num jogo nao e urgencia -- ninguem percebe o visual
/// mudar meio segundo depois --, e a leitura custa duas chamadas ao sistema.
/// Mais rapido que isso seria gasto invisivel, que e o que o orcamento de
/// desempenho do projeto proibe.
const VIGIA_TELA_CHEIA: std::time::Duration = std::time::Duration::from_secs(1);

/// Liga o portao de tela cheia.
///
/// Enquanto um aplicativo de outro processo ocupa a tela inteira, o Morune
/// entra no mesmo estado de "movimento reduzido" que ja existia -- e daqui para
/// frente e por este mesmo caminho que os efeitos caros vao ser desligados.
///
/// **Reaplica o tema so quando o estado vira**, e nao a cada leitura: aplicar
/// tema reescreve dezenas de propriedades e forca um redesenho. Na quase
/// totalidade das leituras nada mudou, e entao nada acontece.
fn wire_fullscreen_gate(
    window: &ui::AppWindow,
    state: &Rc<std::cell::RefCell<AppState>>,
) -> slint::Timer {
    let weak = window.as_weak();
    let state = state.clone();

    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, VIGIA_TELA_CHEIA, move || {
        let _cronometro = quadros::Cronometro::new("tela cheia");
        let Some(window) = weak.upgrade() else { return };
        let cheia = crate::tela_cheia::app_em_tela_cheia();

        let mudou = state.borrow_mut().set_tela_cheia(cheia);
        if mudou {
            state.borrow().apply_theme_to(&window);
            // O material do Windows tambem entra e sai por aqui: acrilico
            // amostra o desktop, e sobre um jogo em tela cheia o desktop nao
            // esta visivel -- ele colapsaria em cinza chapado. Desligar antes e
            // o que torna acrilico aceitavel numa janela de vida longa.
            #[cfg(windows)]
            {
                let s = state.borrow();
                let (opacidade, material) = s.window_effects();
                window.set_native_backdrop_active(ensure_window_effects(
                    window.window(),
                    opacidade,
                    backdrop_da_janela(material, s.tela_cheia_ativa()),
                ));
            }
            // `info` e nao `debug`: acontece duas vezes por partida de jogo, e
            // e a primeira coisa que alguem vai querer no registro quando
            // reclamar de "o visual sumiu" ou "gastou video durante o jogo".
            tracing::info!(tela_cheia = cheia, "portao de tela cheia");
        }
    });

    timer
}

fn wire_progress_tick(
    window: &ui::AppWindow,
    state: &Rc<std::cell::RefCell<AppState>>,
) -> slint::Timer {
    let weak = window.as_weak();
    let state = state.clone();

    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, PROGRESS_TICK, move || {
        let _cronometro = quadros::Cronometro::new("progresso");
        let Some(window) = weak.upgrade() else { return };
        if !window.window().is_visible() {
            return;
        }
        let state = state.borrow();
        if !state.is_playing() {
            return;
        }
        state.push_playback(&window);
    });

    timer
}

/// Devolve o temporizador de leitura, que precisa continuar vivo: descartado,
/// a bandeja para de responder.
fn wire_tray(
    window: &ui::AppWindow,
    state: &Rc<std::cell::RefCell<AppState>>,
    tray: Option<Rc<tray::Tray>>,
) -> Option<slint::Timer> {
    let tray = tray?;
    let weak = window.as_weak();
    let state = state.clone();
    // O menu so existe enquanto esta aberto. Ver `tray_menu`.
    let open_menu: Rc<RefCell<Option<tray_menu::TrayMenu>>> = Rc::new(RefCell::new(None));

    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, tray::POLL_INTERVAL, move || {
        let _cronometro = quadros::Cronometro::new("bandeja");
        let Some(window) = weak.upgrade() else { return };

        for command in tray.poll() {
            match command {
                tray::TrayCommand::Show => {
                    *open_menu.borrow_mut() = None;
                    if let Err(e) = window.show() {
                        tracing::error!(error = %e, "nao foi possivel reabrir a janela");
                    }
                }
                tray::TrayCommand::OpenMenu(anchor) => {
                    // Um segundo clique com o menu aberto fecha, como faria o
                    // menu do sistema.
                    if open_menu.borrow().is_some() {
                        *open_menu.borrow_mut() = None;
                        continue;
                    }
                    match tray_menu::TrayMenu::open(anchor) {
                        Ok(menu) => {
                            state.borrow().push_to_tray_menu(menu.window());
                            wire_tray_menu(&menu, &window, &state, &open_menu);
                            *open_menu.borrow_mut() = Some(menu);

                            // No proximo giro, e nao daqui a 150 ms: e quando a
                            // janela do sistema passa a existir, e ate la o
                            // menu esta na tela sem posicao definida.
                            let open_menu = open_menu.clone();
                            slint::Timer::single_shot(Duration::ZERO, move || {
                                if let Some(menu) = open_menu.borrow_mut().as_mut() {
                                    menu.attach();
                                }
                            });
                        }
                        Err(e) => tracing::error!(error = %e, "menu da bandeja nao abriu"),
                    }
                }
            }
        }

        // A janela do sistema so nasce depois que o laco gira, entao posicao,
        // canto e fechamento automatico sao montados aqui, e nao na abertura.
        if let Some(menu) = open_menu.borrow_mut().as_mut() {
            menu.attach();
            // Mesma opacidade da janela principal, mas sem acrilico: o
            // cartao do menu tem piso opaco proprio, e o fundo do sistema
            // nunca apareceria por tras dele -- ligar o acrilico so custaria
            // composicao.
            #[cfg(windows)]
            {
                // Acrilico aqui, e nao nada: o menu da bandeja e exatamente a
                // "transient, light-dismiss surface" para a qual a documentacao
                // do Windows reserva o material.
                let (opacity, material) = state.borrow().window_effects();
                let backdrop = if material {
                    Backdrop::Acrylic
                } else {
                    Backdrop::None
                };
                ensure_window_effects(menu.window().window(), opacity, backdrop);
            }
        }

        // Clique fora fecha o menu; o aviso chega pelo subclass da janela.
        let dismissed = open_menu
            .borrow()
            .as_ref()
            .is_some_and(tray_menu::TrayMenu::dismiss_requested);
        if dismissed {
            *open_menu.borrow_mut() = None;
        }

        // Com o menu aberto ele e a unica superficie de reproducao visivel:
        // precisa acompanhar a faixa que entra sozinha no fim da anterior.
        if let Some(menu) = open_menu.borrow().as_ref() {
            state.borrow().push_to_tray_menu(menu.window());
        }
    });

    Some(timer)
}

/// Liga os itens do menu da bandeja as mesmas acoes da janela.
///
/// Toda acao fecha o menu, que e o que um menu faz. O fechamento e adiado para
/// o proximo giro do laco: descartar a janela de dentro do callback dela
/// mesma derrubaria o componente que ainda esta despachando o evento.
fn wire_tray_menu(
    menu: &tray_menu::TrayMenu,
    window: &ui::AppWindow,
    state: &Rc<RefCell<AppState>>,
    open_menu: &Rc<RefCell<Option<tray_menu::TrayMenu>>>,
) {
    let ui = menu.window();

    let close = {
        let open_menu = open_menu.clone();
        move || {
            let open_menu = open_menu.clone();
            // Disparo unico de zero: roda no proximo giro do laco, no mesmo
            // thread. `invoke_from_event_loop` exigiria `Send`, que um `Rc`
            // nao e -- e nem precisaria ser, o laco e este mesmo.
            slint::Timer::single_shot(Duration::ZERO, move || {
                *open_menu.borrow_mut() = None;
            });
        }
    };

    ui.on_dismiss({
        let close = close.clone();
        move || close()
    });

    ui.on_show_window({
        let weak = window.as_weak();
        let close = close.clone();
        move || {
            if let Some(window) = weak.upgrade() {
                if let Err(e) = window.show() {
                    tracing::error!(error = %e, "nao foi possivel reabrir a janela");
                }
            }
            close();
        }
    });

    ui.on_toggle_play({
        let state = state.clone();
        let weak = window.as_weak();
        move || {
            state.borrow_mut().toggle_play();
            if let Some(window) = weak.upgrade() {
                state.borrow().push_to_ui(&window);
            }
        }
    });

    ui.on_next({
        let state = state.clone();
        let weak = window.as_weak();
        move || {
            state.borrow_mut().next_track();
            if let Some(window) = weak.upgrade() {
                state.borrow().push_to_ui(&window);
            }
        }
    });

    ui.on_previous({
        let state = state.clone();
        let weak = window.as_weak();
        move || {
            state.borrow_mut().previous_track();
            if let Some(window) = weak.upgrade() {
                state.borrow().push_to_ui(&window);
            }
        }
    });

    ui.on_set_volume({
        let state = state.clone();
        let weak = window.as_weak();
        move |volume| {
            state.borrow_mut().set_volume(volume);
            if let Some(window) = weak.upgrade() {
                state.borrow().push_to_ui(&window);
            }
        }
    });

    ui.on_quit({
        let state = state.clone();
        move || {
            state.borrow().save_config();
            let _ = slint::quit_event_loop();
        }
    });
}

/// Mantem os controles de reproducao no preview da barra de tarefas em sincronia.
///
/// A integracao nasce depois de `show()`: antes disso o backend do Slint pode
/// ainda nao ter criado o HWND que o Windows exige para registrar os botoes.
#[cfg(windows)]
fn wire_taskbar(window: &ui::AppWindow, state: &Rc<std::cell::RefCell<AppState>>) -> slint::Timer {
    let weak = window.as_weak();
    let state = state.clone();
    let mut controls: Option<taskbar::TaskbarControls> = None;
    let mut creation_failed = false;
    // Mesmo temporizador que a barra de tarefas: os dois precisam da janela ja
    // criada, leem o mesmo estado e escrevem so quando ele muda. Um segundo
    // temporizador de 150 ms seria outro despertar do processo em repouso, e o
    // criterio do projeto e nao aparecer no perfil de quem esta jogando.
    let mut media: Option<smtc::MediaControls> = None;
    let mut media_failed = false;
    // Atalhos globais leem no mesmo giro: a thread deles dorme em
    // `GetMessageW`, e quem entrega o comando a interface e este temporizador.
    let hotkeys = hotkeys::GlobalHotkeys::start();

    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, tray::POLL_INTERVAL, move || {
        let _cronometro = quadros::Cronometro::new("barra de tarefas");
        let Some(window) = weak.upgrade() else { return };

        let tint = state.borrow().taskbar_tint();

        if controls.is_none() && !creation_failed {
            match taskbar::TaskbarControls::new(window.window(), tint) {
                Ok(created) => controls = Some(created),
                Err(error) => {
                    creation_failed = true;
                    tracing::warn!(%error, "controles da barra de tarefas indisponiveis");
                }
            }
        }

        let Some(controls) = controls.as_mut() else {
            return;
        };
        for command in controls.poll() {
            match command {
                taskbar::TaskbarCommand::TogglePlay => state.borrow_mut().toggle_play(),
                taskbar::TaskbarCommand::Next => state.borrow_mut().next_track(),
                taskbar::TaskbarCommand::Previous => state.borrow_mut().previous_track(),
            }
            state.borrow().push_to_ui(&window);
        }

        // Antes do `update`: trocar de tema invalida os HICON, e reenviar os
        // botoes com os antigos deixaria a barra apontando para icones mortos.
        controls.set_tint(tint);

        let (now_playing, playing) = state.borrow().tray_status();
        controls.update(now_playing.is_some(), playing);

        if media.is_none() && !media_failed {
            match smtc::MediaControls::new(window.window()) {
                Ok(created) => media = Some(created),
                Err(error) => {
                    media_failed = true;
                    tracing::warn!(%error, "painel de midia do Windows indisponivel");
                }
            }
        }

        if let Some(hotkeys) = hotkeys.as_ref() {
            for command in hotkeys.poll() {
                let mut s = state.borrow_mut();
                match command {
                    hotkeys::HotkeyCommand::TogglePlay => s.toggle_play(),
                    hotkeys::HotkeyCommand::Next => s.next_track(),
                    hotkeys::HotkeyCommand::Previous => s.previous_track(),
                    hotkeys::HotkeyCommand::VolumeUp => s.nudge_volume(0.05),
                    hotkeys::HotkeyCommand::VolumeDown => s.nudge_volume(-0.05),
                    hotkeys::HotkeyCommand::ToggleLike => {
                        let id = window.get_now_id();
                        if !id.is_empty() {
                            s.toggle_favorite(id.as_str());
                        }
                    }
                }
                drop(s);
                state.borrow().push_to_ui(&window);
            }
        }

        if let Some(media) = media.as_mut() {
            for command in media.poll() {
                match command {
                    smtc::MediaCommand::Play => state.borrow_mut().play(),
                    smtc::MediaCommand::Pause => state.borrow_mut().pause(),
                    smtc::MediaCommand::TogglePlay => state.borrow_mut().toggle_play(),
                    smtc::MediaCommand::Next => state.borrow_mut().next_track(),
                    smtc::MediaCommand::Previous => state.borrow_mut().previous_track(),
                }
                state.borrow().push_to_ui(&window);
            }

            let status = state.borrow().media_status();
            if let Err(error) = media.update(status.as_ref()) {
                // Nao desliga o painel: uma falha aqui costuma ser passageira
                // (o servico de midia reiniciando), e a proxima passagem
                // reescreve tudo, porque o estado guardado nao foi atualizado.
                tracing::warn!(%error, "painel de midia recusou a atualizacao");
            }
        }
    });

    timer
}

/// Tamanho a partir do qual o log da vez vira `morune.log.old`.
///
/// Teto explicito, dois arquivos, como o cache de capas: o log nunca cresce sem
/// limite, e a sessao anterior continua disponivel quando o defeito so aparece
/// na seguinte.
const LOG_MAX_BYTES: u64 = 4 * 1024 * 1024;

/// Liga o log em arquivo, e tambem no console quando ele existe.
///
/// **Por que arquivo e obrigatorio:** o build de release e
/// `windows_subsystem = "windows"`, ou seja, roda sem console nenhum. Tudo que
/// o `tracing` escrevia em `stdout` era descartado em silencio -- e um usuario
/// que ve "Sem conexão com o Spotify" nao tinha como saber o motivo, nem tinha
/// o que anexar a um relato de defeito. O caminho e
/// [`AppPaths::log_file`], que ja existia e nunca era usado.
///
/// Escrever e sincrono, sob trava. No nivel padrao (`info`) sao poucas linhas
/// por sessao; `MORUNE_LOG=debug` custa I/O e e opcional, para quem esta
/// investigando.
/// Manda todo panic para o log antes do processo morrer.
///
/// **Por que existe:** o release e `panic = "abort"` e
/// `windows_subsystem = "windows"`. Sem console, a mensagem que o Rust escreve
/// em `stderr` some, e o processo desaparece sem deixar uma linha sequer -- foi
/// exatamente o que aconteceu numa queda relatada e nao houve o que investigar.
/// O gancho roda antes do abort, entao a mensagem chega ao arquivo.
///
/// Precisa ser instalado depois de `init_logging`: antes dele nao ha para onde
/// escrever.
fn log_panics() {
    let anterior = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let local = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "local desconhecido".into());
        // O `Display` do `PanicHookInfo` ja traz a mensagem e a localizacao no
        // formato que o Rust usa no console; registrar os dois separados torna
        // o log pesquisavel por arquivo.
        tracing::error!(
            local = %local,
            thread = ?std::thread::current().name(),
            "panic: {info}"
        );
        anterior(info);
    }));
}

fn init_logging(paths: &morune_storage::AppPaths) {
    use tracing_subscriber::filter::EnvFilter;

    let filter = EnvFilter::try_from_env("MORUNE_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact();

    let (file, aviso) = open_log_file(paths);
    match file {
        // Sem cor: o arquivo e lido em editor de texto, e o codigo de escape
        // apareceria como lixo no meio da mensagem.
        Some(file) => builder
            .with_ansi(false)
            .with_writer(std::sync::Mutex::new(file))
            .init(),
        None => builder.init(),
    }

    // Emitido depois de `init` de proposito: e a primeira linha do arquivo
    // alternativo, e explica por que o principal ficou para tras.
    if let Some(aviso) = aviso {
        tracing::warn!("{aviso}");
    }
}

/// Abre o log para acrescimo, rodando o anterior quando passa do teto.
///
/// Falhar aqui **nao** pode impedir o aplicativo de abrir. Mas tambem nao pode
/// passar despercebido: em release nao existe console, entao um `None` aqui
/// apaga a sessao inteira do registro -- ela roda, faz tudo, e nao deixa uma
/// linha. Ja aconteceu, e transformou um travamento numa investigacao sem
/// nenhuma evidencia.
///
/// Por isso, quando o caminho principal nao abre -- normalmente porque outro
/// programa esta com ele travado, um antivirus ou um indexador --, a sessao vai
/// para um arquivo proprio com o numero do processo no nome. O segundo valor
/// devolvido e o aviso a registrar assim que o log existir.
fn open_log_file(paths: &morune_storage::AppPaths) -> (Option<std::fs::File>, Option<String>) {
    let path = paths.log_file();

    if std::fs::metadata(&path).is_ok_and(|m| m.len() >= LOG_MAX_BYTES) {
        let _ = std::fs::rename(&path, path.with_extension("log.old"));
    }

    let abrir = |path: &std::path::Path| {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
    };

    let erro = match abrir(&path) {
        Ok(file) => return (Some(file), None),
        Err(e) => e,
    };

    let alternativo = path.with_file_name(format!("morune-{}.log", std::process::id()));
    match abrir(&alternativo) {
        Ok(file) => (
            Some(file),
            Some(format!(
                "{} indisponivel ({erro}); esta sessao esta sendo registrada aqui",
                path.display()
            )),
        ),
        // Nem o alternativo abriu: disco cheio ou pasta sem permissao. Nao ha
        // para onde escrever o aviso, e a unica escolha que resta e seguir.
        Err(_) => (None, None),
    }
}

fn wire_callbacks(window: &ui::AppWindow, state: &Rc<std::cell::RefCell<AppState>>) {
    macro_rules! on {
        ($setter:ident, |$w:ident, $s:ident| $body:block) => {{
            let weak = window.as_weak();
            let state = state.clone();
            window.$setter(move || {
                let Some($w) = weak.upgrade() else { return };
                let mut $s = state.borrow_mut();
                $body
            });
        }};
        ($setter:ident, |$w:ident, $s:ident, $arg:ident : $ty:ty| $body:block) => {{
            let weak = window.as_weak();
            let state = state.clone();
            window.$setter(move |$arg: $ty| {
                let Some($w) = weak.upgrade() else { return };
                let mut $s = state.borrow_mut();
                $body
            });
        }};
    }

    on!(on_navigate, |w, s, page: i32| {
        s.navigate(page);
        s.push_to_ui(&w);
    });

    on!(on_toggle_sidebar, |w, s| {
        s.toggle_sidebar();
        // So o layout muda. Reaplicar o tema inteiro aqui -- fonte, imagem de
        // fundo, tintas -- era o atraso entre o clique e a barra recolher.
        s.apply_layout_to(&w);
    });

    on!(on_select_theme, |w, s, id: slint::SharedString| {
        s.select_theme(id.as_str());
        s.apply_theme_to(&w);
        s.push_to_ui(&w);
    });

    on!(on_import_theme, |w, s| {
        s.import_theme_via_dialog();
        s.apply_theme_to(&w);
        s.push_to_ui(&w);
    });

    on!(on_export_theme, |w, s, id: slint::SharedString| {
        s.export_theme_via_dialog(id.as_str());
        s.push_to_ui(&w);
    });

    on!(on_duplicate_theme, |w, s, id: slint::SharedString| {
        s.duplicate_theme(id.as_str());
        s.apply_theme_to(&w);
        s.push_to_ui(&w);
    });

    on!(on_reset_theme, |w, s| {
        s.reset_theme();
        s.apply_theme_to(&w);
        s.push_to_ui(&w);
    });

    on!(on_open_theme_folder, |w, s| {
        s.open_theme_folder();
        s.push_to_ui(&w);
    });

    on!(on_reload_theme, |w, s| {
        s.reload_theme();
        s.apply_theme_to(&w);
        s.push_to_ui(&w);
    });

    on!(on_cancel_login, |w, s| {
        s.cancel_login();
        s.push_to_ui(&w);
    });

    on!(on_copy_auth_url, |w, s| {
        s.copy_auth_url();
        s.push_to_ui(&w);
    });

    on!(on_set_bitrate, |w, s, code: i32| {
        s.set_bitrate(code);
        s.push_to_ui(&w);
    });

    on!(on_set_normalize, |w, s, on: bool| {
        s.set_normalize(on);
        s.push_to_ui(&w);
    });

    on!(on_set_output_device, |w, s, nome: slint::SharedString| {
        s.set_output_device(nome.as_str());
        s.push_to_ui(&w);
    });

    on!(on_check_update, |w, s| {
        s.check_for_update();
        s.push_to_ui(&w);
    });

    on!(on_download_update, |w, s| {
        s.download_update();
        s.push_to_ui(&w);
    });

    on!(on_open_releases_page, |w, s| {
        s.open_releases_page();
        s.push_to_ui(&w);
    });

    // Fora da macro `on!`: encerrar o laco enquanto o `RefCell` do estado esta
    // emprestado deixaria o `borrow_mut` vivo durante o desmonte da janela.
    {
        let weak = window.as_weak();
        let state = state.clone();
        window.on_install_update(move || {
            let Some(window) = weak.upgrade() else { return };
            let instalando = state.borrow_mut().install_update();
            state.borrow().push_to_ui(&window);
            if instalando {
                let _ = slint::quit_event_loop();
            }
        });
    }

    on!(on_search, |w, s, query: slint::SharedString| {
        s.search(query.as_str());
        s.push_to_ui(&w);
    });

    on!(on_dismiss_status, |w, s| {
        s.clear_status();
        s.push_to_ui(&w);
    });

    on!(on_undo_last, |w, s| {
        s.undo_last();
        s.apply_theme_to(&w);
        s.push_to_ui(&w);
    });

    on!(on_retry_last, |w, s| {
        s.retry_last();
        s.push_to_ui(&w);
    });

    on!(on_dismiss_undo, |w, s| {
        s.dismiss_recovery();
        s.clear_status();
        s.push_to_ui(&w);
    });

    // Filtrar nao vai a rede: a lista inteira ja esta em memoria, e responder
    // a cada tecla e o que faz o filtro parecer instantaneo.
    on!(on_filter_playlists, |w, s, texto: slint::SharedString| {
        s.set_playlist_filter(texto.as_str());
        s.push_to_ui(&w);
    });

    on!(on_toggle_pin_playlist, |w, s, tag: slint::SharedString| {
        s.toggle_pin_playlist(tag.as_str());
        s.push_to_ui(&w);
    });

    on!(on_detail_back, |w, s| {
        s.close_detail();
        s.push_to_ui(&w);
    });

    on!(on_close_queue, |w, s| {
        s.close_queue();
        s.push_to_ui(&w);
    });

    on!(on_toggle_mute, |w, s| {
        s.toggle_mute();
        s.push_playback(&w);
    });

    on!(on_nudge_volume, |w, s, delta: f32| {
        s.nudge_volume(delta);
        s.push_playback(&w);
    });

    on!(on_detail_play, |w, s| {
        s.play_detail();
        s.push_to_ui(&w);
    });

    on!(on_detail_filter, |w, s, texto: slint::SharedString| {
        s.set_detail_filter(texto.as_str());
        s.push_to_ui(&w);
    });

    on!(on_detail_sort_by, |w, s, criterio: i32| {
        s.set_detail_sort(criterio);
        s.push_to_ui(&w);
    });

    on!(on_detail_load_more, |w, s| {
        s.load_more_detail();
        s.push_to_ui(&w);
    });

    on!(on_detail_activate, |w, s, id: slint::SharedString| {
        s.activate_detail(id.as_str());
        s.push_to_ui(&w);
    });

    on!(on_play_track, |w, s, id: slint::SharedString| {
        s.play_track(id.as_str());
        s.push_to_ui(&w);
    });

    {
        let acoes = window.global::<ui::Acoes>();
        let weak = window.as_weak();
        let st = state.clone();
        acoes.on_copy_link(move |tag| {
            let Some(w) = weak.upgrade() else { return };
            let mut s = st.borrow_mut();
            s.copy_link(tag.as_str());
            s.push_to_ui(&w);
        });
        let weak = window.as_weak();
        let st = state.clone();
        acoes.on_toggle_hidden(move |tag| {
            let Some(w) = weak.upgrade() else { return };
            let mut s = st.borrow_mut();
            s.toggle_hidden(tag.as_str());
            s.push_to_ui(&w);
        });
        let weak = window.as_weak();
        let st = state.clone();
        acoes.on_add_to_playlist(move |playlist, faixa| {
            let Some(w) = weak.upgrade() else { return };
            let mut s = st.borrow_mut();
            s.add_track_to_playlist(playlist.as_str(), faixa.as_str());
            s.push_to_ui(&w);
        });
        let weak = window.as_weak();
        let st = state.clone();
        acoes.on_create_playlist(move |nome| {
            let Some(w) = weak.upgrade() else { return };
            let mut s = st.borrow_mut();
            s.create_playlist(nome.as_str());
            s.push_to_ui(&w);
        });
        let weak = window.as_weak();
        let st = state.clone();
        acoes.on_rename_playlist(move |tag, nome| {
            let Some(w) = weak.upgrade() else { return };
            let mut s = st.borrow_mut();
            s.rename_playlist(tag.as_str(), nome.as_str());
            s.push_to_ui(&w);
        });
        let weak = window.as_weak();
        let st = state.clone();
        acoes.on_delete_playlist(move |tag| {
            let Some(w) = weak.upgrade() else { return };
            let mut s = st.borrow_mut();
            s.delete_playlist(tag.as_str());
            s.push_to_ui(&w);
        });
    }

    on!(on_toggle_detail_save, |w, s| {
        s.toggle_detail_saved();
        s.push_to_ui(&w);
    });

    on!(on_set_sleep, |w, s, minutos: i32| {
        s.set_sleep_timer(minutos);
        s.push_to_ui(&w);
    });

    on!(on_clear_recent_searches, |w, s| {
        s.clear_recent_searches();
        s.push_to_ui(&w);
    });

    on!(on_toggle_hidden, |w, s, id: slint::SharedString| {
        s.toggle_hidden(id.as_str());
        s.push_to_ui(&w);
    });

    on!(on_toggle_favorite, |w, s, id: slint::SharedString| {
        s.toggle_favorite(id.as_str());
        s.push_to_ui(&w);
    });

    on!(on_queue_play_next, |w, s, id: slint::SharedString| {
        s.queue_play_next(id.as_str());
        s.push_to_ui(&w);
    });

    on!(on_queue_enqueue, |w, s, id: slint::SharedString| {
        s.queue_enqueue(id.as_str());
        s.push_to_ui(&w);
    });

    on!(on_queue_remove, |w, s, index: i32| {
        s.queue_remove(index);
        s.push_to_ui(&w);
    });

    on!(on_queue_play_manual, |w, s, index: i32| {
        s.queue_play_manual(index);
        s.push_to_ui(&w);
    });

    let weak = window.as_weak();
    let state_for_move = state.clone();
    window.on_queue_move(move |from, to| {
        let Some(window) = weak.upgrade() else { return };
        let mut state = state_for_move.borrow_mut();
        state.queue_move(from, to);
        state.push_to_ui(&window);
    });

    on!(on_queue_clear, |w, s| {
        s.queue_clear();
        s.push_to_ui(&w);
    });

    // Os controles da barra espelham so a barra. Reconstruir inicio, busca,
    // biblioteca e fila a cada clique -- e a cada quadro de arraste do volume
    // -- e o que fazia o botao responder devagar.
    on!(on_toggle_play, |w, s| {
        s.toggle_play();
        s.push_playback(&w);
    });

    // Trocar de faixa tambem move o marcador da linha que esta tocando, mas
    // isso pode chegar um tique depois: o `TrackChanged` do motor forca o
    // espelhamento completo em ate 100 ms. Segurar o clique ate reconstruir
    // inicio, busca, biblioteca e fila e o que se sentia como botao lento.
    on!(on_stop_playback, |w, s| {
        s.stop();
        s.push_playback(&w);
    });

    on!(on_next_track, |w, s| {
        s.next_track();
        s.push_playback(&w);
    });

    on!(on_previous_track, |w, s| {
        s.previous_track();
        s.push_playback(&w);
    });

    on!(on_seek, |w, s, position: f32| {
        s.seek(position);
        s.push_playback(&w);
    });

    on!(on_set_volume, |w, s, volume: f32| {
        s.set_volume(volume);
        s.push_playback(&w);
    });

    on!(on_toggle_shuffle, |w, s| {
        s.toggle_shuffle();
        s.push_playback(&w);
    });

    on!(on_cycle_repeat, |w, s| {
        s.cycle_repeat();
        s.push_playback(&w);
    });

    on!(on_login, |w, s| {
        s.login();
        s.push_to_ui(&w);
    });

    on!(on_logout, |w, s| {
        s.logout();
        s.push_to_ui(&w);
    });

    on!(on_set_close_to_tray, |w, s, on: bool| {
        s.set_close_to_tray(on);
        s.push_to_ui(&w);
    });

    on!(on_set_start_with_windows, |w, s, on: bool| {
        s.set_start_with_windows(on);
        s.push_to_ui(&w);
    });

    on!(on_set_autoplay, |w, s, on: bool| {
        s.set_autoplay(on);
        s.push_to_ui(&w);
    });

    // Aparencia. Cada um destes reprepara o fundo e reaplica o tema, porque
    // sao exatamente os ajustes que mudam o que a janela desenha.
    on!(on_choose_background, |w, s| {
        s.choose_background_via_dialog();
        s.apply_theme_to(&w);
        s.push_to_ui(&w);
    });

    on!(on_clear_background, |w, s| {
        s.clear_background();
        s.apply_theme_to(&w);
        s.push_to_ui(&w);
    });

    on!(on_set_background_fit, |w, s, v: i32| {
        s.set_background_fit(v);
        s.apply_theme_to(&w);
        s.push_to_ui(&w);
    });

    on!(on_set_background_opacity, |w, s, v: f32| {
        s.set_background_opacity(v);
        s.apply_theme_to(&w);
        s.push_to_ui(&w);
    });

    on!(on_set_background_tint, |w, s, v: f32| {
        s.set_background_tint(v);
        s.apply_theme_to(&w);
        s.push_to_ui(&w);
    });

    on!(on_set_background_blur, |w, s, v: f32| {
        s.set_background_blur(v);
        s.apply_theme_to(&w);
        s.push_to_ui(&w);
    });

    on!(on_set_glass_blur, |w, s, v: f32| {
        s.set_glass_blur(v);
        s.apply_theme_to(&w);
        s.push_to_ui(&w);
    });

    on!(on_set_font_scale, |w, s, v: f32| {
        s.set_font_scale(v);
        s.apply_theme_to(&w);
        s.push_to_ui(&w);
    });

    // Aplicada na hora, e nao no temporizador de meio segundo: arrastar um
    // slider e esperar a proxima batida parece um controle emperrado.
    on!(on_set_window_opacity, |w, s, v: f32| {
        s.set_window_opacity(v);
        #[cfg(windows)]
        {
            let (opacidade, material) = s.window_effects();
            w.set_native_backdrop_active(ensure_window_effects(
                w.window(),
                opacidade,
                backdrop_da_janela(material, s.tela_cheia_ativa()),
            ));
        }
        s.push_to_ui(&w);
    });

    on!(on_set_reduce_motion, |w, s, on: bool| {
        s.set_reduce_motion(on);
        s.apply_theme_to(&w);
        s.push_to_ui(&w);
    });
}
