//! Estado da aplicacao e as acoes que a interface dispara.
//!
//! Concentrar tudo aqui deixa `main.rs` sendo so fiacao, e mantem a interface
//! sem nenhuma decisao de produto: cada callback do Slint chama um metodo deste
//! tipo e depois pede um `push_to_ui`.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use morune_core::playback::{NullEngine, PlaybackEngine, PlayerCommand, PlayerEvent};
use tokio::sync::broadcast;
use morune_core::queue::{Queue, RepeatMode};
use morune_core::Track;
use morune_storage::{AppPaths, Config};
use morune_theme::{loader, ThemeSpec};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::session::Session;
use crate::theme_bridge::{self, UserOverrides};
use crate::ui;

/// Paginas da interface. Os numeros sao o contrato com o Slint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home = 0,
    Search = 1,
    Library = 2,
    Settings = 3,
    Queue = 4,
}

impl Page {
    fn from_i32(v: i32) -> Self {
        match v {
            1 => Page::Search,
            2 => Page::Library,
            3 => Page::Settings,
            4 => Page::Queue,
            _ => Page::Home,
        }
    }
}

pub struct AppState {
    paths: AppPaths,
    config: Config,
    theme: loader::LoadedTheme,
    overrides: UserOverrides,
    page: Page,
    status: String,
    queue: Queue,
    engine: Arc<dyn PlaybackEngine>,
    session: Session,
    /// Eventos do motor ativo. Trocado junto com o motor.
    player_events: Option<broadcast::Receiver<PlayerEvent>>,
    volume: f32,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("theme", &self.theme.spec.manifest.id)
            .field("page", &self.page)
            .field("engine", &self.engine.name())
            .finish_non_exhaustive()
    }
}

impl AppState {
    pub fn load() -> Self {
        let paths = AppPaths::discover();
        if let Err(e) = paths.ensure() {
            tracing::warn!(error = %e, "nao foi possivel criar as pastas do aplicativo");
        }

        // Instalado antes de carregar o tema para que a primeira execucao ja
        // encontre os temas de exemplo, inclusive se o usuario tiver escolhido
        // um deles numa instalacao anterior.
        let installed = crate::bundled::install_missing(&paths.themes_dir());
        if !installed.is_empty() {
            tracing::info!(?installed, "temas de exemplo instalados");
        }

        let config = Config::load(&paths.config_file());
        let theme = loader::load(&paths.themes_dir(), &config.appearance.theme);
        if theme.fell_back {
            tracing::warn!(
                pedido = config.appearance.theme,
                "tema indisponivel; usando o embutido"
            );
        }

        let overrides = UserOverrides {
            font_scale: config.appearance.font_scale_override,
            reduce_motion: config.appearance.reduce_motion,
            sidebar_collapsed: false,
        };

        let mut queue = Queue::new();
        queue.set_shuffle(config.playback.shuffle);
        queue.set_repeat(config.playback.repeat);

        Self {
            volume: config.playback.volume,
            paths,
            config,
            theme,
            overrides,
            page: Page::Home,
            status: String::new(),
            queue,
            // Sem backend real ate haver login: o motor nulo aceita
            // preferencias e recusa reproducao, sem que a interface precise
            // tratar "sem motor" em lugar nenhum.
            engine: Arc::new(NullEngine::new("entre na sua conta para tocar musica")),
            session: Session::new(Arc::from(morune_storage::platform_store())),
            player_events: None,
        }
    }

    /// Tenta reabrir a ultima sessao do Spotify.
    ///
    /// Chamado depois de a janela aparecer, nunca antes: o orcamento de startup
    /// nao tem espaco para esperar rede, e uma janela que demora a abrir por
    /// causa de login e exatamente o defeito que este aplicativo evita.
    pub fn restore_session(&mut self) {
        self.session.restore();
    }

    /// Recolhe o que o backend produziu desde a ultima leitura.
    ///
    /// Devolve `true` quando algo mudou e a interface precisa ser atualizada.
    /// Roda no mesmo temporizador da bandeja, entao precisa ser barato quando
    /// nao ha nada -- e e: dois `try_recv` que falham.
    pub fn poll_backend(&mut self) -> bool {
        let mut changed = false;

        // Sem tentativa em andamento nao ha canal para ler: o caso comum sai
        // daqui sem tocar em nada.
        let pending = self.session.is_busy().then(|| self.session.poll()).flatten();
        if let Some(change) = pending {
            self.status = change.message;
            if let Some(engine) = change.engine {
                self.player_events = Some(engine.subscribe());
                // O volume escolhido antes do login vale para a sessao nova:
                // o usuario nao deveria ter que ajustar de novo.
                let _ = engine.send(PlayerCommand::SetVolume(self.volume));
                self.engine = engine;
            }
            changed = true;
        }

        while let Some(event) = self.next_player_event() {
            changed |= self.apply_player_event(event);
        }

        changed
    }

    fn next_player_event(&mut self) -> Option<PlayerEvent> {
        let events = self.player_events.as_mut()?;
        match events.try_recv() {
            Ok(event) => Some(event),
            Err(broadcast::error::TryRecvError::Lagged(perdidos)) => {
                // Perder evento e aceitavel: o proximo retrato corrige o que a
                // tela mostra. Registrar serve para notar se vira habito.
                tracing::debug!(perdidos, "eventos do player descartados");
                None
            }
            Err(broadcast::error::TryRecvError::Empty) => None,
            Err(broadcast::error::TryRecvError::Closed) => {
                self.player_events = None;
                None
            }
        }
    }

    /// Aplica um evento do motor. Devolve `true` quando a tela muda.
    fn apply_player_event(&mut self, event: PlayerEvent) -> bool {
        match event {
            // O fim natural da faixa avanca a fila com `user_advance = false`,
            // que e o que faz "repetir uma" repetir em vez de pular.
            PlayerEvent::EndOfTrack(_) => {
                if let Some(next) = self.queue.next(false).cloned() {
                    self.send(PlayerCommand::Load { track: next, start_paused: false });
                } else {
                    self.status = "Fim da fila.".into();
                }
                true
            }
            PlayerEvent::Error(message) => {
                self.status = message;
                true
            }
            PlayerEvent::StateChanged(_) | PlayerEvent::TrackChanged(_) => true,
            // Posicao e volume ja aparecem pelo retrato lido a cada quadro;
            // repintar por causa deles seria trabalho sem efeito visivel.
            _ => false,
        }
    }

    pub fn theme_id(&self) -> &str {
        &self.theme.spec.manifest.id
    }

    fn spec(&self) -> &ThemeSpec {
        &self.theme.spec
    }

    // ---- tema ----

    pub fn apply_theme_to(&self, window: &ui::AppWindow) {
        theme_bridge::apply(
            &window.global::<ui::Theme>(),
            &window.global::<ui::Layout>(),
            self.spec(),
            self.overrides,
        );
    }

    /// Define o tamanho inicial da janela.
    ///
    /// Aplicado so na abertura: o tema propoe um tamanho, mas o ultimo tamanho
    /// escolhido pelo usuario tem prioridade, e trocar de tema depois nunca
    /// redimensiona a janela.
    pub fn apply_initial_window_size(&self, window: &ui::AppWindow) {
        let (width, height) = if self.config.window.width > 0.0 && self.config.window.height > 0.0 {
            (self.config.window.width, self.config.window.height)
        } else {
            theme_bridge::initial_window_size(self.spec())
        };
        window.window().set_size(slint::LogicalSize::new(width, height));
    }

    pub fn select_theme(&mut self, id: &str) {
        let loaded = loader::load(&self.paths.themes_dir(), id);
        if loaded.fell_back {
            self.status = format!("Nao foi possivel aplicar o tema {id}; nada mudou.");
            return;
        }
        self.status = format!("Tema aplicado: {}", loaded.spec.manifest.name);
        self.theme = loaded;
        self.config.appearance.theme = id.to_string();
        self.save_config();
    }

    pub fn reload_theme(&mut self) {
        let id = self.config.appearance.theme.clone();
        self.theme = loader::load(&self.paths.themes_dir(), &id);
        self.status = if self.theme.is_healthy() {
            "Tema recarregado.".into()
        } else {
            "Tema recarregado com avisos; veja os diagnosticos.".into()
        };
    }

    pub fn reset_theme(&mut self) {
        self.theme = loader::LoadedTheme::builtin();
        self.config.appearance = Default::default();
        self.overrides = UserOverrides::default();
        self.status = "Tema restaurado para o padrao.".into();
        self.save_config();
    }

    pub fn duplicate_theme(&mut self, id: &str) {
        let source = loader::load(&self.paths.themes_dir(), id);
        let mut spec = source.spec;

        let new_id = self.unique_theme_id(&format!("{}-copia", spec.manifest.id));
        spec.manifest.name = format!("{} (copia)", spec.manifest.name);
        spec.manifest.id = new_id.clone();
        // A copia nao herda do original: e uma copia completa, entao apagar o
        // original nao pode quebra-la.
        spec.manifest.based_on = None;

        match loader::write_theme(&self.paths.theme_dir(&new_id), &spec) {
            Ok(()) => {
                self.status = format!("Tema duplicado como {new_id}.");
                self.select_theme(&new_id);
            }
            Err(e) => self.status = format!("Nao foi possivel duplicar o tema: {e}"),
        }
    }

    fn unique_theme_id(&self, base: &str) -> String {
        let mut candidate = base.to_string();
        let mut n = 2;
        while self.paths.theme_dir(&candidate).exists() {
            candidate = format!("{base}-{n}");
            n += 1;
        }
        candidate
    }

    pub fn import_theme_via_dialog(&mut self) {
        let Some(file) = rfd::FileDialog::new()
            .add_filter("Pacote de tema Morune", &[morune_theme::PACK_EXTENSION])
            .set_title("Importar tema")
            .pick_file()
        else {
            return;
        };

        match morune_theme::import_pack(&file, &self.paths.themes_dir(), true) {
            Ok(imported) => {
                self.status = format!(
                    "Tema {} importado ({} arquivos).",
                    imported.manifest.name, imported.files_written
                );
                let id = imported.manifest.id.clone();
                self.select_theme(&id);
            }
            Err(e) => {
                tracing::warn!(error = %e, "importacao de tema recusada");
                self.status = format!("Pacote recusado: {e}");
            }
        }
    }

    pub fn export_theme_via_dialog(&mut self, id: &str) {
        let dir = self.paths.theme_dir(id);
        if !dir.is_dir() {
            self.status = "O tema embutido nao pode ser exportado; duplique-o antes.".into();
            return;
        }

        let Some(target) = rfd::FileDialog::new()
            .add_filter("Pacote de tema Morune", &[morune_theme::PACK_EXTENSION])
            .set_file_name(format!("{id}.{}", morune_theme::PACK_EXTENSION))
            .set_title("Exportar tema")
            .save_file()
        else {
            return;
        };

        match morune_theme::export_pack(&dir, &target) {
            Ok(count) => self.status = format!("Tema exportado ({count} arquivos)."),
            Err(e) => self.status = format!("Falha ao exportar: {e}"),
        }
    }

    pub fn open_theme_folder(&mut self) {
        let dir = if self.paths.theme_dir(self.theme_id()).is_dir() {
            self.paths.theme_dir(self.theme_id())
        } else {
            self.paths.themes_dir()
        };
        self.open_in_explorer(&dir);
    }

    fn open_in_explorer(&mut self, dir: &PathBuf) {
        #[cfg(windows)]
        let result = std::process::Command::new("explorer.exe").arg(dir).spawn();
        #[cfg(not(windows))]
        let result = std::process::Command::new("xdg-open").arg(dir).spawn();

        match result {
            // `explorer.exe` retorna codigo diferente de zero mesmo quando abre
            // a janela; so a falha em iniciar o processo e um erro de verdade.
            Ok(_) => self.status = format!("Pasta aberta: {}", dir.display()),
            Err(e) => self.status = format!("Nao foi possivel abrir a pasta: {e}"),
        }
    }

    // ---- comportamento da janela ----

    pub fn close_to_tray(&self) -> bool {
        self.config.window.close_to_tray
    }

    pub fn set_close_to_tray(&mut self, on: bool) {
        self.config.window.close_to_tray = on;
        self.status = if on {
            "Fechar a janela vai manter o Morune tocando na bandeja.".into()
        } else {
            "Fechar a janela vai encerrar o Morune.".into()
        };
        self.save_config();
    }

    /// Texto e estado da faixa atual, para o menu da bandeja.
    pub fn tray_status(&self) -> (Option<String>, bool) {
        let label = self.queue.current().map(|t| {
            let artists = t.artists_line();
            if artists.is_empty() {
                t.name.to_string()
            } else {
                format!("{} — {artists}", t.name)
            }
        });
        (label, self.engine.snapshot().state == morune_core::PlaybackState::Playing)
    }

    pub fn toggle_sidebar(&mut self) {
        self.overrides.sidebar_collapsed = !self.overrides.sidebar_collapsed;
    }

    // ---- navegacao ----

    pub fn navigate(&mut self, page: i32) {
        self.page = Page::from_i32(page);
    }

    /// Abre uma pagina especifica na inicializacao.
    ///
    /// Usada pela captura de tela, que precisa fotografar telas que nao sao a
    /// inicial sem depender de automatizar cliques.
    pub fn open_page_from_env(&mut self) {
        if let Ok(value) = std::env::var("MORUNE_START_PAGE") {
            if let Ok(page) = value.parse::<i32>() {
                self.page = Page::from_i32(page);
            }
        }
    }

    pub fn search(&mut self, query: &str) {
        if query.trim().is_empty() {
            return;
        }
        // A busca real depende do catalogo do provedor, que so existe depois do
        // login. Sem sessao, dizer isso e melhor que uma lista vazia sem
        // explicacao.
        self.status = format!("Busca por \"{query}\" precisa de uma sessao ativa.");
    }

    // ---- reproducao ----

    pub fn play_track(&mut self, id: &str) {
        let Some(index) = self.queue.tracks().iter().position(|t| t.id.canonical() == id) else {
            self.status = "Faixa nao esta na fila atual.".into();
            return;
        };
        let track = self.queue.jump_to(index).cloned();
        if let Some(track) = track {
            self.send(PlayerCommand::Load { track, start_paused: false });
        }
    }

    pub fn toggle_play(&mut self) {
        self.send(PlayerCommand::TogglePlay);
    }

    pub fn next_track(&mut self) {
        if let Some(track) = self.queue.next(true).cloned() {
            self.send(PlayerCommand::Load { track, start_paused: false });
        } else {
            self.send(PlayerCommand::Stop);
        }
    }

    pub fn previous_track(&mut self) {
        if let Some(track) = self.queue.previous().cloned() {
            self.send(PlayerCommand::Load { track, start_paused: false });
        }
    }

    pub fn seek(&mut self, progress: f32) {
        let duration = self.queue.current().map(|t| t.duration).unwrap_or_default();
        let target = duration.mul_f32(progress.clamp(0.0, 1.0));
        self.send(PlayerCommand::Seek(target));
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.config.playback.volume = self.volume;
        let _ = self.engine.send(PlayerCommand::SetVolume(self.volume));
    }

    pub fn toggle_shuffle(&mut self) {
        let on = !self.queue.shuffle();
        self.queue.set_shuffle(on);
        self.config.playback.shuffle = on;
        let _ = self.engine.send(PlayerCommand::SetShuffle(on));
    }

    pub fn cycle_repeat(&mut self) {
        let mode = self.queue.repeat().cycle();
        self.queue.set_repeat(mode);
        self.config.playback.repeat = mode;
        let _ = self.engine.send(PlayerCommand::SetRepeat(mode));
    }

    fn send(&mut self, command: PlayerCommand) {
        if let Err(e) = self.engine.send(command) {
            self.status = e.to_string();
        }
    }

    // ---- sessao ----

    pub fn login(&mut self) {
        self.status = self.session.login();
    }

    pub fn logout(&mut self) {
        // Desligar o motor antes de apagar a credencial: ele segura a conexao
        // com o Spotify, e deixar a conexao viva depois do logout manteria o
        // dispositivo ocupado na conta.
        let _ = self.engine.send(PlayerCommand::Shutdown);
        self.engine = Arc::new(NullEngine::new("sessao encerrada"));
        self.player_events = None;
        self.session.logout();
        self.queue.clear();
        self.status = "Sessao encerrada.".into();
    }

    // ---- persistencia ----

    pub fn save_config(&self) {
        if let Err(e) = self.config.save(&self.paths.config_file()) {
            tracing::error!(error = %e, "falha ao salvar configuracao");
        }
    }

    // ---- espelhamento para a interface ----

    pub fn push_to_ui(&self, window: &ui::AppWindow) {
        window.set_page(self.page as i32);
        window.set_status_message(SharedString::from(self.status.as_str()));
        window.set_logged_in(self.session.state().is_logged_in());
        window.set_account_name(SharedString::from(self.session.state().account_name()));
        window.set_dev_mode(self.config.developer.enabled);
        window.set_close_to_tray(self.config.window.close_to_tray);

        let snapshot = self.engine.snapshot();
        let current = self.queue.current();
        window.set_has_track(current.is_some());
        window.set_playing(snapshot.state == morune_core::PlaybackState::Playing);
        window.set_progress(snapshot.progress());
        window.set_elapsed(format_time(snapshot.position).into());
        window.set_total(
            format_time(current.map(|t| t.duration).unwrap_or(snapshot.duration)).into(),
        );
        window.set_volume(self.volume);
        window.set_shuffle(self.queue.shuffle());
        window.set_repeat(match self.queue.repeat() {
            RepeatMode::Off => 0,
            RepeatMode::All => 1,
            RepeatMode::One => 2,
        });
        window.set_now_title(current.map(|t| t.name.as_ref()).unwrap_or("").into());
        window.set_now_artist(current.map(|t| t.artists_line()).unwrap_or_default().into());

        window.set_queue_tracks(track_rows(self.queue.upcoming(200), current));
        window.set_themes(self.theme_items());
        window.set_diagnostics(self.diagnostics());
        window.set_home_items(ModelRc::new(VecModel::<ui::CardItem>::default()));
        window.set_library_items(ModelRc::new(VecModel::<ui::CardItem>::default()));
        window.set_search_tracks(ModelRc::new(VecModel::<ui::TrackRow>::default()));
    }

    fn theme_items(&self) -> ModelRc<ui::ThemeItem> {
        let active = self.theme_id();
        let items: Vec<ui::ThemeItem> = loader::discover(&self.paths.themes_dir())
            .into_iter()
            .map(|entry| ui::ThemeItem {
                id: entry.manifest.id.as_str().into(),
                name: entry.manifest.name.as_str().into(),
                author: entry.manifest.author.as_str().into(),
                builtin: entry.builtin,
                active: entry.manifest.id == active,
            })
            .collect();
        ModelRc::new(VecModel::from(items))
    }

    fn diagnostics(&self) -> ModelRc<ui::Diagnostic> {
        let mut items: Vec<ui::Diagnostic> = self
            .theme
            .errors
            .iter()
            .map(|e| ui::Diagnostic {
                level: "error".into(),
                field: "tema".into(),
                message: e.as_str().into(),
            })
            .collect();
        items.extend(self.theme.warnings.iter().map(|w| ui::Diagnostic {
            level: "warning".into(),
            field: w.field.as_str().into(),
            message: w.message.as_str().into(),
        }));
        ModelRc::new(VecModel::from(items))
    }
}

fn track_rows(tracks: Vec<&Track>, current: Option<&Track>) -> ModelRc<ui::TrackRow> {
    let rows: Vec<ui::TrackRow> = tracks
        .into_iter()
        .map(|t| ui::TrackRow {
            id: t.id.canonical().into(),
            title: t.name.as_ref().into(),
            artist: t.artists_line().into(),
            album: t.album.as_ref().map(|a| a.name.as_ref()).unwrap_or("").into(),
            duration: format_time(t.duration).into(),
            playable: t.playable,
            playing: current.is_some_and(|c| c.id == t.id),
        })
        .collect();
    ModelRc::new(VecModel::from(rows))
}

/// Formata uma duracao como `m:ss`, ou `h:mm:ss` quando passa de uma hora.
fn format_time(d: Duration) -> String {
    let total = d.as_secs();
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_formatting_matches_what_a_player_shows() {
        assert_eq!(format_time(Duration::ZERO), "0:00");
        assert_eq!(format_time(Duration::from_secs(9)), "0:09");
        assert_eq!(format_time(Duration::from_secs(75)), "1:15");
        assert_eq!(format_time(Duration::from_secs(599)), "9:59");
        assert_eq!(format_time(Duration::from_secs(3600)), "1:00:00");
        assert_eq!(format_time(Duration::from_secs(3661)), "1:01:01");
    }

    #[test]
    fn page_numbers_are_the_contract_with_the_ui() {
        assert_eq!(Page::Home as i32, 0);
        assert_eq!(Page::Queue as i32, 4);
        assert_eq!(Page::from_i32(3), Page::Settings);
        // Valor desconhecido nunca deixa a interface sem pagina.
        assert_eq!(Page::from_i32(99), Page::Home);
        assert_eq!(Page::from_i32(-1), Page::Home);
    }
}
