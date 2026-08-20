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

// Sem console preto atras da janela no Windows. Em depuracao o console e util,
// entao a supressao vale so para builds de release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// A unica fronteira `unsafe` do aplicativo fica isolada em `taskbar.rs`: a API
// nativa exige COM, HWND e um callback Win32. Dentro dela, cada operacao ainda
// precisa declarar explicitamente o seu bloco inseguro.
#![deny(unsafe_op_in_unsafe_fn)]

mod artwork;
mod browse;
mod bundled;
#[cfg(windows)]
mod instance;
mod session;
#[cfg(feature = "snapshot")]
mod snapshot;
mod state;
mod startup;
#[cfg(windows)]
mod taskbar;
mod theme_bridge;
mod tray;

pub mod ui {
    slint::include_modules!();
}

use std::rc::Rc;
use std::time::Instant;

use slint::ComponentHandle;

use state::AppState;

fn main() -> anyhow::Result<()> {
    #[cfg(windows)]
    let Some(_single_instance) = instance::SingleInstance::acquire()? else {
        return Ok(());
    };

    let started = Instant::now();
    let started_with_windows = std::env::args_os().any(|arg| arg == "--startup");
    // Os caminhos vem antes do log porque e neles que o arquivo de log mora.
    // `ensure` e idempotente; `AppState::load` chama de novo logo abaixo.
    let paths = morune_storage::AppPaths::discover();
    let _ = paths.ensure();
    init_logging(&paths);
    tracing::info!(versao = env!("CARGO_PKG_VERSION"), log = %paths.log_file().display(), "Morune iniciando");

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
        window
            .window()
            .set_size(slint::LogicalSize::new(width, height));
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
    let _window_state_poll = wire_window_state(&window, &state);
    wire_close_behavior(&window, &state, tray.is_some());

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
    #[cfg(windows)]
    let _taskbar_poll = wire_taskbar(&window, &state);
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

/// Observa tamanho/maximizacao com baixa frequencia. Isso tambem cobre resize
/// pelo teclado, snap layouts e restauracao do Windows, nao apenas arraste.
fn wire_window_state(
    window: &ui::AppWindow,
    state: &Rc<std::cell::RefCell<AppState>>,
) -> slint::Timer {
    let weak = window.as_weak();
    let state = state.clone();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(500),
        move || {
            let Some(window) = weak.upgrade() else { return };
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
                        "A janela foi escondida na bandeja do Windows. Use o icone do Morune para abrir novamente ou sair.",
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

    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, BACKEND_POLL, move || {
        let Some(window) = weak.upgrade() else { return };

        if !started {
            started = true;
            state.borrow_mut().restore_session();
        }

        if state.borrow_mut().poll_backend() {
            state.borrow().push_to_ui(&window);
        }
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

    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, tray::POLL_INTERVAL, move || {
        let Some(window) = weak.upgrade() else { return };

        for command in tray.poll() {
            match command {
                tray::TrayCommand::Show => {
                    if let Err(e) = window.show() {
                        tracing::error!(error = %e, "nao foi possivel reabrir a janela");
                    }
                }
                tray::TrayCommand::TogglePlay => state.borrow_mut().toggle_play(),
                tray::TrayCommand::Next => state.borrow_mut().next_track(),
                tray::TrayCommand::Previous => state.borrow_mut().previous_track(),
                tray::TrayCommand::Quit => {
                    state.borrow().save_config();
                    let _ = slint::quit_event_loop();
                    return;
                }
            }
            state.borrow().push_to_ui(&window);
        }

        // O menu mostra a faixa atual mesmo com a janela fechada -- e a unica
        // informacao de reproducao visivel nesse estado.
        let (now_playing, playing) = state.borrow().tray_status();
        tray.update(now_playing.as_deref(), playing);
    });

    Some(timer)
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

    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, tray::POLL_INTERVAL, move || {
        let Some(window) = weak.upgrade() else { return };

        if controls.is_none() && !creation_failed {
            match taskbar::TaskbarControls::new(window.window()) {
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

        let (now_playing, playing) = state.borrow().tray_status();
        controls.update(now_playing.is_some(), playing);
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
/// que ve "Sem conexao com o Spotify" nao tinha como saber o motivo, nem tinha
/// o que anexar a um relato de defeito. O caminho e
/// [`AppPaths::log_file`], que ja existia e nunca era usado.
///
/// Escrever e sincrono, sob trava. No nivel padrao (`info`) sao poucas linhas
/// por sessao; `MORUNE_LOG=debug` custa I/O e e opcional, para quem esta
/// investigando.
fn init_logging(paths: &morune_storage::AppPaths) {
    use tracing_subscriber::filter::EnvFilter;

    let filter = EnvFilter::try_from_env("MORUNE_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact();

    match open_log_file(paths) {
        // Sem cor: o arquivo e lido em editor de texto, e o codigo de escape
        // apareceria como lixo no meio da mensagem.
        Some(file) => builder
            .with_ansi(false)
            .with_writer(std::sync::Mutex::new(file))
            .init(),
        None => builder.init(),
    }
}

/// Abre o log para acrescimo, rodando o anterior quando passa do teto.
///
/// Devolve `None` quando o arquivo nao pode ser aberto -- disco cheio, pasta
/// sem permissao. Falhar aqui **nao** pode impedir o aplicativo de abrir: fica
/// so o console, que em release nao existe, e o resto segue.
fn open_log_file(paths: &morune_storage::AppPaths) -> Option<std::fs::File> {
    let path = paths.log_file();

    if std::fs::metadata(&path).is_ok_and(|m| m.len() >= LOG_MAX_BYTES) {
        let _ = std::fs::rename(&path, path.with_extension("log.old"));
    }

    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .ok()
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
        s.apply_theme_to(&w);
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

    on!(on_detail_back, |w, s| {
        s.close_detail();
        s.push_to_ui(&w);
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

    on!(on_detail_activate, |w, s, id: slint::SharedString| {
        s.activate_detail(id.as_str());
        s.push_to_ui(&w);
    });

    on!(on_play_track, |w, s, id: slint::SharedString| {
        s.play_track(id.as_str());
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

    on!(on_toggle_play, |w, s| {
        s.toggle_play();
        s.push_to_ui(&w);
    });

    on!(on_next_track, |w, s| {
        s.next_track();
        s.push_to_ui(&w);
    });

    on!(on_previous_track, |w, s| {
        s.previous_track();
        s.push_to_ui(&w);
    });

    on!(on_seek, |w, s, position: f32| {
        s.seek(position);
        s.push_to_ui(&w);
    });

    on!(on_set_volume, |w, s, volume: f32| {
        s.set_volume(volume);
        s.push_to_ui(&w);
    });

    on!(on_toggle_shuffle, |w, s| {
        s.toggle_shuffle();
        s.push_to_ui(&w);
    });

    on!(on_cycle_repeat, |w, s| {
        s.cycle_repeat();
        s.push_to_ui(&w);
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
}
