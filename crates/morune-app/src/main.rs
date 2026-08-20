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
// `deny` e nao `forbid`: o codigo que o Slint gera para a interface contem
// `allow(unsafe_code)` em trechos proprios, e `forbid` nao pode ser suspenso
// nem pelo modulo gerado. Nosso codigo continua sem `unsafe`.
#![deny(unsafe_code)]

mod artwork;
mod browse;
mod bundled;
mod session;
#[cfg(feature = "snapshot")]
mod snapshot;
mod state;
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
    let started = Instant::now();
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
    let _tray_poll = wire_tray(&window, &state, tray.clone());
    let _backend_poll = wire_backend(&window, &state);
    wire_close_behavior(&window, &state, tray.is_some());

    // Medida real do caminho critico, comparavel entre execucoes. Aparece no
    // log sempre e na sobreposicao de performance no Developer Mode.
    let ready = started.elapsed();
    tracing::info!(ms = ready.as_millis(), "interface pronta");
    window.set_perf_text(
        format!("startup {} ms | tema {}", ready.as_millis(), state.borrow().theme_id()).into(),
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
        slint::Timer::single_shot(std::time::Duration::from_millis(600), move || {
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
    slint::run_event_loop_until_quit()?;

    state.borrow().save_config();
    Ok(())
}

/// Faz o fechamento da janela esconder em vez de encerrar, quando o usuario
/// pediu isso e ha bandeja para trazer o aplicativo de volta.
fn wire_close_behavior(
    window: &ui::AppWindow,
    state: &Rc<std::cell::RefCell<AppState>>,
    has_tray: bool,
) {
    let state = state.clone();
    window.window().on_close_requested(move || {
        let keep_running = has_tray && state.borrow().close_to_tray();
        if keep_running {
            tracing::info!("janela escondida na bandeja; reproducao continua");
        } else {
            state.borrow().save_config();
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
fn wire_backend(
    window: &ui::AppWindow,
    state: &Rc<std::cell::RefCell<AppState>>,
) -> slint::Timer {
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
    let builder = tracing_subscriber::fmt().with_env_filter(filter).with_target(false).compact();

    match open_log_file(paths) {
        // Sem cor: o arquivo e lido em editor de texto, e o codigo de escape
        // apareceria como lixo no meio da mensagem.
        Some(file) => builder.with_ansi(false).with_writer(std::sync::Mutex::new(file)).init(),
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

    std::fs::OpenOptions::new().create(true).append(true).open(&path).ok()
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
}
