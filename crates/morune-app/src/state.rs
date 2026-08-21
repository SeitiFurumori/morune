//! Estado da aplicacao e as acoes que a interface dispara.
//!
//! Concentrar tudo aqui deixa `main.rs` sendo so fiacao, e mantem a interface
//! sem nenhuma decisao de produto: cada callback do Slint chama um metodo deste
//! tipo e depois pede um `push_to_ui`.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use morune_core::playback::{NullEngine, PlaybackEngine, PlayerCommand, PlayerEvent};
use morune_core::queue::{Queue, QueueOrigin, RepeatMode};
use morune_core::{Track, TrackId};
use morune_storage::{AppPaths, Config};
use morune_theme::{loader, ThemeSpec};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use tokio::sync::broadcast;

use crate::browse::{AutoplayOutcome, Card, Home, LibraryOutcome, Outcome, Target};
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
    /// Uma lista aberta: playlist, album, artista ou as curtidas.
    Detail = 5,
}

impl Page {
    fn from_i32(v: i32) -> Self {
        match v {
            1 => Page::Search,
            2 => Page::Library,
            3 => Page::Settings,
            4 => Page::Queue,
            5 => Page::Detail,
            _ => Page::Home,
        }
    }
}

/// Por que a tela de detalhe esta ordenada.
///
/// [`SortBy::Original`] nao e "sem ordem": e a ordem em que a lista foi
/// montada -- a da playlist, a do album, a de quando a faixa foi curtida. E a
/// unica que carrega intencao, entao e o padrao e da para voltar a ela.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Original,
    Title,
    Artist,
    Album,
    Duration,
}

impl SortBy {
    fn from_i32(v: i32) -> Self {
        match v {
            1 => SortBy::Title,
            2 => SortBy::Artist,
            3 => SortBy::Album,
            4 => SortBy::Duration,
            _ => SortBy::Original,
        }
    }
}

/// Acao aguardando a lista inteira quando filtro ou ordenacao tornam o trecho
/// parcial ambiguo. Na ordem original a reproducao comeca imediatamente e as
/// paginas seguintes entram na fila em segundo plano.
#[derive(Debug, Clone)]
enum PendingDetailPlay {
    First,
    Track(TrackId),
}

/// Ultima acao destrutiva que ainda pode ser revertida pelo aviso na tela.
enum UndoAction {
    QueueClear(Vec<Track>),
    ThemeImport {
        previous_id: String,
        installed_id: String,
        backup: Option<PathBuf>,
    },
}

pub struct AppState {
    paths: AppPaths,
    config: Config,
    theme: loader::LoadedTheme,
    overrides: UserOverrides,
    page: Page,
    status: String,
    /// A mensagem que o relogio de expiracao esta contando, e desde quando.
    ///
    /// Comparar com `status` a cada leitura evita ter que lembrar de reiniciar
    /// o relogio nos mais de quarenta lugares que escrevem uma mensagem -- e um
    /// deles esquecido seria uma mensagem que nunca some, que e exatamente o
    /// defeito que isto conserta.
    status_seen: (String, Instant),
    undo: Option<UndoAction>,
    retry_available: bool,
    start_with_windows: bool,
    queue: Queue,
    engine: Arc<dyn PlaybackEngine>,
    session: Session,
    /// Eventos do motor ativo. Trocado junto com o motor.
    player_events: Option<broadcast::Receiver<PlayerEvent>>,
    volume: f32,
    /// Temas instalados, lidos do disco uma vez.
    ///
    /// Memoizado porque `push_to_ui` roda em todo clique, e `loader::discover`
    /// faz `read_dir` mais leitura e parse de um TOML por tema. Repetir isso na
    /// thread da interface a cada acao e o que deixava os controles lentos.
    /// Recarregado por `refresh_themes` quando o conjunto muda.
    themes: Vec<loader::ThemeEntry>,
    /// O que a barra mostra em play/pause.
    ///
    /// Otimista: o clique escreve aqui antes de o motor confirmar, para o icone
    /// trocar no ato. O `StateChanged` do motor continua sendo a verdade e
    /// corrige este campo quando chega.
    playing: bool,
    /// Posicao pedida por um seek que o motor ainda nao confirmou.
    ///
    /// O motor vive noutra thread: sem isto, o espelhamento feito logo depois
    /// do clique leria o retrato antigo e a barra pularia de volta para onde
    /// estava.
    seek_target: Option<Duration>,
    /// Listas de faixas visiveis na tela, guardadas inteiras: ativar uma faixa
    /// transforma a lista onde ela esta em contexto da fila, e nao so a faixa
    /// clicada. Sem isso, clicar numa faixa da busca tocaria uma faixa so e o
    /// botao de proxima nao teria para onde ir.
    search: TrackList,
    search_cards: Vec<Card>,
    search_query: String,
    searching: bool,
    liked: TrackList,
    home_made_for_you: Vec<Card>,
    home_stations: Vec<Card>,
    home_retrospectives: Vec<Card>,
    home_playlists: Vec<Card>,
    library: Vec<Card>,
    /// Estado completo das "Musicas curtidas" do Spotify.
    liked_ids: HashSet<TrackId>,
    /// Cliques aguardando confirmacao remota; impede alternancias duplicadas.
    liked_pending: HashSet<TrackId>,
    /// Texto do filtro da barra lateral.
    ///
    /// Filtrar acontece aqui e nao no backend: as 86 playlists da conta ja
    /// estao na memoria, e ir a rede a cada tecla seria gastar requisicao
    /// para responder o que ja se sabe.
    playlist_filter: String,
    /// Cartao fixo das curtidas, sempre no topo da barra lateral.
    ///
    /// Nao vem do `rootlist`: curtidas nao sao playlist para o Spotify. Mas
    /// sao a lista que mais se abre, entao ficam em primeiro lugar -- antes
    /// inclusive da ordem que o usuario arrumou.
    liked_card: Card,
    /// A lista aberta, quando ha uma.
    detail: Option<crate::browse::Detail>,
    detail_filter: String,
    detail_sort: SortBy,
    /// `true` inverte a ordenacao. Nao se aplica a [`SortBy::Original`]:
    /// inverter a ordem da playlist nao e ordenar, e embaralhar ao contrario.
    detail_desc: bool,
    /// Uma pagina seguinte esta em voo. Impede que roda do mouse e touchpad
    /// disparem a mesma requisicao varias vezes.
    detail_loading: bool,
    /// Continua pedindo paginas sem esperar nova rolagem. Ativado quando uma
    /// acao precisa conhecer a lista inteira (filtro, ordenacao ou fila).
    detail_complete_requested: bool,
    detail_pending_play: Option<PendingDetailPlay>,
    /// A colecao que esta chegando foi pedida so para encher a fila.
    ///
    /// A tela de detalhe carrega, mas o usuario continua onde estava: ele
    /// clicou numa musica, nao numa lista.
    detail_silent: bool,
    /// Faixa a tocar quando as curtidas terminarem de abrir.
    ///
    /// A prateleira do Inicio guarda so um punhado de faixas, e usa-la como
    /// contexto dava uma fila de cinco musicas. Clicar ali passa a abrir a
    /// colecao inteira por tras, que entra na fila em lotes.
    pending_liked_play: Option<TrackId>,
    /// De onde a tela de detalhe foi aberta, para o botao de voltar.
    detail_from: Page,
    /// Capa da faixa tocando: a URL pedida e o arquivo, quando ja chegou.
    ///
    /// Guardada separada dos cartoes porque a faixa tocando nao esta
    /// necessariamente em nenhuma tela aberta -- ela continua tocando com o
    /// usuario navegando por outra coisa.
    now_cover: (String, Option<std::path::PathBuf>),
    /// Capas pequenas das linhas, indexadas pela URL que o modelo da faixa traz.
    track_covers: HashMap<String, std::path::PathBuf>,
    /// Semente do pedido de autoplay em voo; impede uma resposta antiga de
    /// continuar uma fila que o usuario ja substituiu.
    autoplay_seed: Option<morune_core::TrackId>,
    /// `true` depois de a tela ter sido pedida ao backend, e nao depois de
    /// chegar: sem isso, ir e voltar numa tela lenta dispara uma requisicao por
    /// visita.
    home_requested: bool,
    library_requested: bool,
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

        let themes = loader::discover(&paths.themes_dir());

        let overrides = UserOverrides {
            font_scale: config.appearance.font_scale_override,
            reduce_motion: config.appearance.reduce_motion,
            sidebar_collapsed: false,
        };

        let mut queue = Queue::new();
        queue.set_shuffle(config.playback.shuffle);
        queue.set_repeat(config.playback.repeat);

        // A pasta do cache de capas e lida antes de `paths` ser movido para o
        // estado.
        let covers_dir = paths.artwork_cache_dir();
        let start_with_windows = crate::startup::is_enabled();

        let loaded = Self {
            volume: config.playback.volume,
            paths,
            config,
            theme,
            overrides,
            page: Page::Home,
            status: String::new(),
            status_seen: (String::new(), Instant::now()),
            undo: None,
            retry_available: false,
            start_with_windows,
            queue,
            // Sem backend real ate haver login: o motor nulo aceita
            // preferencias e recusa reproducao, sem que a interface precise
            // tratar "sem motor" em lugar nenhum.
            engine: Arc::new(NullEngine::new("entre na sua conta para tocar musica")),
            session: Session::new(Arc::from(morune_storage::platform_store()), covers_dir),
            player_events: None,
            themes,
            playing: false,
            seek_target: None,
            search: TrackList::default(),
            search_cards: Vec::new(),
            search_query: String::new(),
            searching: false,
            liked: TrackList::default(),
            home_made_for_you: Vec::new(),
            home_stations: Vec::new(),
            home_retrospectives: Vec::new(),
            home_playlists: Vec::new(),
            library: Vec::new(),
            liked_ids: HashSet::new(),
            liked_pending: HashSet::new(),
            playlist_filter: String::new(),
            detail: None,
            detail_filter: String::new(),
            detail_sort: SortBy::Original,
            detail_desc: false,
            detail_loading: false,
            detail_complete_requested: false,
            detail_pending_play: None,
            detail_silent: false,
            pending_liked_play: None,
            detail_from: Page::Home,
            liked_card: Card {
                tag: crate::browse::Target::Liked.tag(),
                title: crate::browse::LIKED_TITLE.into(),
                subtitle: String::new(),
                cover: String::new(),
                cover_path: None,
            },
            now_cover: (String::new(), None),
            track_covers: HashMap::new(),
            autoplay_seed: None,
            home_requested: false,
            library_requested: false,
        };

        #[cfg(feature = "snapshot")]
        let mut loaded = loaded;
        #[cfg(feature = "snapshot")]
        loaded.install_snapshot_detail_demo();

        loaded
    }

    /// Conteudo determinista para inspecionar a lista longa sem uma conta nem
    /// rede. Compilado somente pela ferramenta de snapshot, nunca no produto.
    #[cfg(feature = "snapshot")]
    fn install_snapshot_detail_demo(&mut self) {
        if std::env::var_os("MORUNE_SNAPSHOT_DETAIL_DEMO").is_none() {
            return;
        }

        let tracks = (1..=100)
            .map(|number| Track {
                id: TrackId::spotify(format!("snapshot{number}")),
                name: format!("Faixa {number:03}").into(),
                artists: vec![morune_core::model::ArtistRef {
                    id: morune_core::model::ArtistId::spotify("snapshotartist"),
                    name: "Artista de exemplo".into(),
                }],
                album: Some(morune_core::model::AlbumRef {
                    id: morune_core::model::AlbumId::spotify("snapshotalbum"),
                    name: "Album de exemplo".into(),
                    images: Default::default(),
                }),
                duration: Duration::from_secs(180 + u64::from(number % 60)),
                track_number: Some(number),
                disc_number: Some(1),
                explicit: false,
                playable: true,
            })
            .collect();

        self.detail = Some(crate::browse::Detail {
            origin: QueueOrigin::Custom(crate::browse::LIKED_TITLE.into()),
            title: crate::browse::LIKED_TITLE.into(),
            subtitle: "719 faixas".into(),
            kind: "Colecao".into(),
            cover: String::new(),
            cover_path: None,
            tracks,
            cards: Vec::new(),
            total_tracks: Some(719),
            source: Some(Target::Liked),
            has_more: true,
        });
        self.page = Page::Detail;
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
        let pending = self
            .session
            .is_busy()
            .then(|| self.session.poll())
            .flatten();
        if let Some(change) = pending {
            self.status = change.message;
            if let Some(engine) = change.engine {
                self.player_events = Some(engine.subscribe());
                // O volume escolhido antes do login vale para a sessao nova:
                // o usuario nao deveria ter que ajustar de novo.
                let _ = engine.send(PlayerCommand::SetVolume(self.volume));
                self.engine = engine;
            }
            if self.session.state().is_logged_in() {
                // A tela aberta na hora do login precisa se preencher sozinha:
                // o usuario acabou de entrar e nao vai clicar em "Inicio" de
                // novo so para ver o que ja deveria estar la.
                self.home_requested = false;
                self.library_requested = false;
                self.request_page_data();
            }
            changed = true;
        }

        if let Some(outcome) = self.session.browse_mut().and_then(|b| b.poll()) {
            self.apply_browse(outcome);
            changed = true;
        }

        if let Some(outcome) = self.session.browse_mut().and_then(|b| b.poll_autoplay()) {
            self.apply_autoplay(outcome);
            changed = true;
        }

        while let Some(outcome) = self.session.browse_mut().and_then(|b| b.poll_library()) {
            self.apply_library(outcome);
            changed = true;
        }

        while let Some(event) = self.next_player_event() {
            changed |= self.apply_player_event(event);
        }

        // Antes de recolher qualquer coisa: uma sessao caida faz todo o resto
        // falhar, e a volta e silenciosa.
        if self.session.reconnect_if_lost() {
            self.status = "Reconectando ao Spotify...".into();
            changed = true;
        }

        changed |= self.poll_covers();

        // Depois dos eventos do player: a troca de faixa acabou de ser
        // aplicada, entao a capa pedida aqui ja e a da faixa certa.
        self.resolve_now_cover();

        // Por ultimo: tudo acima pode ter escrito uma mensagem nova, e o
        // relogio dela comeca agora, nao no ciclo que vem.
        changed |= self.expire_status();

        changed
    }

    /// Quanto tempo uma mensagem fica na tela antes de sumir sozinha.
    ///
    /// Da para ler uma frase sem pressa e some antes de virar parte do cenario.
    /// "Conectado como fulano" ficava ate alguem clicar no X.
    const STATUS_TIMEOUT: Duration = Duration::from_secs(6);

    /// Apaga a mensagem quando ela ja cumpriu o tempo dela.
    ///
    /// Mensagem com "Desfazer" ou "Tentar novamente" **nao** expira: ela nao e
    /// so aviso, e o unico lugar de onde essa acao pode ser feita, e some-la
    /// tiraria do usuario uma escolha que ele ainda nao fez.
    fn expire_status(&mut self) -> bool {
        if self.status.is_empty() {
            // Esquecer a mensagem antiga aqui e o que faz a *mesma* mensagem,
            // se voltar, aparecer com o relogio zerado em vez de sumir na hora.
            self.status_seen.0.clear();
            return false;
        }

        if self.status != self.status_seen.0 {
            self.status_seen = (self.status.clone(), Instant::now());
            return false;
        }

        let has_action = self.undo_available() || self.retry_available;
        if !status_expired(
            self.status_seen.1.elapsed(),
            has_action,
            Self::STATUS_TIMEOUT,
        ) {
            return false;
        }

        self.status.clear();
        self.status_seen.0.clear();
        true
    }

    /// Aplica o que a busca ou a biblioteca trouxeram.
    fn apply_browse(&mut self, outcome: Outcome) {
        self.retry_available = matches!(&outcome, Outcome::Failed(_));
        match outcome {
            Outcome::Search {
                query,
                tracks,
                cards,
            } => {
                self.searching = false;
                let total = tracks.len() + cards.len();
                self.status = if total == 0 {
                    format!("Nada encontrado para \"{query}\".")
                } else {
                    format!("{total} resultados para \"{query}\".")
                };
                self.search_query = query.clone();
                self.search = TrackList {
                    origin: QueueOrigin::Search(query),
                    tracks,
                };
                self.search_cards = cards;
                self.resolve_covers();
            }
            // Sem mensagem no sucesso: a tela vazia ja se explica sozinha, e
            // uma linha de status aqui apagaria o "Conectado como ..." que o
            // usuario acabou de receber.
            Outcome::Home(home) => {
                let Home {
                    made_for_you,
                    stations,
                    retrospectives,
                    liked,
                    liked_ids,
                    playlists,
                } = *home;
                self.liked_ids = liked_ids.into_iter().collect();
                self.home_made_for_you = made_for_you;

                self.liked = TrackList {
                    origin: QueueOrigin::Custom("Musicas curtidas".into()),
                    tracks: liked,
                };
                #[cfg(feature = "snapshot")]
                if std::env::var_os("MORUNE_SNAPSHOT_QUEUE_DEMO").is_some()
                    && self.liked.tracks.len() >= 2
                {
                    self.queue.play_next(self.liked.tracks[0].clone());
                    self.queue.enqueue(self.liked.tracks[1].clone());
                    self.page = Page::Queue;
                }
                self.home_stations = stations;
                self.home_retrospectives = retrospectives;
                self.home_playlists = playlists;
                self.resolve_covers();
            }
            Outcome::Library(cards) => {
                self.library = cards;
                self.resolve_covers();
            }
            Outcome::Detail(detail) => {
                // Recolhidos aqui, e nao mais abaixo, para que a lista vazia
                // tambem os apague: um pedido silencioso que sobrevivesse faria
                // a proxima lista aberta pelo usuario nao mostrar a tela.
                let silent = std::mem::take(&mut self.detail_silent);
                let pending_play = self.pending_liked_play.take();

                if detail.tracks.is_empty() && detail.cards.is_empty() {
                    self.status =
                        format!("{} nao tem nada que o Morune consiga tocar.", detail.title);
                    return;
                }

                // Abrir e um comeco de leitura, nao de reproducao: filtro e
                // ordenacao da lista anterior nao valem para esta.
                self.detail_filter.clear();
                self.detail_sort = SortBy::Original;
                self.detail_desc = false;
                self.detail_loading = false;
                self.detail_complete_requested = false;
                self.detail_pending_play = None;
                self.detail = Some(*detail);
                self.detail_from = self.page;
                self.status.clear();

                // Pedida so para encher a fila: a lista carrega, a tela nao
                // muda. Se o usuario abrir o detalhe depois, `detail_from` ja
                // aponta para onde ele estava e o botao de voltar funciona.
                if silent {
                    if let Some(id) = pending_play {
                        self.play_opened_collection_from(&id);
                    }
                } else {
                    self.page = Page::Detail;
                }

                self.resolve_detail_cover();
                self.resolve_detail_cards();
            }
            Outcome::DetailMore {
                source,
                tracks,
                total,
                has_more,
            } => {
                self.detail_loading = false;
                let queue_can_follow = self.detail_filter.is_empty()
                    && self.detail_sort == SortBy::Original
                    && self.detail.as_ref().is_some_and(|detail| {
                        self.queue.origin() == &detail.origin
                            && self.queue.len() == detail.tracks.len()
                    });

                let mut accepted = false;
                if let Some(detail) = &mut self.detail {
                    if detail.source.as_ref() == Some(&source) {
                        detail.tracks.extend(tracks.iter().cloned());
                        detail.total_tracks = total.or(detail.total_tracks);
                        detail.has_more = has_more;
                        accepted = true;
                    }
                }

                if accepted && queue_can_follow {
                    self.queue.append_context(tracks);
                }

                if accepted && self.detail_complete_requested && has_more {
                    self.request_detail_more();
                } else if accepted && self.detail_complete_requested {
                    self.detail_complete_requested = false;
                    self.finish_pending_detail_play();
                }
            }
            Outcome::DetailMoreFailed { source, message } => {
                if self
                    .detail
                    .as_ref()
                    .and_then(|detail| detail.source.as_ref())
                    == Some(&source)
                {
                    self.detail_loading = false;
                    self.detail_complete_requested = false;
                    self.detail_pending_play = None;
                    self.status = format!("Nao consegui carregar mais faixas. {message}");
                }
            }
            Outcome::Context {
                origin,
                title,
                tracks,
            } => {
                if tracks.is_empty() {
                    self.status = format!("{title} nao tem nada que o Morune consiga tocar.");
                    return;
                }
                self.status = format!("Tocando {title}.");
                self.autoplay_seed = None;
                self.queue.set_context(origin, tracks, Some(0));
                self.play_current();
            }
            Outcome::Failed(message) => {
                // A tela que falhou pode ser pedida de novo: sem soltar as
                // marcas, voltar a ela mostraria a lista vazia para sempre.
                self.home_requested = false;
                self.library_requested = false;
                self.searching = false;
                // O pedido silencioso morre com a falha. Deixa-lo de pe faria a
                // proxima lista que o usuario abrisse ser tocada sozinha.
                self.detail_silent = false;
                self.pending_liked_play = None;
                self.status = message;
            }
        }
        self.resolve_track_covers();
    }

    fn apply_autoplay(&mut self, outcome: AutoplayOutcome) {
        match outcome {
            AutoplayOutcome::Ready { seed, tracks }
                if self.autoplay_seed.as_ref() == Some(&seed) && self.queue.current().is_none() =>
            {
                self.autoplay_seed = None;
                if let Some(track) = self.queue.append_and_select(tracks).cloned() {
                    self.status = "Radio continuando a fila.".into();
                    self.send(PlayerCommand::Load {
                        track,
                        start_paused: false,
                    });
                    self.resolve_track_covers();
                } else {
                    self.status = "O radio nao encontrou faixas novas.".into();
                }
            }
            AutoplayOutcome::Failed(message) if self.autoplay_seed.take().is_some() => {
                self.status = format!("Fim da fila. {message}");
            }
            _ => {}
        }
    }

    fn apply_library(&mut self, outcome: LibraryOutcome) {
        self.liked_pending.remove(&outcome.id);
        let track = self.find_track(&outcome.id);
        match outcome.result {
            Ok(()) if outcome.saved => {
                self.liked_ids.insert(outcome.id.clone());
                if let Some(track) = track {
                    self.liked.tracks.retain(|item| item.id != outcome.id);
                    self.liked.tracks.insert(0, track.clone());
                    self.liked
                        .tracks
                        .truncate(crate::browse::SHELF_TRACKS as usize);
                    self.status = format!("{} foi adicionada as Musicas curtidas.", track.name);
                } else {
                    self.status = "Faixa adicionada as Musicas curtidas do Spotify.".into();
                }
            }
            Ok(()) => {
                self.liked_ids.remove(&outcome.id);
                self.liked.tracks.retain(|item| item.id != outcome.id);
                if let Some(detail) = &mut self.detail {
                    if detail.title == crate::browse::LIKED_TITLE {
                        detail.tracks.retain(|item| item.id != outcome.id);
                        detail.total_tracks =
                            detail.total_tracks.map(|total| total.saturating_sub(1));
                    }
                }
                self.status = track
                    .map(|track| format!("{} foi removida das Musicas curtidas.", track.name))
                    .unwrap_or_else(|| "Faixa removida das Musicas curtidas do Spotify.".into());
            }
            Err(message) => {
                self.status = format!("Nao consegui atualizar o Spotify. {message}");
            }
        }
    }

    /// Guarda o texto digitado no filtro da barra lateral.
    pub fn set_playlist_filter(&mut self, texto: &str) {
        self.playlist_filter = texto.trim().to_lowercase();
    }

    /// Playlists da barra lateral, com as abertas recentemente primeiro.
    ///
    /// Playlists ainda sem historico preservam a ordem do provedor. Assim a
    /// personalizacao anterior nao e perdida e a lista so muda depois de uma
    /// acao explicita do usuario.
    fn sidebar_playlists(&self) -> Vec<&Card> {
        let playlists = playlists_by_recent(
            &self.home_playlists,
            &self.config.navigation.recent_playlists,
        );

        std::iter::once(&self.liked_card)
            .chain(playlists)
            .filter(|c| {
                self.playlist_filter.is_empty()
                    || c.title.to_lowercase().contains(&self.playlist_filter)
            })
            .collect()
    }

    /// Faixas da tela de detalhe, filtradas e ordenadas.
    ///
    /// Ordenar por texto usa comparacao sem diferenciar maiuscula: uma lista
    /// onde "Zebra" vem antes de "abelha" nao parece ordenada para ninguem.
    fn detail_tracks(&self) -> Vec<&Track> {
        let Some(detail) = &self.detail else {
            return Vec::new();
        };

        let mut tracks: Vec<&Track> = detail
            .tracks
            .iter()
            .filter(|t| self.matches_detail_filter(t))
            .collect();

        match self.detail_sort {
            SortBy::Original => {}
            SortBy::Title => tracks.sort_by_key(|t| t.name.to_lowercase()),
            SortBy::Artist => tracks.sort_by_key(|t| primeiro_artista(t).to_lowercase()),
            SortBy::Album => tracks.sort_by_key(|t| nome_do_album(t).to_lowercase()),
            SortBy::Duration => tracks.sort_by_key(|t| t.duration),
        }

        // A ordem original ja e uma escolha de quem montou a lista; inverte-la
        // nao ordena nada.
        if self.detail_desc && self.detail_sort != SortBy::Original {
            tracks.reverse();
        }

        tracks
    }

    /// `true` quando a faixa combina com o que foi digitado.
    ///
    /// Procura em titulo, artista e album de uma vez: quem digita o nome de um
    /// artista quer as faixas dele, e nao uma tela vazia porque o campo era o
    /// errado.
    fn matches_detail_filter(&self, track: &Track) -> bool {
        if self.detail_filter.is_empty() {
            return true;
        }
        let alvo = &self.detail_filter;
        track.name.to_lowercase().contains(alvo)
            || primeiro_artista(track).to_lowercase().contains(alvo)
            || nome_do_album(track).to_lowercase().contains(alvo)
    }

    pub fn set_detail_filter(&mut self, texto: &str) {
        self.detail_filter = texto.trim().to_lowercase();
        if !self.detail_filter.is_empty() && self.detail_has_more() {
            self.complete_detail();
        }
    }

    /// Escolhe o criterio de ordenacao.
    ///
    /// Escolher o mesmo criterio de novo inverte o sentido, que e o que a
    /// pessoa espera ao clicar duas vezes no mesmo lugar.
    pub fn set_detail_sort(&mut self, criterio: i32) {
        let novo = SortBy::from_i32(criterio);
        if novo == self.detail_sort && novo != SortBy::Original {
            self.detail_desc = !self.detail_desc;
        } else {
            self.detail_sort = novo;
            self.detail_desc = false;
        }
        if self.detail_sort != SortBy::Original && self.detail_has_more() {
            self.complete_detail();
        }
    }

    /// Acao explicita e alternativa de teclado ao carregamento automatico no
    /// fim da rolagem.
    pub fn load_more_detail(&mut self) {
        self.request_detail_more();
    }

    /// Fecha a tela de detalhe e volta de onde ela foi aberta.
    pub fn close_detail(&mut self) {
        self.page = self.detail_from;
        self.detail = None;
        self.detail_loading = false;
        self.detail_complete_requested = false;
        self.detail_pending_play = None;
    }

    /// Toca a lista aberta a partir da primeira faixa visivel.
    ///
    /// "Visivel" e deliberado: com filtro aplicado, tocar tem de tocar o que
    /// esta na tela, e nao a lista inteira que o usuario acabou de esconder.
    pub fn play_detail(&mut self) {
        if self.detail_has_more()
            && (!self.detail_filter.is_empty() || self.detail_sort != SortBy::Original)
        {
            self.detail_pending_play = Some(PendingDetailPlay::First);
            self.complete_detail();
            return;
        }
        self.play_detail_from(0);
        if self.detail_has_more() {
            self.complete_detail();
        }
    }

    /// Toca a lista aberta a partir de uma faixa.
    ///
    /// A fila recebe a lista **como esta na tela** -- filtrada e ordenada --
    /// porque e isso que a pessoa esta vendo quando aperta. Tocar a ordem
    /// original depois de ordenar seria ignorar o que ela acabou de pedir.
    fn play_detail_from(&mut self, index: usize) {
        let Some(detail) = &self.detail else { return };
        let origin = detail.origin.clone();
        let title = detail.title.clone();

        let tracks: Vec<Track> = self.detail_tracks().into_iter().cloned().collect();
        if tracks.is_empty() {
            return;
        }

        self.status = format!("Tocando {title}.");
        self.queue.set_context(origin, tracks, Some(index));
        self.play_current();
    }

    /// Toca a partir da faixa clicada na tela de detalhe.
    ///
    /// A posicao e procurada na lista **visivel**: clicar na terceira linha tem
    /// de tocar a terceira linha, e nao a terceira da lista original que o
    /// filtro escondeu.
    pub fn activate_detail(&mut self, tag: &str) {
        // A linha chega com a forma completa -- `track/spotify:<id>` --, e nao
        // com o id cru: comparar com o id direto nunca casava, e o clique saia
        // sem tocar nada. Passar pelo `parse` e o que garante que os dois lados
        // falem a mesma lingua, hoje e quando a forma mudar.
        let Some(target) = Target::parse(tag) else {
            return;
        };
        let Target::Track(alvo) = target else {
            let Some(browse) = self.session.browse_mut() else {
                self.status = "Backend do Spotify indisponivel nesta maquina.".into();
                return;
            };
            browse.open(target);
            self.status = "Carregando...".into();
            return;
        };

        let Some(index) = self.detail_tracks().iter().position(|t| t.id == alvo) else {
            return;
        };

        if self.detail_has_more()
            && (!self.detail_filter.is_empty() || self.detail_sort != SortBy::Original)
        {
            self.detail_pending_play = Some(PendingDetailPlay::Track(alvo));
            self.complete_detail();
            return;
        }
        self.play_detail_from(index);
        if self.detail_has_more() {
            self.complete_detail();
        }
    }

    fn detail_has_more(&self) -> bool {
        self.detail.as_ref().is_some_and(|detail| detail.has_more)
    }

    /// Pede uma unica pagina. Retorna sem efeito enquanto outra esta em voo.
    fn request_detail_more(&mut self) {
        if self.detail_loading {
            return;
        }
        let Some(detail) = self.detail.as_ref() else {
            return;
        };
        if !detail.has_more {
            return;
        }
        let Some(source) = detail.source.clone() else {
            return;
        };
        let offset = detail.tracks.len() as u32;
        let Some(browse) = self.session.browse_mut() else {
            self.status = "Backend do Spotify indisponivel nesta maquina.".into();
            return;
        };
        if browse.load_more(source, offset) {
            self.detail_loading = true;
        }
    }

    /// Completa a lista em lotes. A interface continua responsiva entre cada
    /// resposta e mostra o progresso no lugar do antigo aviso de recorte.
    fn complete_detail(&mut self) {
        if !self.detail_has_more() {
            self.finish_pending_detail_play();
            return;
        }
        self.detail_complete_requested = true;
        self.request_detail_more();
    }

    /// Comeca a tocar a colecao recem-aberta a partir de uma faixa.
    ///
    /// A faixa quase sempre esta na primeira pagina -- a prateleira do Inicio
    /// mostra as curtidas mais recentes, que sao as primeiras da colecao --, e
    /// nesse caso o som comeca sem esperar o resto. Quando nao esta, a
    /// reproducao aguarda a lista completar em vez de tocar a faixa errada.
    fn play_opened_collection_from(&mut self, id: &TrackId) {
        match self
            .detail_tracks()
            .iter()
            .position(|track| track.id == *id)
        {
            Some(index) => {
                self.play_detail_from(index);
                if self.detail_has_more() {
                    self.complete_detail();
                }
            }
            None => {
                self.detail_pending_play = Some(PendingDetailPlay::Track(id.clone()));
                self.complete_detail();
            }
        }
    }

    fn finish_pending_detail_play(&mut self) {
        match self.detail_pending_play.take() {
            Some(PendingDetailPlay::First) => self.play_detail_from(0),
            Some(PendingDetailPlay::Track(id)) => {
                if let Some(index) = self.detail_tracks().iter().position(|track| track.id == id) {
                    self.play_detail_from(index);
                }
            }
            None => {}
        }
    }

    /// Pede a capa da lista aberta.
    fn resolve_detail_cover(&mut self) {
        let Some(url) = self.detail.as_ref().map(|d| d.cover.clone()) else {
            return;
        };
        if url.is_empty() {
            return;
        }
        let Some(browse) = self.session.browse_mut() else {
            return;
        };
        let path = browse.cover(&url);
        if let Some(detail) = &mut self.detail {
            detail.cover_path = path;
        }
    }

    /// Resolve a capa de cada cartao visivel.
    ///
    /// O que ja esta em disco entra no mesmo quadro em que a lista aparece;
    /// o resto e pedido e chega depois, por [`AppState::poll_covers`].
    fn resolve_covers(&mut self) {
        let Some(browse) = self.session.browse_mut() else {
            return;
        };

        // Emprestar cada lista separadamente evita mover os cartoes so para
        // preencher um campo.
        browse.resolve_covers(&mut self.home_made_for_you);
        browse.resolve_covers(&mut self.home_stations);
        browse.resolve_covers(&mut self.home_retrospectives);
        browse.resolve_covers(&mut self.home_playlists);
        browse.resolve_covers(&mut self.library);
        browse.resolve_covers(&mut self.search_cards);
    }

    /// Pede a capa da faixa que esta tocando, se ela mudou.
    ///
    /// Separado de [`AppState::resolve_covers`] porque a origem e outra: a
    /// faixa tocando vem da fila, e nao de uma tela.
    fn resolve_now_cover(&mut self) {
        let url = self
            .queue
            .current()
            .and_then(|t| t.album.as_ref())
            .and_then(|a| a.images.best_for_width(PLAYER_ARTWORK_WIDTH))
            .map(|i| i.url.to_string())
            .unwrap_or_default();

        if url == self.now_cover.0 {
            return;
        }

        self.now_cover = (url.clone(), None);
        if url.is_empty() {
            return;
        }

        let Some(browse) = self.session.browse_mut() else {
            return;
        };
        self.now_cover.1 = browse.cover(&url);
    }

    /// Recolhe as capas que terminaram de baixar e liga cada uma ao cartao.
    ///
    /// Devolve `true` quando alguma chegou. Roda a cada 100 ms junto com o
    /// resto, entao o caminho comum -- nenhuma capa pronta -- sai daqui sem
    /// percorrer lista nenhuma.
    fn poll_covers(&mut self) -> bool {
        let Some(browse) = self.session.browse_mut() else {
            return false;
        };

        let prontas = browse.poll_artwork();
        if prontas.is_empty() {
            return false;
        }

        for ready in prontas {
            self.track_covers
                .insert(ready.url.clone(), ready.path.clone());
            if let Some(detail) = &mut self.detail {
                if detail.cover == ready.url {
                    detail.cover_path = Some(ready.path.clone());
                }
                for card in detail
                    .cards
                    .iter_mut()
                    .filter(|card| card.cover == ready.url)
                {
                    card.cover_path = Some(ready.path.clone());
                }
            }

            if ready.url == self.now_cover.0 {
                self.now_cover.1 = Some(ready.path.clone());
            }

            for lista in [
                &mut self.home_made_for_you,
                &mut self.home_stations,
                &mut self.home_retrospectives,
                &mut self.home_playlists,
                &mut self.library,
                &mut self.search_cards,
            ] {
                for card in lista.iter_mut().filter(|c| c.cover == ready.url) {
                    card.cover_path = Some(ready.path.clone());
                }
            }
        }

        true
    }

    fn resolve_detail_cards(&mut self) {
        let Some(detail) = &mut self.detail else {
            return;
        };
        let Some(browse) = self.session.browse_mut() else {
            return;
        };
        browse.resolve_covers(&mut detail.cards);
    }

    /// Resolve as capas pequenas das listas sem bloquear a interface.
    fn resolve_track_covers(&mut self) {
        let urls: Vec<String> = self
            .search
            .tracks
            .iter()
            .chain(self.liked.tracks.iter())
            .chain(self.detail.iter().flat_map(|detail| detail.tracks.iter()))
            .chain(self.queue.tracks().iter())
            .filter_map(track_cover_url)
            .collect();

        let Some(browse) = self.session.browse_mut() else {
            return;
        };
        for url in urls {
            if self.track_covers.contains_key(&url) {
                continue;
            }
            if let Some(path) = browse.cover(&url) {
                self.track_covers.insert(url, path);
            }
        }
    }

    /// Pede ao backend o que a tela aberta mostra, se ainda nao pediu.
    fn request_page_data(&mut self) {
        if !self.session.state().is_logged_in() {
            return;
        }

        let page = self.page;
        let (home_requested, library_requested) = (self.home_requested, self.library_requested);
        let Some(browse) = self.session.browse_mut() else {
            return;
        };

        match page {
            Page::Home if !home_requested => {
                browse.load_home();
                self.home_requested = true;
            }
            Page::Library if !library_requested => {
                browse.load_library();
                self.library_requested = true;
            }
            _ => {}
        }
    }

    /// Procura a faixa nas listas visiveis e devolve a lista inteira.
    ///
    /// Clona porque a fila fica dona do contexto; e o mesmo custo que a busca
    /// ja pagava, e vale a pena para o botao de proxima ter para onde ir.
    fn open_lists(
        &self,
        id: &morune_core::model::TrackId,
    ) -> Option<(QueueOrigin, Vec<Track>, usize)> {
        // So a busca. As curtidas ficam de fora de proposito: a prateleira do
        // Inicio tem um punhado de faixas, e trata-la como lista completa dava
        // uma fila de cinco musicas. Elas passam por `play_liked_collection`.
        let index = self.search.tracks.iter().position(|t| t.id == *id)?;
        Some((
            self.search.origin.clone(),
            self.search.tracks.clone(),
            index,
        ))
    }

    /// Toca uma faixa curtida com a colecao inteira como contexto.
    ///
    /// Abre "Musicas curtidas" por tras, sem tirar o usuario de onde ele esta:
    /// ele clicou numa musica, nao numa lista. A primeira pagina ja comeca a
    /// tocar e as seguintes entram na fila em lotes, pelo mesmo caminho que a
    /// tela de detalhe usa.
    fn play_liked_collection(&mut self, id: TrackId) {
        let Some(browse) = self.session.browse_mut() else {
            self.status = "Entre na sua conta do Spotify para tocar.".into();
            return;
        };
        browse.open(Target::Liked);
        self.detail_silent = true;
        self.pending_liked_play = Some(id);
        self.status = format!("Carregando {}...", crate::browse::LIKED_TITLE);
    }

    /// Carrega a faixa selecionada na fila e comeca a tocar.
    fn play_current(&mut self) {
        if let Some(track) = self.queue.current().cloned() {
            self.playing = true;
            self.seek_target = Some(Duration::ZERO);
            self.send(PlayerCommand::Load {
                track,
                start_paused: false,
            });
        }
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
                let seed = self.queue.current().map(|track| track.id.clone());
                if let Some(next) = self.queue.next(false).cloned() {
                    self.send(PlayerCommand::Load {
                        track: next,
                        start_paused: false,
                    });
                } else if self.config.playback.autoplay {
                    let requested = seed.is_some_and(|seed| {
                        let Some(browse) = self.session.browse_mut() else {
                            return false;
                        };
                        if browse.autoplay(seed.clone()) {
                            self.autoplay_seed = Some(seed);
                            true
                        } else {
                            false
                        }
                    });
                    self.status = if requested {
                        "Preparando o radio...".into()
                    } else {
                        "Fim da fila.".into()
                    };
                } else {
                    self.playing = false;
                    self.status = "Fim da fila.".into();
                }
                true
            }
            PlayerEvent::Error(message) => {
                self.status = message;
                true
            }
            // O motor e a verdade sobre play/pause. O campo otimista existe so
            // para o icone nao esperar o round-trip; quando o motor fala, ele
            // manda -- inclusive para desfazer um `Play` que nao pegou.
            PlayerEvent::StateChanged(state) => {
                self.playing = state == morune_core::PlaybackState::Playing;
                // Qualquer noticia do motor ja vem com o relogio dele acertado,
                // entao a posicao provisoria perdeu a razao de existir.
                self.seek_target = None;
                if self.playing {
                    self.preload_next();
                }
                true
            }
            PlayerEvent::TrackChanged(_) => true,
            // Chega no seek confirmado e na correcao de deriva, nao a cada
            // quadro: a librespot so corrige acima de um segundo de desvio.
            // Repintar aqui e o que tira a barra da posicao provisoria.
            PlayerEvent::Position { .. } => {
                self.seek_target = None;
                true
            }
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
        window
            .window()
            .set_size(slint::LogicalSize::new(width, height));
        window.window().set_maximized(self.config.window.maximized);
    }

    /// Mantem em memoria o ultimo estado escolhido pelo usuario. A gravacao
    /// fica para ocultar/fechar, evitando escrever no disco a cada pixel de um
    /// redimensionamento continuo.
    pub fn remember_window_state(&mut self, width: f32, height: f32, maximized: bool) {
        self.config.window.maximized = maximized;
        if !maximized
            && width.is_finite()
            && height.is_finite()
            && width >= 480.0
            && height >= 320.0
        {
            self.config.window.width = width;
            self.config.window.height = height;
        }
    }

    /// Rele a pasta de temas.
    ///
    /// So quando o conjunto muda -- instalar, duplicar, desfazer uma importacao
    /// ou pedir recarga. Trocar qual tema esta ativo nao precisa disto: a marca
    /// de ativo sai de `theme_id()` na hora de montar a lista.
    fn refresh_themes(&mut self) {
        self.themes = loader::discover(&self.paths.themes_dir());
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
        // Recarregar existe para pegar o que mudou no disco, entao a lista de
        // temas instalados tambem tem que ser relida aqui.
        self.refresh_themes();
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
                self.refresh_themes();
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

        // Primeiro abre o pacote numa pasta temporaria. Assim o usuario ve o
        // que vai instalar antes de qualquer tema existente ser substituido.
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let preview_dir = std::env::temp_dir().join(format!(
            "morune-theme-preview-{}-{nonce}",
            std::process::id()
        ));
        let preview = morune_theme::import_pack(&file, &preview_dir, false);
        let _ = std::fs::remove_dir_all(&preview_dir);
        let preview = match preview {
            Ok(preview) => preview,
            Err(e) => {
                tracing::warn!(error = %e, "importacao de tema recusada");
                self.status = format!("Pacote recusado: {e}");
                return;
            }
        };

        let destination = self.paths.theme_dir(&preview.manifest.id);
        let replacement = if destination.exists() {
            "\n\nJa existe um tema com este ID. Ele sera substituido, mas voce podera desfazer."
        } else {
            ""
        };
        let description = format!(
            "{}\nPor: {}\nVersao: {}\n\n{}{}",
            preview.manifest.name,
            if preview.manifest.author.is_empty() {
                "Autor nao informado"
            } else {
                preview.manifest.author.as_str()
            },
            if preview.manifest.version.is_empty() {
                "Nao informada"
            } else {
                preview.manifest.version.as_str()
            },
            if preview.manifest.description.is_empty() {
                "Sem descricao."
            } else {
                preview.manifest.description.as_str()
            },
            replacement
        );
        let confirmed = rfd::MessageDialog::new()
            .set_title("Importar este tema?")
            .set_description(description)
            .set_buttons(rfd::MessageButtons::YesNo)
            .show();
        if confirmed != rfd::MessageDialogResult::Yes {
            self.status = "Importacao cancelada; nada foi alterado.".into();
            return;
        }

        self.discard_undo();
        let previous_id = self.config.appearance.theme.clone();
        let backup = if destination.exists() {
            let path = self
                .paths
                .themes_dir()
                .join(format!(".{}.undo-import", preview.manifest.id));
            let _ = std::fs::remove_dir_all(&path);
            match std::fs::rename(&destination, &path) {
                Ok(()) => Some(path),
                Err(e) => {
                    self.status = format!("Nao foi possivel preparar a substituicao: {e}");
                    return;
                }
            }
        } else {
            None
        };

        match morune_theme::import_pack(&file, &self.paths.themes_dir(), false) {
            Ok(imported) => {
                let id = imported.manifest.id.clone();
                self.refresh_themes();
                self.select_theme(&id);
                self.status = format!(
                    "Tema {} importado e aplicado ({} arquivos).",
                    imported.manifest.name, imported.files_written
                );
                self.undo = Some(UndoAction::ThemeImport {
                    previous_id,
                    installed_id: id,
                    backup,
                });
            }
            Err(e) => {
                if let Some(backup) = backup {
                    let _ = std::fs::rename(backup, destination);
                }
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

    pub fn take_tray_hint(&mut self) -> bool {
        if self.config.window.close_to_tray && !self.config.window.tray_hint_shown {
            self.config.window.tray_hint_shown = true;
            true
        } else {
            false
        }
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

    pub fn set_start_with_windows(&mut self, on: bool) {
        match crate::startup::set_enabled(on) {
            Ok(()) => {
                self.start_with_windows = on;
                self.status = if on {
                    "O Morune vai iniciar em segundo plano com o Windows.".into()
                } else {
                    "O Morune nao vai mais iniciar com o Windows.".into()
                };
            }
            Err(error) => {
                self.start_with_windows = crate::startup::is_enabled();
                self.status = format!("Nao foi possivel alterar a inicializacao: {error}");
            }
        }
    }

    pub fn set_autoplay(&mut self, on: bool) {
        self.config.playback.autoplay = on;
        self.status = if on {
            "Autoplay ligado: o radio continua quando a fila acabar.".into()
        } else {
            "Autoplay desligado: a reproducao para no fim da fila.".into()
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
        (
            label,
            self.engine.snapshot().state == morune_core::PlaybackState::Playing,
        )
    }

    pub fn toggle_sidebar(&mut self) {
        self.overrides.sidebar_collapsed = !self.overrides.sidebar_collapsed;
    }

    // ---- navegacao ----

    pub fn navigate(&mut self, page: i32) {
        self.page = Page::from_i32(page);
        if self.page != Page::Detail {
            self.detail_loading = false;
            self.detail_complete_requested = false;
            self.detail_pending_play = None;
        }
        self.request_page_data();
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
        let query = query.trim();
        if query.is_empty() {
            self.search_query.clear();
            self.searching = false;
            self.search = TrackList::default();
            self.search_cards.clear();
            return;
        }
        self.search_query = query.to_string();
        // A busca depende do catalogo, que so responde depois do login. Sem
        // sessao, dizer isso e melhor que uma lista vazia sem explicacao.
        if !self.session.state().is_logged_in() {
            self.status = format!("Busca por \"{query}\" precisa de uma sessao ativa.");
            return;
        }

        let Some(browse) = self.session.browse_mut() else {
            self.status = "Backend do Spotify indisponivel nesta maquina.".into();
            return;
        };
        self.searching = true;
        self.search = TrackList::default();
        self.search_cards.clear();
        browse.search(query);
        self.status = format!("Buscando \"{query}\"...");
    }

    pub fn clear_status(&mut self) {
        self.status.clear();
    }

    pub fn undo_available(&self) -> bool {
        self.undo.is_some()
    }

    pub fn retry_last(&mut self) {
        self.retry_available = false;
        match self.page {
            Page::Search if !self.search_query.is_empty() => {
                let query = self.search_query.clone();
                self.search(&query);
            }
            Page::Home | Page::Library => self.request_page_data(),
            _ => {
                self.status = "Volte a abrir o item para tentar novamente.".into();
            }
        }
    }

    pub fn dismiss_recovery(&mut self) {
        self.discard_undo();
        self.retry_available = false;
    }

    /// Descarta a oportunidade de desfazer e limpa apenas o backup privado que
    /// o Morune criou para uma substituicao de tema ja confirmada.
    pub fn discard_undo(&mut self) {
        if let Some(UndoAction::ThemeImport {
            backup: Some(backup),
            ..
        }) = self.undo.take()
        {
            let _ = std::fs::remove_dir_all(backup);
        }
    }

    pub fn undo_last(&mut self) {
        let Some(action) = self.undo.take() else {
            self.status = "Nao ha nenhuma acao recente para desfazer.".into();
            return;
        };

        match action {
            UndoAction::QueueClear(tracks) => {
                let count = tracks.len();
                for track in tracks {
                    self.queue.enqueue(track);
                }
                self.status = format!("Fila restaurada: {count} faixas voltaram.");
            }
            UndoAction::ThemeImport {
                previous_id,
                installed_id,
                backup,
            } => {
                let installed = self.paths.theme_dir(&installed_id);
                if installed.exists() {
                    let _ = std::fs::remove_dir_all(&installed);
                }
                if let Some(backup) = backup {
                    if let Err(e) = std::fs::rename(backup, &installed) {
                        self.status = format!("Nao foi possivel restaurar o tema anterior: {e}");
                        return;
                    }
                }
                self.refresh_themes();
                self.select_theme(&previous_id);
                self.status = "Importacao desfeita; o tema anterior foi restaurado.".into();
            }
        }
    }

    /// Adiciona ou remove uma faixa das "Musicas curtidas" do Spotify.
    pub fn toggle_favorite(&mut self, tag: &str) {
        let Some(Target::Track(id)) = Target::parse(tag) else {
            self.status = "Nao reconheci a faixa que voce quer curtir.".into();
            return;
        };
        if !self.session.state().is_logged_in() {
            self.status = "Entre no Spotify para curtir esta faixa.".into();
            return;
        }
        if !self.liked_pending.insert(id.clone()) {
            return;
        }

        let saved = !self.liked_ids.contains(&id);
        let Some(browse) = self.session.browse_mut() else {
            self.liked_pending.remove(&id);
            self.status = "Spotify indisponivel nesta sessao.".into();
            return;
        };
        browse.set_track_saved(id, saved);
        self.status = if saved {
            "Adicionando as Musicas curtidas do Spotify...".into()
        } else {
            "Removendo das Musicas curtidas do Spotify...".into()
        };
    }

    fn find_track(&self, id: &morune_core::TrackId) -> Option<Track> {
        self.queue
            .tracks()
            .iter()
            .chain(self.search.tracks.iter())
            .chain(self.liked.tracks.iter())
            .chain(self.detail.iter().flat_map(|detail| detail.tracks.iter()))
            .find(|track| track.id == *id)
            .cloned()
    }

    pub fn queue_play_next(&mut self, tag: &str) {
        let Some(track) = self.track_from_tag(tag) else {
            return;
        };
        if self.queue.current().is_none() {
            self.queue.set_context(
                QueueOrigin::Custom("Fila manual".into()),
                vec![track.clone()],
                Some(0),
            );
            self.play_current();
            self.status = format!("Tocando {}.", track.name);
        } else {
            self.queue.play_next(track.clone());
            self.status = format!("{} tocara a seguir.", track.name);
        }
        self.resolve_track_covers();
    }

    pub fn queue_enqueue(&mut self, tag: &str) {
        let Some(track) = self.track_from_tag(tag) else {
            return;
        };
        self.queue.enqueue(track.clone());
        self.status = format!("{} foi adicionada ao fim da fila.", track.name);
        self.resolve_track_covers();
    }

    pub fn queue_remove(&mut self, index: i32) {
        let Some(track) = usize::try_from(index)
            .ok()
            .and_then(|index| self.queue.remove_from_user_queue(index))
        else {
            self.status = "Essa faixa ja nao esta mais na fila.".into();
            return;
        };
        self.status = format!("{} foi removida da fila.", track.name);
    }

    pub fn queue_play_manual(&mut self, index: i32) {
        let Some(track) = usize::try_from(index)
            .ok()
            .and_then(|index| self.queue.remove_from_user_queue(index))
        else {
            self.status = "Essa faixa ja nao esta mais na fila.".into();
            return;
        };
        self.queue.play_next(track.clone());
        if let Some(next) = self.queue.next(true).cloned() {
            self.send(PlayerCommand::Load {
                track: next,
                start_paused: false,
            });
            self.status = format!("Tocando {}.", track.name);
        }
    }

    pub fn queue_move(&mut self, from: i32, to: i32) {
        let moved = usize::try_from(from)
            .ok()
            .zip(usize::try_from(to).ok())
            .is_some_and(|(from, to)| self.queue.move_user_queue(from, to));
        if !moved {
            self.status = "Nao foi possivel mover essa faixa na fila.".into();
        }
    }

    pub fn queue_clear(&mut self) {
        let removed: Vec<_> = self.queue.user_queue().cloned().collect();
        let count = removed.len();
        self.queue.clear_user_queue();
        self.status = if count == 0 {
            "A fila manual ja estava vazia.".into()
        } else {
            self.discard_undo();
            self.undo = Some(UndoAction::QueueClear(removed));
            format!("Fila manual limpa: {count} faixas removidas.")
        };
    }

    fn track_from_tag(&mut self, tag: &str) -> Option<Track> {
        let Some(Target::Track(id)) = Target::parse(tag) else {
            self.status = "Nao reconheci a faixa escolhida.".into();
            return None;
        };
        let track = self.find_track(&id);
        if track.is_none() {
            self.status = "Essa faixa nao esta mais disponivel nesta tela.".into();
        }
        track
    }

    // ---- reproducao ----

    /// Toca o que a interface ativou: uma faixa, um album, uma playlist ou um
    /// artista.
    ///
    /// Faixa que ja esta em alguma lista aberta toca na hora, sem ida a rede.
    /// O resto vai ao catalogo, porque so o clique nao diz quais sao as outras
    /// faixas do album.
    pub fn play_track(&mut self, tag: &str) {
        let Some(target) = Target::parse(tag) else {
            self.status = "Nao reconheci o que voce clicou.".into();
            return;
        };
        self.autoplay_seed = None;
        let opened_playlist = matches!(&target, Target::Playlist(_)).then(|| target.tag());

        if let Target::Track(id) = &target {
            // Na fila: e so pular para ela, mantendo o contexto que ja estava
            // tocando.
            if let Some(index) = self.queue.tracks().iter().position(|t| t.id == *id) {
                self.queue.jump_to(index);
                self.play_current();
                return;
            }

            // Curtida vinda da prateleira do Inicio: o contexto certo e a
            // colecao inteira, e nao as poucas faixas que cabem na prateleira.
            if self.liked.tracks.iter().any(|track| track.id == *id) {
                self.play_liked_collection(id.clone());
                return;
            }

            // Numa lista aberta: a lista inteira vira o contexto, para que
            // "proxima" continue por ela e nao pare na primeira faixa.
            if let Some((origin, tracks, index)) = self.open_lists(id) {
                self.queue.set_context(origin, tracks, Some(index));
                self.play_current();
                return;
            }
        }

        let Some(browse) = self.session.browse_mut() else {
            self.status = "Entre na sua conta do Spotify para tocar.".into();
            return;
        };
        browse.open(target);
        if let Some(tag) = opened_playlist {
            if remember_recent_playlist(&mut self.config.navigation.recent_playlists, tag) {
                self.save_config();
            }
        }
        self.status = "Carregando...".into();
    }

    /// `true` quando a barra esta mostrando reproducao em andamento.
    ///
    /// E a intencao ja refletida na tela, nao o retrato do motor: quem le isto
    /// quer saber se ha algo se movendo para o usuario.
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Posicao a mostrar: a de um seek ainda nao confirmado, ou a do motor.
    fn shown_position(&self, snapshot: &morune_core::playback::PlayerSnapshot) -> Duration {
        self.seek_target.unwrap_or(snapshot.position)
    }

    pub fn toggle_play(&mut self) {
        // Sem faixa nao ha o que alternar, e deixar o icone virar "pausar" sem
        // som nenhum seria mentir para quem clicou.
        if self.queue.current().is_none() {
            return;
        }

        // Manda a intencao, e nao `TogglePlay`: a interface acaba de decidir o
        // que vai mostrar, entao ela e quem sabe o alvo. O motor deixa de ter
        // que ler o proprio retrato para descobrir, e as duas pontas nao podem
        // discordar quando dois cliques chegam juntos.
        let target = !self.playing;
        self.playing = target;
        self.send(if target {
            PlayerCommand::Play
        } else {
            PlayerCommand::Pause
        });
    }

    pub fn next_track(&mut self) {
        if let Some(track) = self.queue.next(true).cloned() {
            self.playing = true;
            self.seek_target = Some(Duration::ZERO);
            self.send(PlayerCommand::Load {
                track,
                start_paused: false,
            });
        } else {
            self.playing = false;
            self.send(PlayerCommand::Stop);
        }
    }

    /// Duracao tocada a partir da qual "anterior" reinicia em vez de voltar.
    ///
    /// A conta e a de sempre nos tocadores: no comeco da faixa o gesto quer a
    /// faixa passada; depois disso quer ouvir esta de novo.
    const RESTART_THRESHOLD: Duration = Duration::from_secs(3);

    pub fn previous_track(&mut self) {
        // Passado o comeco da faixa, ou sem historico para onde voltar, o botao
        // reinicia. Antes ele simplesmente nao fazia nada na primeira faixa da
        // fila, e um botao que nao responde parece um botao lento.
        let played = self.shown_position(&self.engine.snapshot());
        if played >= Self::RESTART_THRESHOLD {
            self.restart_current();
            return;
        }

        if let Some(track) = self.queue.previous().cloned() {
            self.playing = true;
            self.seek_target = Some(Duration::ZERO);
            self.send(PlayerCommand::Load {
                track,
                start_paused: false,
            });
        } else {
            self.restart_current();
        }
    }

    fn restart_current(&mut self) {
        if self.queue.current().is_none() {
            return;
        }
        self.seek_target = Some(Duration::ZERO);
        self.send(PlayerCommand::Seek(Duration::ZERO));
    }

    pub fn seek(&mut self, progress: f32) {
        let duration = self.queue.current().map(|t| t.duration).unwrap_or_default();
        let target = duration.mul_f32(progress.clamp(0.0, 1.0));
        // Guardado ate o motor confirmar: ele roda noutra thread, e o retrato
        // lido logo abaixo ainda traria a posicao velha.
        self.seek_target = Some(target);
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

    /// Pede ao motor que adiante a proxima faixa da fila.
    ///
    /// Chamado quando a atual **comeca a tocar**, e nao quando ela e carregada:
    /// duas faixas baixando ao mesmo tempo disputariam banda justamente no
    /// momento em que a que o usuario esta esperando precisa dela.
    ///
    /// Sem isto, toda troca de faixa paga chave de audio, CDN e decodificador
    /// -- mediana de 588 ms e p90 de 996 ms medidos em uso real. Vale tanto
    /// para o botao de proxima quanto para a emenda no fim da musica.
    fn preload_next(&self) {
        let Some(next) = self.queue.upcoming(1).first().map(|track| (*track).clone()) else {
            return;
        };
        // Nao passa por `send`: adiantar e otimizacao, e falhar nisso nao pode
        // escrever nada na barra de status.
        let _ = self.engine.send(PlayerCommand::Preload(next));
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
        self.playing = false;
        self.seek_target = None;
        self.autoplay_seed = None;
        // Busca, inicio e biblioteca sao da conta que saiu: deixa-los na tela
        // mostraria a playlist de alguem que nao esta mais conectado.
        self.search = TrackList::default();
        self.liked = TrackList::default();
        self.liked_ids.clear();
        self.liked_pending.clear();
        self.home_made_for_you.clear();
        self.home_stations.clear();
        self.home_retrospectives.clear();
        self.home_playlists.clear();
        self.library.clear();
        self.search = TrackList::default();
        self.search_cards.clear();
        self.search_query.clear();
        self.searching = false;
        self.detail = None;
        self.detail_loading = false;
        self.detail_complete_requested = false;
        self.detail_pending_play = None;
        self.detail_silent = false;
        self.pending_liked_play = None;
        self.home_requested = false;
        self.library_requested = false;
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
        window.set_undo_available(self.undo_available());
        window.set_retry_available(self.retry_available);
        window.set_logged_in(self.session.state().is_logged_in());
        window.set_account_name(SharedString::from(self.session.state().account_name()));
        window.set_dev_mode(self.config.developer.enabled);
        window.set_close_to_tray(self.config.window.close_to_tray);
        window.set_start_with_windows(self.start_with_windows);
        window.set_autoplay(self.config.playback.autoplay);
        window.set_search_query(self.search_query.as_str().into());
        window.set_searching(self.searching);

        self.push_playback(window);

        let current = self.queue.current();
        window.set_sidebar_playlists(card_refs(&self.sidebar_playlists()));

        if let Some(detail) = &self.detail {
            window.set_detail_title(detail.title.as_str().into());
            window.set_detail_subtitle(detail.subtitle.as_str().into());
            window.set_detail_kind(detail.kind.as_str().into());
            window.set_detail_cover(cover_image(detail.cover_path.as_deref()));
            window.set_detail_tracks(track_rows(
                self.detail_tracks(),
                current,
                &self.track_covers,
                &self.liked_ids,
            ));
            window.set_detail_sort(self.detail_sort as i32);
            window.set_detail_descending(self.detail_desc);
            window.set_detail_filtered(!self.detail_filter.is_empty());
            window.set_detail_has_more(detail.has_more);
            window.set_detail_loading(self.detail_loading);
            window.set_detail_loaded_count(detail.tracks.len() as i32);
            window.set_detail_items(card_items(&detail.cards));
        }
        window.set_queue_manual_tracks(track_rows(
            self.queue.user_queue().take(200).collect(),
            current,
            &self.track_covers,
            &self.liked_ids,
        ));
        window.set_queue_context_tracks(track_rows(
            self.queue.upcoming_context(200),
            current,
            &self.track_covers,
            &self.liked_ids,
        ));
        window.set_themes(self.theme_items());
        window.set_diagnostics(self.diagnostics());
        window.set_home_made_for_you(card_items(&self.home_made_for_you));
        window.set_home_liked(track_rows(
            self.liked.tracks.iter().collect(),
            current,
            &self.track_covers,
            &self.liked_ids,
        ));
        window.set_home_playlists(card_items(&self.home_playlists));
        window.set_home_stations(card_items(&self.home_stations));
        window.set_home_retrospectives(card_items(&self.home_retrospectives));
        window.set_library_items(card_items(&self.library));
        window.set_search_items(card_items(&self.search_cards));
        window.set_search_tracks(track_rows(
            self.search.tracks.iter().collect(),
            current,
            &self.track_covers,
            &self.liked_ids,
        ));
    }

    /// Espelha so a barra de reproducao.
    ///
    /// Separado de [`AppState::push_to_ui`] porque este e o caminho que o
    /// usuario sente: volume, seek e play/pause chamam so isto. Sao propriedades
    /// escalares -- nenhum `VecModel` reconstruido, nenhuma linha de faixa, nada
    /// de disco -- entao pode rodar a cada quadro de arraste e a cada tique do
    /// relogio de progresso sem aparecer no medidor de CPU.
    pub fn push_playback(&self, window: &ui::AppWindow) {
        let snapshot = self.engine.snapshot();
        let position = self.shown_position(&snapshot);
        let current = self.queue.current();
        let duration = current.map(|t| t.duration).unwrap_or(snapshot.duration);

        window.set_has_track(current.is_some());
        window.set_now_cover(cover_image(self.now_cover.1.as_deref()));
        window.set_playing(self.playing);
        window.set_progress(if duration.is_zero() {
            0.0
        } else {
            (position.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
        });
        window.set_elapsed(format_time(position).into());
        window.set_total(format_time(duration).into());
        window.set_volume(self.volume);
        window.set_shuffle(self.queue.shuffle());
        window.set_repeat(match self.queue.repeat() {
            RepeatMode::Off => 0,
            RepeatMode::All => 1,
            RepeatMode::One => 2,
        });
        window.set_now_title(current.map(|t| t.name.as_ref()).unwrap_or("").into());
        window.set_now_artist(current.map(|t| t.artists_line()).unwrap_or_default().into());
        window.set_now_id(
            current
                .map(|track| Target::Track(track.id.clone()).tag())
                .unwrap_or_default()
                .into(),
        );
        window.set_now_favorite(current.is_some_and(|track| self.liked_ids.contains(&track.id)));
    }

    fn theme_items(&self) -> ModelRc<ui::ThemeItem> {
        let active = self.theme_id();
        let items: Vec<ui::ThemeItem> = self
            .themes
            .iter()
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

/// `true` quando a mensagem na tela ja pode sumir sozinha.
///
/// Separado do estado para a regra ficar sob teste: `has_action` marca a
/// mensagem que carrega "Desfazer" ou "Tentar novamente", e essa nunca expira
/// -- e o unico lugar de onde a acao pode ser feita.
fn status_expired(age: Duration, has_action: bool, timeout: Duration) -> bool {
    !has_action && age >= timeout
}

fn track_rows(
    tracks: Vec<&Track>,
    current: Option<&Track>,
    covers: &HashMap<String, std::path::PathBuf>,
    liked_ids: &HashSet<TrackId>,
) -> ModelRc<ui::TrackRow> {
    let rows: Vec<ui::TrackRow> = tracks
        .into_iter()
        .map(|t| ui::TrackRow {
            id: Target::Track(t.id.clone()).tag().into(),
            title: t.name.as_ref().into(),
            artist: t.artists_line().into(),
            album: t
                .album
                .as_ref()
                .map(|a| a.name.as_ref())
                .unwrap_or("")
                .into(),
            cover: cover_image(
                track_cover_url(t)
                    .as_deref()
                    .and_then(|url| covers.get(url))
                    .map(|p| p.as_path()),
            ),
            duration: format_time(t.duration).into(),
            playable: t.playable,
            playing: current.is_some_and(|c| c.id == t.id),
            favorite: liked_ids.contains(&t.id),
        })
        .collect();
    ModelRc::new(VecModel::from(rows))
}

fn track_cover_url(track: &Track) -> Option<String> {
    track
        .album
        .as_ref()?
        .images
        .best_for_width(TRACK_ROW_ARTWORK_WIDTH)
        .map(|image| image.url.to_string())
}

/// Uma lista de faixas visivel numa tela, com a origem que ela dara a fila.
#[derive(Debug, Default)]
struct TrackList {
    origin: QueueOrigin,
    tracks: Vec<Track>,
}

/// Largura em que a barra de reproducao desenha a capa.
///
/// Bem menor que a das grades: e um quadrado de poucas dezenas de pixels no
/// canto. Pedir a capa grande para desenhar isso seria baixar dez vezes mais
/// bytes do que a tela usa.
const PLAYER_ARTWORK_WIDTH: u32 = 64;

/// Largura pedida para capas desenhadas nas linhas de faixa.
const TRACK_ROW_ARTWORK_WIDTH: u32 = 64;

/// Igual a [`card_items`], para uma lista de referencias.
///
/// Existe porque o filtro da barra lateral devolve emprestimos, e copiar os
/// cartoes so para poder converte-los seria alocar a lista inteira a cada
/// tecla digitada.
/// Primeiro artista de uma faixa, ou vazio.
///
/// Ordenar por "artista" numa faixa com tres significa ordenar pelo primeiro,
/// que e o que aparece na linha.
fn primeiro_artista(track: &Track) -> &str {
    track
        .artists
        .first()
        .map(|a| a.name.as_ref())
        .unwrap_or_default()
}

fn nome_do_album(track: &Track) -> &str {
    track
        .album
        .as_ref()
        .map(|a| a.name.as_ref())
        .unwrap_or_default()
}

fn card_refs(cards: &[&Card]) -> ModelRc<ui::CardItem> {
    let items: Vec<ui::CardItem> = cards.iter().map(|c| card_item(c)).collect();
    ModelRc::new(VecModel::from(items))
}

fn card_item(c: &Card) -> ui::CardItem {
    ui::CardItem {
        id: c.tag.as_str().into(),
        title: c.title.as_str().into(),
        subtitle: c.subtitle.as_str().into(),
        cover: cover_image(c.cover_path.as_deref()),
    }
}

fn card_items(cards: &[Card]) -> ModelRc<ui::CardItem> {
    let items: Vec<ui::CardItem> = cards.iter().map(card_item).collect();
    ModelRc::new(VecModel::from(items))
}

thread_local! {
    /// Capas ja convertidas em `slint::Image`, indexadas pelo caminho.
    ///
    /// **Nao e cache de decodificacao** -- o Slint ja tem o dele. E cache da
    /// *chamada*: `Image::load_from_path` monta a chave do cache com
    /// `CachedPath::new`, que faz um `std::fs::metadata` **a cada chamada**,
    /// inclusive quando a imagem ja esta decodificada. Medido nesta maquina:
    /// 229 us por `stat`. Com a barra de volume espelhando a cada movimento do
    /// mouse -- e um mouse gamer reporta ate 1000 vezes por segundo -- isso
    /// sozinho consumia a thread da interface. Uma lista de fila de 400 linhas
    /// pagava 400 desses por espelhamento.
    ///
    /// Guardar pelo caminho e seguro porque o arquivo tem o hash do conteudo no
    /// nome: caminho igual significa imagem igual, e nunca ha versao nova no
    /// mesmo lugar. A falha tambem fica guardada, pelo mesmo motivo -- um JPEG
    /// truncado nao vai decodificar na proxima tentativa, e repetir o `stat`
    /// para descobrir isso e o que se quer evitar.
    static COVER_CACHE: std::cell::RefCell<HashMap<std::path::PathBuf, slint::Image>> =
        std::cell::RefCell::new(HashMap::new());
}

/// Carrega a capa do arquivo, ou devolve uma imagem vazia.
///
/// Imagem vazia nao e falta de tratamento: e o que faz o cartao mostrar o
/// bloco neutro no lugar, sem mudar o layout quando a capa de verdade chegar.
fn cover_image(path: Option<&std::path::Path>) -> slint::Image {
    let Some(path) = path else {
        return slint::Image::default();
    };

    if let Some(cached) = COVER_CACHE.with(|c| c.borrow().get(path).cloned()) {
        return cached;
    }

    let image = slint::Image::load_from_path(path).unwrap_or_else(|e| {
        // Arquivo truncado ou formato inesperado: o cartao fica sem capa e o
        // aplicativo segue. Trocar isto por `expect` derrubaria a tela por
        // causa de um JPEG ruim.
        tracing::debug!(path = %path.display(), error = ?e, "capa nao decodificou");
        slint::Image::default()
    });
    COVER_CACHE.with(|c| {
        c.borrow_mut().insert(path.to_path_buf(), image.clone());
    });
    image
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

/// Move uma playlist para o inicio do historico quando ela foi aberta.
/// Devolve `true` apenas quando ha algo novo para persistir.
fn remember_recent_playlist(recent: &mut Vec<String>, tag: String) -> bool {
    if recent.first() == Some(&tag) {
        return false;
    }
    recent.retain(|saved| saved != &tag);
    recent.insert(0, tag);
    recent.truncate(100);
    true
}

/// Ordena apenas o que tem historico. O sort estavel conserva a sequencia do
/// provedor para todas as playlists que ainda nao foram abertas no Morune.
fn playlists_by_recent<'a>(playlists: &'a [Card], recent: &[String]) -> Vec<&'a Card> {
    let positions: HashMap<&str, usize> = recent
        .iter()
        .enumerate()
        .map(|(position, tag)| (tag.as_str(), position))
        .collect();
    let mut ordered: Vec<&Card> = playlists.iter().collect();
    ordered.sort_by_key(|card| {
        positions
            .get(card.tag.as_str())
            .copied()
            .unwrap_or(usize::MAX)
    });
    ordered
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
    fn a_message_with_an_action_never_expires_on_its_own() {
        let timeout = Duration::from_secs(6);

        assert!(!status_expired(Duration::from_secs(1), false, timeout));
        assert!(status_expired(Duration::from_secs(6), false, timeout));

        // "Desfazer" e "Tentar novamente" vivem dentro do aviso: some-lo
        // tiraria do usuario uma escolha que ele ainda nao fez.
        assert!(!status_expired(Duration::from_secs(600), true, timeout));
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

    #[test]
    fn recent_playlist_moves_to_front_without_duplicates() {
        let mut recent = vec!["playlist/spotify:a".into(), "playlist/spotify:b".into()];

        assert!(remember_recent_playlist(
            &mut recent,
            "playlist/spotify:b".into()
        ));
        assert_eq!(recent, ["playlist/spotify:b", "playlist/spotify:a"]);
        assert!(!remember_recent_playlist(
            &mut recent,
            "playlist/spotify:b".into()
        ));
    }

    #[test]
    fn sidebar_puts_recent_first_and_preserves_the_rest() {
        let card = |id: &str| Card {
            tag: format!("playlist/spotify:{id}"),
            title: id.into(),
            subtitle: String::new(),
            cover: String::new(),
            cover_path: None,
        };
        let playlists = vec![card("a"), card("b"), card("c"), card("d")];
        let recent = vec!["playlist/spotify:c".into(), "playlist/spotify:a".into()];

        let ordered: Vec<&str> = playlists_by_recent(&playlists, &recent)
            .into_iter()
            .map(|item| item.title.as_str())
            .collect();

        assert_eq!(ordered, ["c", "a", "b", "d"]);
    }
}
