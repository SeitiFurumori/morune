//! Morune com a tela em web. O motor (`AppState`) e o mesmo do app em Slint;
//! aqui ele roda na thread principal do Tauri, e a tela recebe o estado em JSON.
//!
//! Regras de segundo plano (ADR-0007):
//! - na bandeja a tela e destruida e sobra so o motor;
//! - minimizada, a tela e suspensa;
//! - com jogo em tela cheia, o movimento vai a zero (`set_tela_cheia`).
// Sem terminal tambem na versao de teste: o registro vai para o morune.log.
#![windows_subsystem = "windows"]
// O motor veio do app em Slint, que tem metodos que esta tela ainda nao usa.
#![allow(dead_code)]

mod artwork;
mod browse;
mod bundled;
mod capas;
mod clipboard;
mod crash;
mod hotkeys;
mod instance;
mod material;
mod notify;
mod oculta;
mod ordem_curtidas;
mod registro;
mod retomar;
mod session;
mod smtc;
mod startup;
mod state;
mod taskbar;
mod tela_cheia;
mod tema;
mod tint;
mod ui;
mod update;
mod vista_tipos;

use std::cell::RefCell;
use std::time::{Duration, Instant};

use serde_json::{Map, Value};
use state::AppState;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

/// Intervalo do giro do motor: ler eventos do Spotify, do audio e das teclas.
const GIRO: Duration = Duration::from_millis(100);
/// A barra de progresso anda neste ritmo enquanto toca.
const PROGRESSO: Duration = Duration::from_millis(250);
/// A conferencia de jogo em tela cheia.
const VIGIA_TELA_CHEIA: Duration = Duration::from_secs(1);

struct Motor {
    state: AppState,
    /// O "estado da janela" que o motor escreve com os `set_*` de sempre.
    janela: ui::AppWindow,
    /// O ultimo estado mandado para a tela, campo a campo. Vazio quando nao ha tela.
    enviado: Map<String, Value>,
    tema_enviado: Value,
    hotkeys: Option<hotkeys::GlobalHotkeys>,
    oculta: Option<oculta::JanelaOculta>,
    media: Option<smtc::MediaControls>,
    notificador: Option<notify::TrackNotifier>,
    /// Botoes anterior/tocar/proxima na miniatura da barra de tarefas. Vivem
    /// com a janela: fechar para a bandeja os derruba, reabrir os recria.
    barra: Option<taskbar::TaskbarControls>,
    barra_falhou: bool,
    /// O `acrylic` do tema da ultima vez que o material foi aplicado.
    material_pedido: Option<bool>,
    /// O acrilico ficou ligado de fato (o tema pede e o Windows deixa).
    vidro_ativo: bool,
    /// Tamanho e "maximizada" de antes do mini player. `Some` = mini aberto.
    mini_antes: Option<(f64, f64, bool)>,
    ultimo_progresso: Instant,
    ultima_tela_cheia: Instant,
}

thread_local! {
    static MOTOR: RefCell<Option<Motor>> = const { RefCell::new(None) };
}

/// Mexer na janela de dentro daqui (tamanho, maximizar) faz o Windows chamar
/// o tratador de eventos na hora, que pede o motor de novo. Esse segundo
/// pedido e recusado em vez de entrar em panico e travar a tela; quem mexe na
/// janela deve faze-lo fora, com `run_on_main_thread`.
fn com_motor<R>(f: impl FnOnce(&mut Motor) -> R) -> Option<R> {
    MOTOR.with(|m| match m.try_borrow_mut() {
        Ok(mut m) => m.as_mut().map(f),
        Err(_) => {
            tracing::warn!("motor pedido de novo enquanto ocupado; pedido ignorado");
            None
        }
    })
}

fn main() -> anyhow::Result<()> {
    #[cfg(windows)]
    let Some(_instancia) = instance::SingleInstance::acquire()? else {
        return Ok(());
    };

    let iniciou_com_windows = std::env::args_os().any(|arg| arg == "--startup");
    let paths = morune_storage::AppPaths::discover();
    let _ = paths.ensure();
    registro::init_logging(&paths);
    registro::log_panics();
    crash::instalar(paths.data_dir());
    tracing::info!(versao = env!("MORUNE_RELEASE"), log = %paths.log_file().display(), "Morune iniciando (tela web)");

    capas::permitir(vec![
        paths.cache_dir().to_path_buf(),
        paths.data_dir().to_path_buf(),
        paths.themes_dir().to_path_buf(),
        paths.config_dir().to_path_buf(),
    ]);

    tauri::Builder::default()
        .register_uri_scheme_protocol("capa", |_ctx, req| servir_capa(req))
        .invoke_handler(tauri::generate_handler![acao, estado_inicial, pronta])
        .setup(move |app| {
            let mut state = AppState::load();
            state.restore_session();
            let motor = Motor {
                state,
                janela: ui::AppWindow::default(),
                enviado: Map::new(),
                tema_enviado: Value::Null,
                hotkeys: hotkeys::GlobalHotkeys::start(),
                oculta: oculta::JanelaOculta::criar(),
                media: None,
                notificador: None,
                barra: None,
                barra_falhou: false,
                material_pedido: None,
                vidro_ativo: false,
                mini_antes: None,
                ultimo_progresso: Instant::now(),
                ultima_tela_cheia: Instant::now(),
            };
            MOTOR.with(|m| *m.borrow_mut() = Some(motor));
            com_motor(|m| {
                if let Some(janela) = m.oculta.as_ref() {
                    match smtc::MediaControls::new(janela) {
                        Ok(media) => m.media = Some(media),
                        Err(error) => tracing::warn!(%error, "painel de midia do Windows indisponivel"),
                    }
                }
                match notify::TrackNotifier::new() {
                    Ok(n) => m.notificador = Some(n),
                    Err(error) => tracing::warn!(%error, "notificacoes do Windows indisponiveis"),
                }
            });

            criar_bandeja(app.handle())?;
            if !iniciou_com_windows {
                abrir_janela(app.handle());
            }

            // O giro do motor: uma thread so marca o tempo; o trabalho roda na
            // thread principal, onde o motor mora.
            let handle = app.handle().clone();
            std::thread::Builder::new()
                .name("giro".into())
                .spawn(move || loop {
                    std::thread::sleep(GIRO);
                    let h = handle.clone();
                    if handle.run_on_main_thread(move || giro(&h)).is_err() {
                        break;
                    }
                })?;
            Ok(())
        })
        .build(tauri::generate_context!())?
        .run(|app, event| match event {
            tauri::RunEvent::ExitRequested { api, code, .. } => {
                // Fechar a ultima janela nao encerra: o motor fica na bandeja.
                if code.is_none() {
                    api.prevent_exit();
                }
            }
            tauri::RunEvent::Exit => {
                com_motor(|m| {
                    m.state.gravar_retomada(true);
                    m.state.save_config();
                });
                let _ = app;
            }
            _ => {}
        });
    Ok(())
}

// ---- tela ----

fn abrir_janela(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let (largura, altura, maximizada) =
        com_motor(|m| m.state.tamanho_inicial()).unwrap_or((1280.0, 800.0, false));
    let janela = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("Morune")
        .inner_size(largura as f64, altura as f64)
        .min_inner_size(720.0, 480.0)
        .maximized(maximizada)
        // Sempre transparente: o tema decide se o que esta atras aparece
        // (acrilico) ou se a tela pinta o fundo inteiro. Ver `material.rs`.
        .transparent(true)
        .build();
    let janela = match janela {
        Ok(j) => j,
        Err(e) => {
            tracing::error!(error = %e, "nao foi possivel abrir a janela");
            return;
        }
    };
    let app2 = app.clone();
    janela.on_window_event(move |evento| tratar_evento_janela(&app2, evento));
    com_motor(|m| {
        // A tela nova pede o estado inteiro ao carregar (`estado_inicial`).
        m.enviado.clear();
        // Janela nova nasce sem efeito: o material e reaplicado.
        m.material_pedido = None;
    });
}

/// O tema para a tela, com o material da janela ja resolvido.
///
/// O acrilico e aplicado aqui, quando o tema muda, e a tela recebe
/// `vidro_ativo` para saber se pinta o fundo solido ou deixa ver atras.
fn tema_com_material(app: &AppHandle, m: &mut Motor) -> Value {
    let mut tema = m.state.tema_json();
    let pede = tema
        .pointer("/spec/effects/acrylic")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if m.material_pedido != Some(pede) {
        if let Some(w) = app.get_webview_window("main") {
            m.vidro_ativo = material::aplicar(&w, pede);
            m.material_pedido = Some(pede);
        }
    }
    if let Value::Object(mapa) = &mut tema {
        mapa.insert("vidro_ativo".into(), Value::Bool(m.vidro_ativo));
    }
    tema
}

fn tratar_evento_janela(app: &AppHandle, evento: &WindowEvent) {
    match evento {
        WindowEvent::CloseRequested { api, .. } => {
            let para_bandeja = com_motor(|m| m.state.close_to_tray()).unwrap_or(true);
            com_motor(|m| {
                m.state.save_config();
                // A subclasse sai enquanto a janela ainda existe.
                m.barra = None;
                m.barra_falhou = false;
            });
            if !para_bandeja {
                api.prevent_close();
                app.exit(0);
            }
            // Para a bandeja: a janela fecha de verdade e a tela e destruida.
        }
        WindowEvent::Destroyed => {
            com_motor(|m| {
                m.enviado.clear();
                m.tema_enviado = Value::Null;
                // A janela volta sempre no tamanho normal.
                m.mini_antes = None;
                m.janela.set_mini_player(false);
                m.janela.set_mini_fixo(false);
            });
            tracing::info!("tela destruida; motor segue na bandeja");
        }
        WindowEvent::Resized(_) => {
            if let Some(w) = app.get_webview_window("main") {
                let minimizada = w.is_minimized().unwrap_or(false);
                suspender_tela(&w, minimizada);
                if !minimizada {
                    let maximizada = w.is_maximized().unwrap_or(false);
                    let escala = w.scale_factor().unwrap_or(1.0);
                    if let Ok(t) = w.inner_size() {
                        let t = t.to_logical::<f32>(escala);
                        // O tamanho do mini player nao e o tamanho da janela.
                        com_motor(|m| {
                            if m.mini_antes.is_none() {
                                m.state.remember_window_state(t.width, t.height, maximizada)
                            }
                        });
                    }
                }
            }
        }
        _ => {}
    }
}

/// Minimizada, a tela para de rodar script e de desenhar.
fn suspender_tela(w: &tauri::WebviewWindow, suspender: bool) {
    let _ = w.emit("visibilidade", !suspender);
    #[cfg(windows)]
    {
        let _ = w.with_webview(move |webview| {
            use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2_3, ICoreWebView2Controller};
            use windows_core_webview::Interface;
            // SAFETY: chamadas COM no controlador vivo, na thread da tela.
            unsafe {
                let controller: ICoreWebView2Controller = webview.controller();
                let _ = controller.SetIsVisible(!suspender);
                if suspender {
                    if let Ok(core) = controller.CoreWebView2() {
                        if let Ok(core3) = core.cast::<ICoreWebView2_3>() {
                            let _ = core3.TrySuspend(None);
                        }
                    }
                }
            }
        });
    }
}

fn criar_bandeja(app: &AppHandle) -> tauri::Result<()> {
    TrayIconBuilder::with_id("morune")
        .icon(app.default_window_icon().cloned().expect("icone embutido"))
        .tooltip("Morune")
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button, button_state: MouseButtonState::Up, rect, .. } = e {
                match button {
                    MouseButton::Left => abrir_janela(tray.app_handle()),
                    MouseButton::Right => abrir_menu_bandeja(tray.app_handle(), rect),
                    _ => {}
                }
            }
        })
        .build(app)?;
    Ok(())
}

/// Largura do menu da bandeja, em pixels logicos.
const MENU_LARGURA: f64 = 272.0;
/// Distancia entre o menu e o icone, e entre o menu e a borda da tela.
const MENU_MARGEM: f64 = 8.0;

/// O menu proprio da bandeja: o do Windows nao aceita cor, fonte nem forma.
///
/// A janela so existe enquanto o menu esta aberto -- uma segunda tela viva em
/// repouso pesaria na maquina de quem esta jogando. Fecha ao perder o foco.
fn abrir_menu_bandeja(app: &AppHandle, icone: tauri::Rect) {
    if let Some(w) = app.get_webview_window("bandeja") {
        let _ = w.close();
        return;
    }
    let com_faixa = com_motor(|m| m.state.tray_status().0.is_some()).unwrap_or(false);
    let altura = if com_faixa { 240.0 } else { 184.0 };

    // A janela nasce escondida, vai para o lugar e so entao aparece.
    let janela = WebviewWindowBuilder::new(app, "bandeja", WebviewUrl::App("index.html#bandeja".into()))
        .title("Morune")
        .inner_size(MENU_LARGURA, altura)
        .decorations(false)
        .resizable(false)
        // Com a sombra do Windows, a borda invisivel de redimensionar aparece
        // como faixa branca de 5 px nos lados e embaixo.
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        // Opaca, a janela pintava de branco a moldura em volta da tela.
        .transparent(true)
        .visible(false)
        .build();
    let janela = match janela {
        Ok(j) => j,
        Err(e) => {
            tracing::error!(error = %e, "nao foi possivel abrir o menu da bandeja");
            return;
        }
    };

    // Acima do icone, centrado nele e dentro da area util do monitor; quando
    // nao cabe acima (barra de tarefas no alto), cai para baixo dele.
    let escala = janela.scale_factor().unwrap_or(1.0);
    let pos = icone.position.to_physical::<f64>(escala);
    let tam = icone.size.to_physical::<f64>(escala);
    let (largura, alto, margem) = (MENU_LARGURA * escala, altura * escala, MENU_MARGEM * escala);
    let mut x = pos.x + tam.width / 2.0 - largura / 2.0;
    let mut y = pos.y - alto - margem;
    if let Ok(Some(monitor)) = app.monitor_from_point(pos.x, pos.y) {
        let area = monitor.work_area();
        let (ax, ay) = (area.position.x as f64, area.position.y as f64);
        let (aw, ah) = (area.size.width as f64, area.size.height as f64);
        if y < ay + margem {
            y = pos.y + tam.height + margem;
        }
        x = x.clamp(ax + margem, ax + aw - largura - margem);
        y = y.clamp(ay + margem, ay + ah - alto - margem);
    }
    let _ = janela.set_position(tauri::PhysicalPosition::new(x.round() as i32, y.round() as i32));
    material::cantos_redondos(&janela);
    let _ = janela.show();
    let _ = janela.set_focus();

    janela.on_window_event({
        let app = app.clone();
        move |evento| {
            // Ao aparecer, a janela ganha o foco, perde por um instante e
            // ganha de novo. Fechar na primeira perda fechava o menu antes de
            // ele ser visto: so fecha se o foco nao voltou logo depois.
            if let WindowEvent::Focused(false) = evento {
                let app = app.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(150));
                    let app2 = app.clone();
                    let _ = app.run_on_main_thread(move || {
                        if let Some(w) = app2.get_webview_window("bandeja") {
                            if !w.is_focused().unwrap_or(false) {
                                let _ = w.close();
                            }
                        }
                    });
                });
            }
        }
    });
}

/// Roda `f` na linha principal depois que o motor for solto. Chamado da
/// propria linha principal, `run_on_main_thread` executaria na hora; pedido de
/// outra linha, entra na fila.
fn depois(w: &tauri::WebviewWindow, f: impl FnOnce(tauri::WebviewWindow) + Send + 'static) {
    let w = w.clone();
    std::thread::spawn(move || {
        let w2 = w.clone();
        let _ = w.run_on_main_thread(move || f(w2));
    });
}

/// Tamanho do mini player, em pixels logicos.
const MINI: (f64, f64) = (340.0, 420.0);

/// Encolhe a janela no mini player ou devolve o tamanho de antes, como o
/// botao de mini player do YouTube. A mesma janela, so outro tamanho e sem
/// barra de titulo: nada de segunda tela viva.
fn alternar_mini(app: &AppHandle, m: &mut Motor) {
    let Some(w) = app.get_webview_window("main") else {
        return;
    };
    match m.mini_antes.take() {
        None => {
            let escala = w.scale_factor().unwrap_or(1.0);
            let antes = w.inner_size().map(|t| t.to_logical::<f64>(escala)).ok();
            let maximizada = w.is_maximized().unwrap_or(false);
            let (largura, altura) = antes.map(|t| (t.width, t.height)).unwrap_or((1280.0, 800.0));
            m.mini_antes = Some((largura, altura, maximizada));
            m.janela.set_mini_player(true);
            // Fora do motor: ver `com_motor`.
            depois(&w, |w2| {
                let _ = w2.unmaximize();
                let _ = w2.set_decorations(false);
                let _ = w2.set_min_size(Some(tauri::LogicalSize::new(MINI.0, MINI.1)));
                let _ = w2.set_size(tauri::LogicalSize::new(MINI.0, MINI.1));
                let _ = w2.set_resizable(false);
                material::cantos_redondos(&w2);
            });
        }
        Some((largura, altura, maximizada)) => {
            m.janela.set_mini_player(false);
            m.janela.set_mini_fixo(false);
            depois(&w, move |w2| {
                let _ = w2.set_always_on_top(false);
                let _ = w2.set_resizable(true);
                let _ = w2.set_decorations(true);
                let _ = w2.set_min_size(Some(tauri::LogicalSize::new(720.0, 480.0)));
                let _ = w2.set_size(tauri::LogicalSize::new(largura, altura));
                if maximizada {
                    let _ = w2.maximize();
                }
            });
        }
    }
}

fn fechar_menu_bandeja(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("bandeja") {
        let _ = w.close();
    }
}

// ---- giro do motor ----

fn giro(app: &AppHandle) {
    com_motor(|m| {
        let mut mudou = m.state.poll_backend();
        m.state.gravar_retomada(false);

        if let Some(hk) = m.hotkeys.as_ref() {
            for c in hk.poll() {
                let s = &mut m.state;
                match c {
                    hotkeys::HotkeyCommand::TogglePlay => s.toggle_play(),
                    hotkeys::HotkeyCommand::Next => s.next_track(),
                    hotkeys::HotkeyCommand::Previous => s.previous_track(),
                    hotkeys::HotkeyCommand::VolumeUp => s.nudge_volume(0.05),
                    hotkeys::HotkeyCommand::VolumeDown => s.nudge_volume(-0.05),
                    hotkeys::HotkeyCommand::ToggleLike => {
                        let id = m.janela.get_now_id();
                        if !id.is_empty() {
                            s.toggle_favorite(&id);
                        }
                    }
                }
                mudou = true;
            }
        }

        if let Some(media) = m.media.as_mut() {
            for c in media.poll() {
                let s = &mut m.state;
                match c {
                    smtc::MediaCommand::Play => s.play(),
                    smtc::MediaCommand::Pause => s.pause(),
                    smtc::MediaCommand::TogglePlay => s.toggle_play(),
                    smtc::MediaCommand::Next => s.next_track(),
                    smtc::MediaCommand::Previous => s.previous_track(),
                }
                mudou = true;
            }
            let status = m.state.media_status();
            if let Err(error) = media.update(status.as_ref()) {
                tracing::warn!(%error, "painel de midia recusou a atualizacao");
            }
            if let Some(n) = m.notificador.as_mut() {
                let ligada = m.state.track_notifications();
                match app.get_webview_window("main") {
                    Some(w) => n.update(status.as_ref(), ligada, &w),
                    None => {
                        if let Some(o) = m.oculta.as_ref() {
                            n.update(status.as_ref(), ligada, o);
                        }
                    }
                }
            }
        }

        if let Some(w) = app.get_webview_window("main") {
            let tint = m.state.taskbar_tint();
            if m.barra.is_none() && !m.barra_falhou {
                match taskbar::TaskbarControls::new(&w, tint) {
                    Ok(b) => m.barra = Some(b),
                    Err(error) => {
                        m.barra_falhou = true;
                        tracing::warn!(%error, "controles da barra de tarefas indisponiveis");
                    }
                }
            }
            if let Some(barra) = m.barra.as_mut() {
                for c in barra.poll() {
                    let s = &mut m.state;
                    match c {
                        taskbar::TaskbarCommand::TogglePlay => s.toggle_play(),
                        taskbar::TaskbarCommand::Next => s.next_track(),
                        taskbar::TaskbarCommand::Previous => s.previous_track(),
                    }
                    mudou = true;
                }
                // Antes do `update`: trocar de tema invalida os icones antigos.
                barra.set_tint(tint);
                let (agora, tocando) = m.state.tray_status();
                barra.update(agora.is_some(), tocando);
            }
        }

        if m.ultima_tela_cheia.elapsed() >= VIGIA_TELA_CHEIA {
            m.ultima_tela_cheia = Instant::now();
            let cheia = tela_cheia::app_em_tela_cheia();
            if m.state.set_tela_cheia(cheia) {
                mudou = true;
                let _ = app.emit("tela_cheia", cheia);
            }
        }

        let progresso = m.state.is_playing() && m.ultimo_progresso.elapsed() >= PROGRESSO;
        if progresso {
            m.ultimo_progresso = Instant::now();
        }
        // O brilho que respira: o pulso vai para a tela a cada giro, mas so com
        // a janela a vista. Minimizada ou na bandeja, ninguem le.
        let a_vista = app
            .get_webview_window("main")
            .is_some_and(|w| !w.is_minimized().unwrap_or(true) && w.is_visible().unwrap_or(false));
        let pulso = if a_vista && m.state.respirando() {
            (m.state.pulso() * 20.0).round() / 20.0
        } else {
            0.0
        };
        let respira = (pulso - m.janela.estado.borrow().nivel).abs() > f32::EPSILON;
        if respira {
            m.janela.set_nivel(pulso);
        }
        if mudou || progresso || respira {
            enviar(app, m, mudou);
        }
    });
}

/// Manda para a tela so os campos que mudaram. Sem tela, nao monta nada.
///
/// O evento vai para todas as janelas: a principal e, quando aberto, o menu da
/// bandeja, que mostra a mesma faixa e o mesmo volume.
fn enviar(app: &AppHandle, m: &mut Motor, tudo: bool) {
    if app.get_webview_window("main").is_none() && app.get_webview_window("bandeja").is_none() {
        return;
    }
    let w = app;
    if tudo {
        m.state.push_to_ui(&m.janela);
    }
    m.state.push_playback(&m.janela);
    let Ok(Value::Object(atual)) = serde_json::to_value(&*m.janela.estado.borrow()) else {
        return;
    };
    #[cfg(debug_assertions)]
    if let Some(destino) = std::env::var_os("MORUNE_WEB_DUMP") {
        let _ = std::fs::write(destino, serde_json::json!({ "estado": &atual, "tema": m.state.tema_json() }).to_string());
    }
    let mut diferenca = Map::new();
    for (k, v) in atual {
        if m.enviado.get(&k) != Some(&v) {
            m.enviado.insert(k.clone(), v.clone());
            diferenca.insert(k, v);
        }
    }
    if !diferenca.is_empty() {
        let _ = w.emit("estado", Value::Object(diferenca));
    }
    let tema = tema_com_material(app, m);
    if tema != m.tema_enviado {
        m.tema_enviado = tema.clone();
        let _ = w.emit("tema", tema);
    }
}

// ---- comandos da tela ----

#[tauri::command]
fn estado_inicial(app: AppHandle) -> Value {
    com_motor(|m| {
        m.state.push_to_ui(&m.janela);
        m.state.push_playback(&m.janela);
        let estado = serde_json::to_value(&*m.janela.estado.borrow()).unwrap_or(Value::Null);
        if let Value::Object(mapa) = &estado {
            m.enviado = mapa.clone();
        }
        let tema = tema_com_material(&app, m);
        m.tema_enviado = tema.clone();
        tracing::info!(campos = m.enviado.len(), "tela web carregou o estado inicial");
        // So no build de desenvolvimento: copia do estado para conferir a tela fora do app.
        #[cfg(debug_assertions)]
        if let Some(destino) = std::env::var_os("MORUNE_WEB_DUMP") {
            let _ = std::fs::write(destino, serde_json::json!({ "estado": &estado, "tema": &tema }).to_string());
        }
        serde_json::json!({ "estado": estado, "tema": tema, "acoes": ui::ACOES })
    })
    .unwrap_or(Value::Null)
}

/// A tela terminou de montar: confere o tamanho e libera a abertura.
#[tauri::command]
fn pronta() {}

#[tauri::command]
fn acao(app: AppHandle, nome: String, args: Vec<Value>) -> Result<(), String> {
    let resultado = com_motor(|m| {
        let r = executar(&app, m, &nome, &args);
        enviar(&app, m, true);
        r
    });
    let resultado = resultado.unwrap_or_else(|| Err("motor indisponivel".into()));
    if let Err(erro) = &resultado {
        tracing::warn!(%erro, "acao da tela recusada");
    }
    resultado
}

fn texto(args: &[Value], i: usize) -> String {
    args.get(i).and_then(Value::as_str).unwrap_or_default().to_string()
}
fn inteiro(args: &[Value], i: usize) -> i32 {
    args.get(i).and_then(Value::as_i64).unwrap_or_default() as i32
}
fn real(args: &[Value], i: usize) -> f32 {
    args.get(i).and_then(Value::as_f64).unwrap_or_default() as f32
}
fn logico(args: &[Value], i: usize) -> bool {
    args.get(i).and_then(Value::as_bool).unwrap_or_default()
}

/// As acoes da tela, com os mesmos nomes dos callbacks do app em Slint.
fn executar(app: &AppHandle, m: &mut Motor, nome: &str, a: &[Value]) -> Result<(), String> {
    let s = &mut m.state;
    match nome {
        "navigate" => s.navigate(inteiro(a, 0)),
        "search" => s.search(&texto(a, 0)),
        "play-track" | "open" => s.play_track(&texto(a, 0)),
        // Pausar grava o ponto: parado, o giro nao grava mais.
        "toggle-play" => {
            s.toggle_play();
            s.gravar_retomada(true);
        }
        "stop-playback" => s.stop(),
        "next-track" => s.next_track(),
        "previous-track" => s.previous_track(),
        "seek" => s.seek(real(a, 0)),
        "seek-by" => s.seek_by(real(a, 0)),
        "set-volume" => s.set_volume(real(a, 0)),
        "toggle-mute" => s.toggle_mute(),
        "nudge-volume" => s.nudge_volume(real(a, 0)),
        "toggle-shuffle" => s.toggle_shuffle(),
        "cycle-repeat" => s.cycle_repeat(),
        "toggle-favorite" => s.toggle_favorite(&texto(a, 0)),
        "queue-play-next" => s.queue_play_next(&texto(a, 0)),
        "queue-enqueue" => s.queue_enqueue(&texto(a, 0)),
        "queue-remove" => s.queue_remove(inteiro(a, 0)),
        "queue-play-manual" => s.queue_play_manual(inteiro(a, 0)),
        "queue-move" => s.queue_move(inteiro(a, 0), inteiro(a, 1)),
        "liked-move" => s.mover_curtida(&texto(a, 0), inteiro(a, 1)),
        "queue-clear" => s.queue_clear(),
        "close-queue" => s.close_queue(),
        "select-theme" => s.select_theme(&texto(a, 0)),
        "import-theme" => s.import_theme_via_dialog(),
        "export-theme" => s.export_theme_via_dialog(&texto(a, 0)),
        "duplicate-theme" => s.duplicate_theme(&texto(a, 0)),
        "reset-theme" => s.reset_theme(),
        "open-theme-folder" => s.open_theme_folder(),
        "reload-theme" => s.reload_theme(),
        "toggle-sidebar" => s.toggle_sidebar(),
        "filter-playlists" => s.set_playlist_filter(&texto(a, 0)),
        "filter-library" => s.set_library_filter(&texto(a, 0)),
        "set-library-kind" => s.set_library_kind(inteiro(a, 0)),
        "set-library-sort" => s.set_library_sort(inteiro(a, 0)),
        "set-search-kind" => s.set_search_kind(inteiro(a, 0)),
        "toggle-pin-playlist" => s.toggle_pin_playlist(&texto(a, 0)),
        "detail-back" => s.close_detail(),
        "detail-play" => s.play_detail(),
        "detail-filter" => s.set_detail_filter(&texto(a, 0)),
        "detail-sort-by" => s.set_detail_sort(inteiro(a, 0)),
        "detail-activate" => s.activate_detail(&texto(a, 0)),
        "detail-load-more" => s.load_more_detail(),
        "toggle-detail-save" => s.toggle_detail_saved(),
        "set-sleep" => s.set_sleep_timer(inteiro(a, 0)),
        "clear-recent-searches" => s.clear_recent_searches(),
        "toggle-hidden" => s.toggle_hidden(&texto(a, 0)),
        "copy-link" => s.copy_link(&texto(a, 0)),
        "start-radio" => s.start_radio(&texto(a, 0)),
        "remove-from-playlist" => s.remove_track_from_open_playlist(&texto(a, 0)),
        "add-to-playlist" => s.add_track_to_playlist(&texto(a, 0), &texto(a, 1)),
        "create-playlist" => s.create_playlist(&texto(a, 0)),
        "rename-playlist" => s.rename_playlist(&texto(a, 0), &texto(a, 1)),
        "delete-playlist" => s.delete_playlist(&texto(a, 0)),
        "set-close-to-tray" => s.set_close_to_tray(logico(a, 0)),
        "set-track-notifications" => s.set_track_notifications(logico(a, 0)),
        "set-start-with-windows" => s.set_start_with_windows(logico(a, 0)),
        "set-autoplay" => s.set_autoplay(logico(a, 0)),
        "set-bitrate" => s.set_bitrate(inteiro(a, 0)),
        "set-normalize" => s.set_normalize(logico(a, 0)),
        "set-mono" => s.set_mono(logico(a, 0)),
        "clear-cache" => s.clear_cache(),
        "set-output-device" => s.set_output_device(&texto(a, 0)),
        "check-update" => s.check_for_update(),
        "download-update" => s.download_update(),
        "open-releases-page" => s.open_releases_page(),
        "install-update" => {
            if s.install_update() {
                app.exit(0);
            }
        }
        "toggle-mini-player" => {
            alternar_mini(app, m);
            return Ok(());
        }
        "toggle-mini-pin" => {
            let fixo = !m.janela.estado.borrow().mini_fixo;
            if let Some(w) = app.get_webview_window("main") {
                depois(&w, move |w2| {
                    let _ = w2.set_always_on_top(fixo);
                });
            }
            m.janela.set_mini_fixo(fixo);
            return Ok(());
        }
        // Buscar a partir do mini: a lista de resultados nao cabe nele, entao
        // a janela volta ao tamanho normal ja na Busca.
        "mini-search" => {
            if m.mini_antes.is_some() {
                alternar_mini(app, m);
            }
            m.state.navigate(1);
            return Ok(());
        }
        "fechar-bandeja" => fechar_menu_bandeja(app),
        // O mini player nao tem barra de titulo, entao nem o X da janela.
        "fechar-janela" => {
            if let Some(w) = app.get_webview_window("main") {
                depois(&w, |w| {
                    let _ = w.close();
                });
            }
        }
        // Criar janela dentro de um comando sincrono trava no Windows (aviso
        // do Tauri em `WebviewWindowBuilder::build`): a tela nunca aparecia e
        // a linha principal parava. Vai para a fila, como o clique no icone.
        "abrir-janela" => {
            fechar_menu_bandeja(app);
            let app = app.clone();
            std::thread::spawn(move || {
                let a = app.clone();
                let _ = app.run_on_main_thread(move || abrir_janela(&a));
            });
        }
        "sair" => {
            s.gravar_retomada(true);
            s.save_config();
            app.exit(0);
        }
        "login" => s.login(),
        "cancel-login" => s.cancel_login(),
        "copy-auth-url" => s.copy_auth_url(),
        "logout" => s.logout(),
        "dismiss-status" => s.clear_status(),
        "undo-last" => s.undo_last(),
        "retry-last" => s.retry_last(),
        "dismiss-undo" => {
            s.dismiss_recovery();
            s.clear_status();
        }
        "choose-background" => s.choose_background_via_dialog(),
        "clear-background" => s.clear_background(),
        "set-background-fit" => s.set_background_fit(inteiro(a, 0)),
        "set-background-opacity" => s.set_background_opacity(real(a, 0)),
        "set-background-tint" => s.set_background_tint(real(a, 0)),
        "set-background-blur" => s.set_background_blur(real(a, 0)),
        "set-glass-blur" => s.set_glass_blur(real(a, 0)),
        "set-font-scale" => s.set_font_scale(real(a, 0)),
        "zoom" => s.zoom(inteiro(a, 0)),
        "set-window-opacity" => s.set_window_opacity(real(a, 0)),
        "set-reduce-motion" => s.set_reduce_motion(logico(a, 0)),
        "set-hot-reload" => s.set_hot_reload(logico(a, 0)),
        outro => return Err(format!("acao desconhecida: {outro}")),
    }
    Ok(())
}

// ---- protocolo das capas ----

fn servir_capa(req: tauri::http::Request<Vec<u8>>) -> tauri::http::Response<Vec<u8>> {
    use tauri::http::Response;
    let consulta = req.uri().query().unwrap_or_default();
    match capas::resolver(consulta).and_then(|p| std::fs::read(&p).ok().map(|b| (p, b))) {
        Some((p, bytes)) => Response::builder()
            .header("Content-Type", capas::tipo(&p))
            .header("Cache-Control", "max-age=31536000, immutable")
            .body(bytes)
            .unwrap_or_default(),
        None => Response::builder().status(404).body(Vec::new()).unwrap_or_default(),
    }
}
