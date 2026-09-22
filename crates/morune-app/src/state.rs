//! Estado da aplicacao e as acoes que a interface dispara.
//!
//! Concentrar tudo aqui deixa `main.rs` sendo so fiacao, e mantem a interface
//! sem nenhuma decisao de produto: cada callback do Slint chama um metodo deste
//! tipo e depois pede um `push_to_ui`.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use morune_core::playback::{
    AudioSettings, NullEngine, PlaybackEngine, PlayerCommand, PlayerEvent,
};
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

/// "1 faixa" ou "N faixas".
///
/// Existe porque as mensagens montavam "{count} faixas" na mao, e uma fila com
/// uma faixa so dizia "1 faixas". E o tipo de detalhe que ninguem reporta como
/// defeito e todo mundo percebe.
fn faixas(quantidade: usize) -> String {
    if quantidade == 1 {
        "1 faixa".to_string()
    } else {
        format!("{quantidade} faixas")
    }
}

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

/// Um detalhe que ficou atras de outro detalhe aberto a partir de um cartao.
///
/// A tela de detalhe tambem pode mostrar outros albuns, playlists e artistas.
/// Guardar o estado de leitura impede que o botao Voltar troque para uma tela
/// de detalhe vazia e preserva o filtro e a ordenacao que a pessoa ja tinha
/// escolhido.
#[derive(Debug)]
struct DetailHistory {
    detail: crate::browse::Detail,
    from: Page,
    filter: String,
    sort: SortBy,
    desc: bool,
}

/// O que o botao "Tentar novamente" repete, e a qual mensagem ele pertence.
///
/// **Por que carrega a mensagem:** isto era um `bool` solto. Uma falha ligava a
/// marca, e qualquer coisa que escrevesse status depois -- um login concluido,
/// uma reconexao -- trocava a frase sem desligar o botao. O resultado era
/// "Conectado como fulano." com um "Tentar novamente" ao lado, oferecendo
/// repetir algo que ninguem sabia mais o que era. Pior: mensagem com acao nao
/// expira (ver [`AppState::expire_status`]), entao o aviso ficava pregado na
/// tela ate alguem fecha-lo a mao -- exatamente o defeito que o relogio de
/// expiracao existe para consertar.
///
/// Guardar a mensagem junto resolve isso sem tocar nos mais de quarenta lugares
/// que escrevem status: a oferta vale enquanto a frase que a criou estiver na
/// tela, e some sozinha quando outra a substitui.
#[derive(Debug, Clone, PartialEq)]
struct Retry {
    target: RetryTarget,
    /// Mensagem que a falha escreveu.
    message: String,
}

/// A requisicao que falhou, guardada para poder ser refeita.
///
/// Registrada quando o pedido **sai**, e nao deduzida da tela no momento do
/// clique: entre a falha e o clique da para navegar para outro lugar, e a
/// versao anterior repetia o que estivesse aberto em vez do que falhou.
#[derive(Debug, Clone, PartialEq)]
enum RetryTarget {
    /// Prateleiras do Inicio ou da Biblioteca.
    Page(Page),
    Search(String),
    /// Uma lista, na forma textual de [`Target::tag`].
    Detail(String),
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
    /// Fundo ja decodificado, reduzido e desfocado.
    ///
    /// Guardado no estado porque preparar custa caro e o resultado so muda
    /// quando o tema ou a escolha do usuario mudam -- refazer isso a cada
    /// `apply_theme_to` decodificaria um JPEG a cada recarga de tela.
    wallpaper: crate::wallpaper::Wallpaper,
    overrides: UserOverrides,
    /// Ha um aplicativo de outro processo ocupando a tela inteira na frente.
    ///
    /// Atualizado pelo vigia em [`crate::tela_cheia`], nunca lido do arquivo de
    /// configuracao: e estado do momento, nao preferencia.
    tela_cheia: bool,
    page: Page,
    /// De onde a Fila foi aberta, para `Esc` devolver o usuario ao lugar certo.
    page_before_queue: Page,
    /// Volume de antes de silenciar, para o mudo ser reversivel.
    volume_before_mute: f32,
    status: String,
    /// A mensagem que o relogio de expiracao esta contando, e desde quando.
    ///
    /// Comparar com `status` a cada leitura evita ter que lembrar de reiniciar
    /// o relogio nos mais de quarenta lugares que escrevem uma mensagem -- e um
    /// deles esquecido seria uma mensagem que nunca some, que e exatamente o
    /// defeito que isto conserta.
    status_seen: (String, Instant),
    undo: Option<UndoAction>,
    /// Oferta de repetir a ultima requisicao que falhou.
    retry: Option<Retry>,
    /// O que a requisicao em voo repetiria, se ela falhar.
    pending_retry: Option<RetryTarget>,
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
    /// Nomes dos dispositivos de saida, lidos do sistema.
    ///
    /// Memoizado pelo mesmo motivo dos temas: `push_to_ui` roda a cada clique e
    /// enumerar dispositivos de audio conversa com o driver. Relido ao entrar em
    /// Configuracoes, que e quando um fone recem-conectado precisa aparecer.
    output_devices: Vec<String>,
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
    /// Estado completo das "Músicas curtidas" do Spotify.
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
    /// Detalhes abertos antes do atual, para voltar por colecoes aninhadas.
    detail_history: Vec<DetailHistory>,
    /// Foto da conta: a URL pedida e o arquivo, quando ja chegou.
    ///
    /// Passa pelo mesmo cache de capas, e nao por um caminho proprio: e uma
    /// imagem pequena vinda do mesmo servidor, e um segundo downloader so para
    /// ela seria duplicar o cache, o descarte por LRU e o tratamento de falha.
    account_avatar: (String, Option<std::path::PathBuf>),
    /// Capa da faixa tocando: a URL pedida e o arquivo, quando ja chegou.
    ///
    /// Guardada separada dos cartoes porque a faixa tocando nao esta
    /// necessariamente em nenhuma tela aberta -- ela continua tocando com o
    /// usuario navegando por outra coisa.
    now_cover: (String, Option<std::path::PathBuf>),
    /// Cor dominante da capa que esta tocando, e de qual arquivo ela saiu.
    ///
    /// Guardada porque `push_playback` roda a cada 100 ms e a cor so muda
    /// quando a capa muda -- recalcular ali dentro reabriria a imagem dez vezes
    /// por segundo.
    now_tint: (Option<std::path::PathBuf>, Option<slint::Color>),
    /// As listas que a interface exibe, vivas entre um espelhamento e outro.
    listas: Listas,
    /// Capas pequenas das linhas, indexadas pela URL que o modelo da faixa traz.
    track_covers: HashMap<String, std::path::PathBuf>,
    /// Semente do pedido de autoplay em voo; impede uma resposta antiga de
    /// continuar uma fila que o usuario ja substituiu.
    autoplay_seed: Option<morune_core::TrackId>,
    /// `true` depois de a tela ter sido pedida ao backend, e nao depois de
    /// chegar: sem isso, ir e voltar numa tela lenta dispara uma requisicao por
    /// visita.
    home_requested: bool,
    /// `true` quando a Home recebeu uma resposta, inclusive uma resposta sem
    /// prateleiras. Separa "carregando" de "conta sem conteudo".
    home_loaded: bool,
    library_requested: bool,
    /// `true` quando a Biblioteca recebeu uma resposta, inclusive vazia.
    library_loaded: bool,
    /// Verificacao e download de versao nova. Toca a rede sozinho no maximo uma
    /// vez por dia (ver `Updater::auto_check`); o download continua sendo
    /// sempre por clique. Existe desde a abertura porque guardar o resultado da
    /// verificacao entre visitas a tela de configuracoes custa menos que
    /// refaze-la.
    updater: crate::update::Updater,
    /// Vigia o proprio executavel: uma instalacao nova por baixo de um processo
    /// que ficou dias aberto e invisivel de qualquer outro jeito.
    binary: crate::update::BinaryWatch,
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
            // O sistema so **tira** movimento, nunca devolve: quem desligou
            // "Mostrar animacoes no Windows" pediu isso a todo aplicativo, e um
            // tema com movimento nao pode desfazer o pedido. O caminho
            // contrario continua livre -- desligar aqui vale mesmo com a
            // animacao do sistema ligada.
            reduce_motion: config.appearance.reduce_motion || !system_animation_enabled(),
            sidebar_collapsed: false,
        };

        let mut queue = Queue::new();
        queue.set_shuffle(config.playback.shuffle);
        queue.set_repeat(config.playback.repeat);

        // A pasta do cache de capas e lida antes de `paths` ser movido para o
        // estado.
        let covers_dir = paths.artwork_cache_dir();
        let updates_dir = paths.updates_dir();
        let audio_cache_dir = paths.audio_cache_dir();
        // Lido antes de `config` ser movido para dentro do estado.
        let audio = audio_settings(&config.playback);
        let start_with_windows = crate::startup::is_enabled();

        let loaded = Self {
            volume: config.playback.volume,
            paths,
            config,
            theme,
            wallpaper: Default::default(),
            overrides,
            tela_cheia: false,
            page: Page::Home,
            page_before_queue: Page::Home,
            volume_before_mute: 0.0,
            status: String::new(),
            status_seen: (String::new(), Instant::now()),
            undo: None,
            retry: None,
            pending_retry: None,
            start_with_windows,
            queue,
            // Sem backend real ate haver login: o motor nulo aceita
            // preferencias e recusa reproducao, sem que a interface precise
            // tratar "sem motor" em lugar nenhum.
            engine: Arc::new(NullEngine::new("entre na sua conta para tocar musica")),
            session: Session::new(
                Arc::from(morune_storage::platform_store()),
                covers_dir,
                audio,
                &audio_cache_dir,
            ),
            player_events: None,
            themes,
            output_devices: morune_spotify::output_devices(),
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
            detail_history: Vec::new(),
            liked_card: Card {
                tag: crate::browse::Target::Liked.tag(),
                title: crate::browse::LIKED_TITLE.into(),
                subtitle: String::new(),
                cover: String::new(),
                cover_path: None,
            },
            account_avatar: (String::new(), None),
            now_cover: (String::new(), None),
            now_tint: (None, None),
            listas: Listas::default(),
            track_covers: HashMap::new(),
            autoplay_seed: None,
            home_requested: false,
            home_loaded: false,
            library_requested: false,
            library_loaded: false,
            updater: crate::update::Updater::new(updates_dir),
            binary: crate::update::BinaryWatch::new(),
        };

        let mut loaded = loaded;
        // Decodificar o fundo aqui, e nao na primeira pintura, e o que evita a
        // janela abrir com cor solida e trocar de aparencia meio segundo
        // depois.
        loaded.refresh_wallpaper();

        #[cfg(feature = "snapshot")]
        loaded.install_snapshot_detail_demo();
        #[cfg(feature = "snapshot")]
        loaded.install_snapshot_playback_demo();

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
                    name: "Álbum de exemplo".into(),
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
            kind: "Coleção".into(),
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

    /// Poe uma faixa tocando, para a verificacao visual do player.
    ///
    /// A barra de reproducao sem faixa mostra o triangulo de play desabilitado
    /// -- ou seja, o estado que menos importa conferir. Sem isto nao ha como
    /// capturar o pause, que e justamente o glifo mais dificil de acertar.
    #[cfg(feature = "snapshot")]
    fn install_snapshot_playback_demo(&mut self) {
        if std::env::var_os("MORUNE_SNAPSHOT_PLAYING").is_none() {
            return;
        }

        let track = Track {
            id: TrackId::spotify("snapshotnow"),
            name: "Nome de faixa razoavelmente longo".into(),
            artists: vec![morune_core::model::ArtistRef {
                id: morune_core::model::ArtistId::spotify("snapshotartist"),
                name: "Artista de exemplo".into(),
            }],
            album: Some(morune_core::model::AlbumRef {
                id: morune_core::model::AlbumId::spotify("snapshotalbum"),
                name: "Álbum de exemplo".into(),
                images: Default::default(),
            }),
            duration: Duration::from_secs(214),
            track_number: Some(1),
            disc_number: Some(1),
            explicit: false,
            playable: true,
        };
        self.queue
            .set_context(QueueOrigin::Custom("Snapshot".into()), vec![track], Some(0));
        self.playing = true;
        // Progresso parado no meio: uma barra vazia nao mostraria a cor de
        // preenchimento nem o puxador.
        self.seek_target = Some(Duration::from_secs(97));

        // Mensagem de status, para a verificacao visual pegar o aviso flutuante
        // junto com o resto da tela.
        if let Ok(msg) = std::env::var("MORUNE_SNAPSHOT_STATUS") {
            self.status = msg;
            self.status_seen = (self.status.clone(), Instant::now());
        }

        // Capa vinda de um arquivo local, para conferir o tint sem depender de
        // rede nem de conta.
        if let Some(cover) = std::env::var_os("MORUNE_SNAPSHOT_COVER") {
            let path = std::path::PathBuf::from(cover);
            if path.is_file() {
                self.now_cover = ("snapshot".into(), Some(path));
                self.refresh_tint();
            }
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
        let pending = self
            .session
            .is_busy()
            .then(|| self.session.poll())
            .flatten();
        if let Some(change) = pending {
            self.status = change.message;
            if let Some(engine) = change.engine {
                self.player_events = Some(engine.subscribe());
                self.engine = engine;
                self.restore_recreated_engine();
            }
            if self.session.state().is_logged_in() {
                // A tela aberta na hora do login precisa se preencher sozinha:
                // o usuario acabou de entrar e nao vai clicar em "Inicio" de
                // novo so para ver o que ja deveria estar la.
                self.home_requested = false;
                self.home_loaded = false;
                self.library_requested = false;
                self.library_loaded = false;
                self.request_page_data();
            }
            changed = true;
        }

        if let Some(outcome) = self.session.browse_mut().and_then(|b| b.poll()) {
            self.apply_browse(outcome);
            changed = true;
        }

        if let Some(outcome) = self.session.browse_mut().and_then(|b| b.poll_detail_more()) {
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

        // Sai daqui sem custo quando ninguem clicou em verificar: `poll` so
        // olha um `Option` vazio.
        self.updater.auto_check();
        changed |= self.updater.poll();

        if let Some(versao) = self.updater.take_anuncio() {
            self.status = format!("Versão {versao} disponível. Veja em Configurações.");
            changed = true;
        }

        // Uma vez por sessao, e no minuto em que acontece.
        if self.binary.poll() {
            self.status =
                "O Morune foi atualizado em disco. Feche e abra para usar a versão nova.".into();
            changed = true;
        }

        // Depois dos eventos do player: a troca de faixa acabou de ser
        // aplicada, entao a capa pedida aqui ja e a da faixa certa.
        self.resolve_now_cover();
        self.resolve_account_avatar();
        self.refresh_tint();

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

        let has_action = self.undo_available() || self.retry_available();
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
        // Uma resposta encerra o pedido em voo, seja qual for o desfecho: a
        // oferta de repetir so renasce logo abaixo, se esta resposta for uma
        // falha. Sem o `take`, uma falha antiga continuaria oferecendo repetir
        // um pedido que ja deu certo depois.
        let detail_more = matches!(
            &outcome,
            Outcome::DetailMore { .. } | Outcome::DetailMoreFailed { .. }
        );
        let pendente = (!detail_more).then(|| self.pending_retry.take()).flatten();
        if !detail_more {
            self.retry = None;
        }
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
                self.home_loaded = true;
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
                    origin: QueueOrigin::Custom("Músicas curtidas".into()),
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
                self.library_loaded = true;
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
                        format!("Nenhuma faixa de {} pode ser tocada aqui.", detail.title);
                    return;
                }

                if self.page == Page::Detail {
                    if let Some(previous) = self.detail.take() {
                        self.detail_history.push(DetailHistory {
                            detail: previous,
                            from: self.detail_from,
                            filter: std::mem::take(&mut self.detail_filter),
                            sort: self.detail_sort,
                            desc: self.detail_desc,
                        });
                    }
                } else {
                    // Uma nova entrada por Inicio, Busca ou Biblioteca inicia
                    // uma navegacao de detalhe independente da anterior.
                    self.detail_history.clear();
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

                self.borrow_cover_from_cards();
                self.resolve_detail_cover();
                self.resolve_detail_cards();
            }
            Outcome::DetailMore {
                source,
                tracks,
                total,
                has_more,
            } => {
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

                if accepted {
                    self.detail_loading = false;
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
                    self.status = format!("Não consegui carregar mais faixas. {message}");
                }
            }
            Outcome::Context {
                origin,
                title,
                tracks,
            } => {
                if tracks.is_empty() {
                    self.status = format!("Nenhuma faixa de {title} pode ser tocada aqui.");
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
                let home_failed = matches!(pendente.as_ref(), Some(RetryTarget::Page(Page::Home)));
                self.home_requested = false;
                if home_failed {
                    self.home_loaded = false;
                }
                let library_failed =
                    matches!(pendente.as_ref(), Some(RetryTarget::Page(Page::Library)));
                self.library_requested = false;
                if library_failed {
                    self.library_loaded = false;
                }
                self.searching = false;
                // O pedido silencioso morre com a falha. Deixa-lo de pe faria a
                // proxima lista que o usuario abrisse ser tocada sozinha.
                self.detail_silent = false;
                self.pending_liked_play = None;
                self.status = message;
                // A oferta nasce colada nesta frase: se outra mensagem a
                // substituir antes do clique, o botao sai junto.
                self.retry = pendente.map(|target| Retry {
                    target,
                    message: self.status.clone(),
                });
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
                    self.status = "Rádio continuando a fila.".into();
                    self.send(PlayerCommand::Load {
                        track,
                        start_paused: false,
                    });
                    self.resolve_track_covers();
                } else {
                    self.status = "O rádio não encontrou faixas novas.".into();
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
                    self.status = format!("{} foi adicionada às Músicas curtidas.", track.name);
                } else {
                    self.status = "Faixa adicionada às Músicas curtidas do Spotify.".into();
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
                    .map(|track| format!("{} foi removida das Músicas curtidas.", track.name))
                    .unwrap_or_else(|| "Faixa removida das Músicas curtidas do Spotify.".into());
            }
            Err(message) => {
                self.status = format!("Não consegui atualizar o Spotify. {message}");
            }
        }
    }

    /// Guarda o texto digitado no filtro da barra lateral.
    pub fn set_playlist_filter(&mut self, texto: &str) {
        self.playlist_filter = texto.trim().to_lowercase();
    }

    /// Playlists da barra lateral: fixadas primeiro, depois as abertas
    /// recentemente, depois o resto na ordem do provedor.
    ///
    /// Entra o `rootlist` inteiro, de qualquer `PlaylistKind`. Procurar "seus
    /// mais ouvidos" na lateral e o gesto normal de quem vem do Spotify, e
    /// deixar mixes e retrospectivas so nas prateleiras do Inicio fazia elas
    /// nao existirem para quem navega pela barra.
    ///
    /// Encadeia os quatro vetores em vez de alargar `home_playlists` porque
    /// `home_playlists` **tambem** e a prateleira "Suas playlists" do Inicio:
    /// mexer nele mudaria uma tela que nao era para mudar. E nao ha
    /// deduplicacao porque nao ha o que deduplicar -- `Browse::load_home`
    /// classifica cada playlist num unico balde.
    fn sidebar_playlists(&self) -> Vec<&Card> {
        let todas = self
            .home_playlists
            .iter()
            .chain(&self.home_made_for_you)
            .chain(&self.home_stations)
            .chain(&self.home_retrospectives);

        sidebar_order(
            &self.liked_card,
            todas,
            &self.config.navigation.pinned_playlists,
            &self.config.navigation.recent_playlists,
            &self.playlist_filter,
        )
    }

    /// Fixa ou desafixa uma playlist no topo da barra lateral.
    ///
    /// Fixar entra na frente das outras fixadas, e nao no fim: a lista salta
    /// para o topo no mesmo quadro do clique, e esse salto e a unica
    /// confirmacao de que a acao aconteceu.
    pub fn toggle_pin_playlist(&mut self, tag: &str) {
        let tag = tag.trim();
        if !matches!(
            crate::browse::Target::parse(tag),
            Some(crate::browse::Target::Playlist(_))
        ) {
            // As curtidas ja moram no topo por definicao, e nada mais chega
            // aqui: a barra lateral so oferece o menu para playlist.
            self.status = "So playlists podem ser fixadas no topo.".into();
            return;
        }

        // O nome sai antes do emprestimo mutavel de `self.config`.
        let nome = self
            .sidebar_playlists()
            .into_iter()
            .find(|card| card.tag == tag)
            .map(|card| card.title.clone())
            .unwrap_or_else(|| "Playlist".into());

        let fixadas = &mut self.config.navigation.pinned_playlists;
        self.status = if let Some(posicao) = fixadas.iter().position(|saved| saved == tag) {
            fixadas.remove(posicao);
            format!("{nome} não está mais fixada.")
        } else {
            fixadas.insert(0, tag.to_string());
            fixadas.truncate(PINNED_LIMIT);
            format!("{nome} fixada no topo.")
        };
        self.save_config();
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
        self.detail_loading = false;
        self.detail_complete_requested = false;
        self.detail_pending_play = None;

        if let Some(previous) = self.detail_history.pop() {
            self.detail = Some(previous.detail);
            self.detail_from = previous.from;
            self.detail_filter = previous.filter;
            self.detail_sort = previous.sort;
            self.detail_desc = previous.desc;
            self.page = Page::Detail;
        } else {
            self.page = self.detail_from;
            self.detail = None;
        }
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
                self.status = "Não foi possível iniciar o Spotify nesta máquina. Feche e abra o Morune para tentar de novo.".into();
                return;
            };
            browse.open(target);
            self.pending_retry = Some(RetryTarget::Detail(tag.to_string()));
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
            self.status = "Não foi possível iniciar o Spotify nesta máquina. Feche e abra o Morune para tentar de novo.".into();
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

    /// Empresta ao cabecalho a capa que os cartoes ja conhecem.
    ///
    /// **O defeito que isto conserta:** a mesma playlist aparecia com a capa
    /// certa na barra lateral e sem capa nenhuma no cabecalho, a vinte
    /// centimetros de distancia. Sao duas fontes diferentes -- o cartao vem do
    /// rootlist, que traz a imagem; o cabecalho vem de `Catalog::playlist`, que
    /// no protocolo interno devolve `PlaylistContents { name, track_ids }` e
    /// nada mais. Nao ha imagem para pedir, entao o cabecalho ficava vazio para
    /// sempre, sem que nada estivesse falhando.
    ///
    /// Copiar do cartao e o conserto certo aqui, e nao um remendo: e a mesma
    /// playlist, o dado ja esta em memoria e foi obtido pelo caminho que o
    /// carrega. Fazer o protobuf da playlist entregar a imagem seria melhor, e
    /// continua valendo -- mas exige mexer no parser, e nao pode ser condicao
    /// para o cabecalho parar de mentir.
    fn borrow_cover_from_cards(&mut self) {
        let Some(tag) = self
            .detail
            .as_ref()
            .filter(|d| d.cover.is_empty())
            .and_then(|d| d.source.as_ref().map(|alvo| alvo.tag()))
        else {
            return;
        };

        let achada = self
            .home_playlists
            .iter()
            .chain(&self.home_made_for_you)
            .chain(&self.home_stations)
            .chain(&self.home_retrospectives)
            .chain(&self.library)
            .find(|card| card.tag == tag && !card.cover.is_empty())
            .map(|card| (card.cover.clone(), card.cover_path.clone()));

        if let (Some((url, path)), Some(detail)) = (achada, self.detail.as_mut()) {
            detail.cover = url;
            detail.cover_path = path;
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
    /// Pede a foto da conta, uma vez por sessao.
    ///
    /// Sai no primeiro `if` em todo tique depois que a URL estabiliza -- e ela
    /// so muda no login e no logout --, entao nao pesa no laco de 100 ms.
    fn resolve_account_avatar(&mut self) {
        let url = self.session.state().avatar_url().to_string();
        if url == self.account_avatar.0 {
            return;
        }

        self.account_avatar = (url.clone(), None);
        if url.is_empty() {
            return;
        }

        let Some(browse) = self.session.browse_mut() else {
            return;
        };
        // `None` aqui nao e falha: o download comecou e o arquivo chega num
        // tique proximo, pelo mesmo canal das capas.
        self.account_avatar.1 = browse.cover(&url);
    }

    fn resolve_now_cover(&mut self) {
        // A capa forcada pela verificacao visual nao vem da fila, entao o
        // primeiro tique a apagaria. Fora da feature `snapshot` isto nem existe.
        #[cfg(feature = "snapshot")]
        if std::env::var_os("MORUNE_SNAPSHOT_COVER").is_some() {
            return;
        }

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
                self.refresh_tint();
            }

            if ready.url == self.account_avatar.0 {
                self.account_avatar.1 = Some(ready.path.clone());
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
        self.request_data_for(self.page);
    }

    /// Pede os dados de uma pagina especifica.
    ///
    /// Separado de [`AppState::request_page_data`] para que "Tentar novamente"
    /// possa refazer o pedido **da pagina que falhou**, e nao da que estiver
    /// aberta no instante do clique.
    fn request_data_for(&mut self, page: Page) {
        if !self.session.state().is_logged_in() {
            return;
        }

        let (home_requested, library_requested) = (self.home_requested, self.library_requested);
        let Some(browse) = self.session.browse_mut() else {
            return;
        };

        match page {
            Page::Home if !home_requested => {
                browse.load_home();
                self.home_requested = true;
                self.pending_retry = Some(RetryTarget::Page(Page::Home));
            }
            Page::Library if !library_requested => {
                browse.load_library();
                self.library_requested = true;
                self.pending_retry = Some(RetryTarget::Page(Page::Library));
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
    /// Abre "Músicas curtidas" por tras, sem tirar o usuario de onde ele esta:
    /// ele clicou numa musica, nao numa lista. A primeira pagina ja comeca a
    /// tocar e as seguintes entram na fila em lotes, pelo mesmo caminho que a
    /// tela de detalhe usa.
    fn play_liked_collection(&mut self, id: TrackId) {
        let Some(browse) = self.session.browse_mut() else {
            self.status = "Entre na sua conta do Spotify para tocar.".into();
            return;
        };
        browse.open(Target::Liked);
        self.pending_retry = Some(RetryTarget::Detail(Target::Liked.tag()));
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
                        "Preparando o rádio...".into()
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

    /// Opacidade e acrilico pedidos pelo tema, para o codigo de plataforma.
    ///
    /// A escolha do usuario, quando existe, substitui a do tema -- mesma regra
    /// do fundo e da escala tipografica.
    pub fn window_effects(&self) -> (f32, bool) {
        let escolhido = self.config.appearance.window_opacity_override;
        let opacidade = if escolhido > 0.0 {
            escolhido.clamp(0.2, 1.0)
        } else {
            self.spec().effects.window_opacity
        };
        (opacidade, self.spec().effects.acrylic)
    }

    /// Opacidade da janela no formato do slider: `0` opaca, `1` no limite.
    pub fn window_opacity_slider(&self) -> f32 {
        opacidade_para_slider(self.window_effects().0)
    }

    pub fn set_window_opacity(&mut self, value: f32) {
        self.config.appearance.window_opacity_override = slider_para_opacidade(value);
        self.save_config();
    }

    /// Recarregar o tema sozinho quando o arquivo mudar em disco.
    pub fn hot_reload(&self) -> bool {
        self.config.developer.hot_reload
    }

    pub fn set_hot_reload(&mut self, on: bool) {
        self.config.developer.hot_reload = on;
        self.save_config();
    }

    pub fn themes_dir(&self) -> std::path::PathBuf {
        self.paths.themes_dir()
    }

    pub fn theme_id(&self) -> &str {
        &self.theme.spec.manifest.id
    }

    /// Cor dos glifos da barra de tarefas do Windows.
    ///
    /// O acento, e nao o texto: aqueles botoes ficam sobre a miniatura que o
    /// **Windows** desenha, cujo fundo segue o tema do sistema e nao o do
    /// Morune. Uma cor de texto seguiria o contraste errado -- clara num
    /// Windows claro some. O acento e a unica cor do tema pensada para se
    /// destacar sozinha.
    ///
    /// No material Aero o glifo ganha a esfera de vidro do Windows 7 atras, na
    /// cor de selecao do tema, com o proprio acento (escuro) como glifo e borda.
    pub fn taskbar_tint(&self) -> crate::taskbar::IconStyle {
        let c = self.spec().colors.accent;
        let s = self.spec().colors.selected;
        let orbe = (self.spec().effects.material == morune_theme::tokens::MaterialKind::Aero)
            .then_some(crate::taskbar::Orbe {
                base: [s.r, s.g, s.b],
                borda: [c.r, c.g, c.b],
            });
        crate::taskbar::IconStyle {
            tint: [c.r, c.g, c.b],
            orbe,
        }
    }

    fn spec(&self) -> &ThemeSpec {
        &self.theme.spec
    }

    // ---- tema ----

    pub fn apply_theme_to(&self, window: &ui::AppWindow) {
        // Antes de tudo: a familia so resolve depois de a fonte existir.
        theme_bridge::apply_bundled_font(self.spec(), self.theme.source.as_deref());
        let theme = window.global::<ui::Theme>();
        theme_bridge::apply(
            &theme,
            &window.global::<ui::Layout>(),
            self.spec(),
            self.overrides,
        );
        theme_bridge::apply_background(&theme, self.spec(), &self.wallpaper);
        theme_bridge::apply_icons(&window.global::<ui::Icons>(), self.theme.source.as_deref());
    }

    /// So a composicao da tela (barra lateral, player, grade), sem tocar em
    /// cor, fonte nem imagem. Para o que muda geometria e nada mais.
    pub fn apply_layout_to(&self, window: &ui::AppWindow) {
        theme_bridge::apply_layout(&window.global::<ui::Layout>(), self.spec(), self.overrides);
    }

    /// Reprepara a imagem de fundo a partir do tema e da escolha do usuario.
    ///
    /// Chamada so quando um dos dois muda. A escolha do usuario vence a do
    /// tema inteira -- imagem e ajustes juntos -- pelo mesmo motivo que a
    /// escala tipografica substitui em vez de multiplicar: dois lados
    /// misturados dao um resultado que ninguem pediu.
    pub fn refresh_wallpaper(&mut self) {
        let user = &self.config.appearance;
        let tokens = if user.background_image.is_empty() {
            self.theme.spec.background.clone()
        } else {
            morune_theme::BackgroundTokens {
                image: String::new(),
                fit: user.background_fit,
                opacity: user.background_opacity,
                tint: self.theme.spec.background.tint,
                tint_strength: user.background_tint_strength,
                blur: user.background_blur,
            }
        };
        let user_image = (!user.background_image.is_empty())
            .then(|| std::path::PathBuf::from(&user.background_image));
        self.wallpaper = crate::wallpaper::load(
            self.theme.source.as_deref(),
            &tokens,
            user_image.as_deref(),
            user.glass_blur,
        );
        self.wallpaper.tintas = Some(theme_bridge::tintas_legiveis(self.spec(), &self.wallpaper));
        // Em `debug` e nao `info`: e diagnostico de tema, so interessa a quem
        // esta descobrindo por que o fundo dele nao apareceu.
        tracing::debug!(
            tema = ?self.theme.source,
            arquivo = %tokens.image,
            largura = self.wallpaper.image.size().width,
            altura = self.wallpaper.image.size().height,
            "fundo preparado"
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
            self.status = format!("Não foi possível aplicar o tema {id}; nada mudou.");
            return;
        }
        self.status = format!("Tema aplicado: {}", loaded.spec.manifest.name);
        self.theme = loaded;
        self.config.appearance.theme = id.to_string();
        self.refresh_wallpaper();
        self.save_config();
    }

    pub fn reload_theme(&mut self) {
        let id = self.config.appearance.theme.clone();
        // Recarregar existe para pegar o que mudou no disco, entao a lista de
        // temas instalados tambem tem que ser relida aqui.
        self.refresh_themes();
        self.theme = loader::load(&self.paths.themes_dir(), &id);
        self.refresh_wallpaper();
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
        self.refresh_wallpaper();
        self.status = "Tema restaurado para o padrão.".into();
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
            Err(e) => self.status = format!("Não foi possível duplicar o tema: {e}"),
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
                "Autor não informado"
            } else {
                preview.manifest.author.as_str()
            },
            if preview.manifest.version.is_empty() {
                "Não informada"
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
            self.status = "Importação cancelada; nada foi alterado.".into();
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
                    self.status = format!("Não foi possível preparar a substituição: {e}");
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
            self.status = "O tema embutido não pode ser exportado; duplique-o antes.".into();
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
            Err(e) => self.status = format!("Não foi possível abrir a pasta: {e}"),
        }
    }

    /// Endereco de autorizacao do login em andamento. Vazio fora dele.
    pub fn auth_url(&self) -> &str {
        self.session.auth_url()
    }

    /// Desiste do login em andamento e libera a porta.
    pub fn cancel_login(&mut self) {
        self.session.cancel_login();
        self.status = "Login cancelado.".into();
    }

    /// Copia o endereco de autorizacao para a area de transferencia.
    ///
    /// E a saida de quem tem mais de um navegador: o Morune abre o padrao do
    /// sistema, que pode ser o perfil errado, e colar o link no navegador certo
    /// e a unica forma de concluir sem reiniciar nada.
    pub fn copy_auth_url(&mut self) {
        let url = self.session.auth_url().to_string();
        if url.is_empty() {
            return;
        }

        match crate::clipboard::copy(&url) {
            Ok(()) => self.status = "Link copiado. Cole no navegador da sua conta.".into(),
            Err(e) => {
                tracing::warn!(erro = %e, "nao foi possivel copiar o link");
                self.status = "Não foi possível copiar. Selecione o link e copie à mão.".into();
            }
        }
    }

    // ---- atualizacao ----

    /// Pergunta ao GitHub se ha versao nova. So a partir de um clique.
    pub fn check_for_update(&mut self) {
        self.updater.check();
    }

    /// Baixa o instalador da versao encontrada.
    pub fn download_update(&mut self) {
        self.updater.download();
    }

    /// Abre a pagina de lancamentos, para quem prefere baixar a mao.
    pub fn open_releases_page(&mut self) {
        let result = open_link(crate::update::RELEASES_URL);
        if let Err(e) = result {
            self.status = format!("Não foi possível abrir o navegador: {e}");
        }
    }

    /// Executa o instalador baixado e pede o encerramento do aplicativo.
    ///
    /// Devolve `true` quando o instalador subiu -- e so entao quem chamou
    /// fecha o laco de eventos. Falhar aqui deixa tudo como estava: a musica
    /// continua tocando e a mensagem explica o que houve.
    ///
    /// `/S` e o modo silencioso do NSIS e `/RESTART` e a nossa opcao, que faz o
    /// instalador esperar este processo sair e reabrir o Morune no fim. Sem ela
    /// o modo silencioso instalaria e nao devolveria o aplicativo a pessoa, que
    /// so veria a janela desaparecer.
    #[must_use]
    pub fn install_update(&mut self) -> bool {
        let Some(installer) = self.updater.installer() else {
            return false;
        };
        let installer = installer.to_path_buf();

        match std::process::Command::new(&installer)
            .args(["/S", "/RESTART"])
            .spawn()
        {
            Ok(_) => {
                tracing::info!(arquivo = %installer.display(), "instalador iniciado");
                // A reproducao para antes de o processo morrer: deixar a
                // librespot ser derrubada no meio de um buffer produz um
                // estalo na saida de audio.
                self.stop();
                self.save_config();
                true
            }
            Err(e) => {
                tracing::error!(erro = %e, "instalador nao pode ser executado");
                self.status = format!("Não foi possível abrir o instalador: {e}");
                false
            }
        }
    }

    /// O que a tela de configuracoes mostra sobre atualizacao.
    ///
    /// Devolve `(codigo, titulo, detalhe, percentual)`. O codigo e o que a
    /// interface usa para escolher o botao; manter a decisao aqui evita
    /// espalhar a maquina de estados pelo `.slint`, onde ela nao pode ser
    /// testada.
    ///
    /// Os codigos acompanham [`crate::update::Phase`]: 0 parado, 1 verificando,
    /// 2 em dia, 3 disponivel, 4 baixando, 5 pronto, 6 falhou.
    pub fn update_status(&self) -> (i32, String, String, f32) {
        use crate::update::Phase;

        let atual = format!("Versão {}", self.updater.current());
        let nova = || {
            self.updater
                .release()
                .map(|r| r.version.to_string())
                .unwrap_or_default()
        };
        let notas = || {
            self.updater
                .release()
                .map(|r| r.notes.clone())
                .unwrap_or_default()
        };

        // Vale mais que qualquer fase: enquanto o executavel em disco nao for o
        // que este processo carregou, "em dia" e "disponível" falam de um
        // binario que nao e o que esta na frente da pessoa.
        if self.binary.changed() {
            return (
                6,
                atual,
                "O executável do Morune mudou em disco depois que este processo abriu. \
                 Feche e abra o aplicativo para usar a versão nova."
                    .into(),
                0.0,
            );
        }

        // Build local nunca tem lancamento para receber -- ele sai de commits
        // que os publicados ainda nao tem. Dizer "você já está na versão mais
        // recente" aqui seria verdade sem informacao: o que a pessoa precisa
        // saber e que este binario se atualiza recompilando, e nao pelo botao.
        if self.updater.is_dev() && matches!(self.updater.phase(), Phase::Idle | Phase::UpToDate) {
            return (
                2,
                atual,
                "Build local, compilado fora do fluxo de publicação. \
                 A atualização automática não vale para ele: recompile do repositório."
                    .into(),
                0.0,
            );
        }

        match self.updater.phase() {
            Phase::Idle => (0, atual, String::new(), 0.0),
            Phase::Checking => (1, atual, "Procurando...".into(), 0.0),
            Phase::UpToDate => (2, atual, "Você já está na versão mais recente.".into(), 0.0),
            Phase::Available => (3, format!("Versão {} disponível", nova()), notas(), 0.0),
            Phase::Downloading(pct) => (
                4,
                format!("Baixando a versão {}", nova()),
                format!("{pct}%"),
                *pct as f32 / 100.0,
            ),
            Phase::Ready => (
                5,
                format!("Versão {} pronta para instalar", nova()),
                "O Morune fecha, instala e abre de novo. A música para durante a instalação."
                    .into(),
                1.0,
            ),
            Phase::Failed(mensagem) => (6, atual, mensagem.clone(), 0.0),
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

    // ---- aparencia ----

    /// Escolhe a imagem de fundo e copia para dentro da pasta do aplicativo.
    ///
    /// Copiar, e nao guardar o caminho original: uma imagem em Downloads que a
    /// pessoa apaga depois deixaria a janela sem fundo sem nenhum aviso, e o
    /// custo de uma copia unica e menor que o de explicar isso.
    pub fn choose_background_via_dialog(&mut self) {
        let Some(file) = rfd::FileDialog::new()
            .add_filter("Imagem", &["png", "jpg", "jpeg", "webp", "bmp"])
            .set_title("Escolher imagem de fundo")
            .pick_file()
        else {
            return;
        };

        let dir = self.paths.backgrounds_dir();
        if let Err(error) = std::fs::create_dir_all(&dir) {
            self.status = "Não foi possível guardar a imagem de fundo.".into();
            tracing::warn!(%error, dir = %dir.display(), "pasta de fundos nao criada");
            return;
        }

        let name = file
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "fundo".into());
        let target = dir.join(&name);
        // O mesmo arquivo escolhido de novo nao precisa ser recopiado, e copiar
        // um arquivo sobre ele mesmo falharia.
        if target != file {
            if let Err(error) = std::fs::copy(&file, &target) {
                self.status = "Não foi possível copiar a imagem de fundo.".into();
                tracing::warn!(%error, de = %file.display(), "copia do fundo falhou");
                return;
            }
        }

        self.config.appearance.background_image = target.to_string_lossy().to_string();
        self.status = format!("Fundo aplicado: {name}");
        self.refresh_wallpaper();
        self.save_config();
    }

    pub fn clear_background(&mut self) {
        // O arquivo copiado fica: remover uma imagem do fundo nao e o mesmo que
        // apagar o que a pessoa escolheu, e ela pode querer de volta.
        self.config.appearance.background_image.clear();
        self.status = "Fundo removido.".into();
        self.refresh_wallpaper();
        self.save_config();
    }

    /// Nome do arquivo de fundo, para a tela de configuracoes.
    pub fn background_name(&self) -> String {
        std::path::Path::new(&self.config.appearance.background_image)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default()
    }

    pub fn set_background_fit(&mut self, code: i32) {
        self.config.appearance.background_fit = match code {
            1 => morune_theme::BackgroundFit::Contain,
            2 => morune_theme::BackgroundFit::Center,
            3 => morune_theme::BackgroundFit::Stretch,
            _ => morune_theme::BackgroundFit::Cover,
        };
        self.refresh_wallpaper();
        self.save_config();
    }

    pub fn background_fit_code(&self) -> i32 {
        match self.config.appearance.background_fit {
            morune_theme::BackgroundFit::Cover => 0,
            morune_theme::BackgroundFit::Contain => 1,
            morune_theme::BackgroundFit::Center => 2,
            morune_theme::BackgroundFit::Stretch => 3,
        }
    }

    pub fn set_background_opacity(&mut self, value: f32) {
        self.config.appearance.background_opacity = value.clamp(0.0, 1.0);
        self.refresh_wallpaper();
        self.save_config();
    }

    pub fn set_background_tint(&mut self, value: f32) {
        self.config.appearance.background_tint_strength = value.clamp(0.0, 1.0);
        self.refresh_wallpaper();
        self.save_config();
    }

    /// Desfoque, recebido normalizado em `[0, 1]` e guardado em pixels.
    ///
    /// O slider da interface nao sabe o teto; guardar em pixels e o que faz o
    /// valor continuar significando a mesma coisa se o teto mudar.
    pub fn set_background_blur(&mut self, value: f32) {
        self.config.appearance.background_blur = value.clamp(0.0, 1.0) * 64.0;
        self.refresh_wallpaper();
        self.save_config();
    }

    /// Desfoque do vidro, recebido normalizado em `[0, 1]` e guardado em pixels.
    ///
    /// So o borrao e refeito, a partir da copia que ja esta na memoria:
    /// decodificar o JPEG de novo a cada toque no slider travava a tela.
    pub fn set_glass_blur(&mut self, value: f32) {
        let raio = value.clamp(0.0, 1.0) * crate::wallpaper::RAIO_VIDRO_MAX;
        self.config.appearance.glass_blur = raio;
        self.wallpaper.com_desfoque_do_vidro(raio);
        self.wallpaper.tintas = Some(theme_bridge::tintas_legiveis(self.spec(), &self.wallpaper));
        self.save_config();
    }

    /// Escala tipografica, recebida normalizada em `[0, 1]` sobre 0,8..1,6.
    pub fn set_font_scale(&mut self, value: f32) {
        let scale = 0.8 + value.clamp(0.0, 1.0) * 0.8;
        self.config.appearance.font_scale_override = scale;
        self.overrides.font_scale = scale;
        self.save_config();
    }

    /// A escala corrente de volta para `[0, 1]`, para o slider.
    pub fn font_scale_slider(&self) -> f32 {
        let scale = if self.config.appearance.font_scale_override > 0.0 {
            self.config.appearance.font_scale_override
        } else {
            self.spec().typography.scale
        };
        ((scale - 0.8) / 0.8).clamp(0.0, 1.0)
    }

    pub fn set_reduce_motion(&mut self, on: bool) {
        self.config.appearance.reduce_motion = on;
        self.recalcular_movimento();
        self.save_config();
    }

    /// Ha um aplicativo em tela cheia na frente agora?
    pub fn tela_cheia_ativa(&self) -> bool {
        self.tela_cheia
    }

    /// Avisa que ha (ou deixou de haver) um aplicativo em tela cheia na frente.
    ///
    /// Devolve `true` quando o estado mudou -- so ai vale a pena reaplicar o
    /// tema, que e a operacao cara. O vigia le a cada segundo e na quase
    /// totalidade das leituras nada mudou.
    pub fn set_tela_cheia(&mut self, cheia: bool) -> bool {
        if self.tela_cheia == cheia {
            return false;
        }
        self.tela_cheia = cheia;
        self.recalcular_movimento();
        true
    }

    /// As tres razoes para nao haver movimento, resolvidas num valor so.
    ///
    /// A regra e a mesma para as tres: elas so **tiram** movimento, nunca
    /// devolvem. Quem desligou animacao no Windows pediu isso a todo
    /// aplicativo; quem esta com um jogo em tela cheia na frente nao quer o
    /// player gastando placa de video atras; e quem desligou aqui decidiu por
    /// conta propria. Basta uma para zerar.
    fn recalcular_movimento(&mut self) {
        self.overrides.reduce_motion =
            self.config.appearance.reduce_motion || !system_animation_enabled() || self.tela_cheia;
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
                    "O Morune não vai mais iniciar com o Windows.".into()
                };
            }
            Err(error) => {
                self.start_with_windows = crate::startup::is_enabled();
                self.status = format!("Não foi possível alterar a inicializacao: {error}");
            }
        }
    }

    /// Os tres degraus que o Spotify oferece, em kbps.
    ///
    /// A interface manda o indice, e nao o numero: sao opcoes de uma lista
    /// fechada, e deixar a tela escolher o valor faria dois lugares terem de
    /// concordar sobre quais numeros existem.
    const BITRATES: [u32; 3] = [96, 160, 320];

    /// Indice da qualidade atual. Um valor fora da lista cai no mais proximo
    /// para baixo, igual ao que o backend faz.
    pub fn bitrate_code(&self) -> i32 {
        Self::BITRATES
            .iter()
            .rposition(|&kbps| kbps <= self.config.playback.bitrate)
            .unwrap_or(0) as i32
    }

    pub fn set_bitrate(&mut self, code: i32) {
        let Some(&kbps) = Self::BITRATES.get(code.max(0) as usize) else {
            return;
        };
        self.config.playback.bitrate = kbps;
        self.apply_audio_settings();
        self.status = "A qualidade vale a partir da próxima vez que o Morune abrir.".into();
    }

    /// Dispositivos oferecidos na tela, sem a opcao "padrao do sistema" --
    /// essa a interface acrescenta como primeira linha.
    pub fn output_devices(&self) -> &[String] {
        &self.output_devices
    }

    /// Nome do dispositivo escolhido. Vazio = o padrao do sistema.
    pub fn output_device(&self) -> &str {
        &self.config.playback.output_device
    }

    /// Escolhe onde o som sai. Nome vazio devolve a escolha ao Windows.
    ///
    /// Vale so para o proximo motor, como qualidade e nivelamento: o
    /// dispositivo e aberto quando o `Player` nasce e trocar agora pararia a
    /// musica. A mensagem diz isso em vez de fingir efeito imediato.
    pub fn set_output_device(&mut self, nome: &str) {
        if self.config.playback.output_device == nome {
            return;
        }
        self.config.playback.output_device = nome.to_string();
        self.apply_audio_settings();
        self.status = if nome.is_empty() {
            "A saída volta a seguir o Windows na próxima vez que o Morune abrir.".into()
        } else {
            format!("A saída vale a partir da próxima vez que o Morune abrir: {nome}.")
        };
    }

    pub fn normalize(&self) -> bool {
        self.config.playback.normalize
    }

    pub fn set_normalize(&mut self, on: bool) {
        self.config.playback.normalize = on;
        self.apply_audio_settings();
        self.status = "O nivelamento vale a partir da próxima vez que o Morune abrir.".into();
    }

    /// Guarda as preferencias e as entrega ao backend.
    ///
    /// O motor que esta tocando nao muda -- ver `SpotifyBackend::set_audio`. Por
    /// isso quem chama escreve uma mensagem dizendo quando o ajuste vale: um
    /// controle que parece nao fazer nada e pior que um controle ausente.
    fn apply_audio_settings(&mut self) {
        self.session
            .set_audio(audio_settings(&self.config.playback));
        self.save_config();
    }

    pub fn set_autoplay(&mut self, on: bool) {
        self.config.playback.autoplay = on;
        self.status = if on {
            "Rádio ligado: quando a fila acabar, a música continua.".into()
        } else {
            "Rádio desligado: a música para no fim da fila.".into()
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

    /// O que o painel de midia do Windows deve mostrar. `None` sem faixa.
    ///
    /// Separado de [`AppState::tray_status`], que devolve uma linha unica: o
    /// painel do sistema tem campos proprios para titulo, artista e album, e
    /// juntar tudo numa string faria o Windows exibir o texto do Morune em vez
    /// da faixa.
    #[cfg(windows)]
    pub fn media_status(&self) -> Option<crate::smtc::MediaStatus> {
        let track = self.queue.current()?;
        Some(crate::smtc::MediaStatus {
            title: track.name.to_string(),
            artist: track.artists_line(),
            album: track
                .album
                .as_ref()
                .map(|a| a.name.to_string())
                .unwrap_or_default(),
            // A mesma capa que o menu da bandeja usa, ja no cache em disco.
            cover: self.now_cover.1.clone(),
            playing: self.engine.snapshot().state == morune_core::PlaybackState::Playing,
        })
    }

    /// Manda tocar, sem alternar.
    ///
    /// O painel do Windows tem botoes separados para tocar e pausar, e o
    /// usuario pode clicar em "tocar" no que ja esta tocando. Alternar ali
    /// pausaria o que ele acabou de mandar tocar.
    pub fn play(&mut self) {
        if self.queue.current().is_none() || self.playing {
            return;
        }
        self.playing = true;
        self.send(PlayerCommand::Play);
    }

    /// Manda pausar, sem alternar. Ver [`AppState::play`].
    pub fn pause(&mut self) {
        if self.queue.current().is_none() || !self.playing {
            return;
        }
        self.playing = false;
        self.send(PlayerCommand::Pause);
    }

    /// Preenche o menu da bandeja com a faixa atual.
    ///
    /// O menu e a unica superficie de reproducao visivel com a janela fechada,
    /// entao ele mostra capa, titulo e artista separados -- e nao a linha unica
    /// que cabia num item de menu do sistema.
    pub fn push_to_tray_menu(&self, menu: &ui::TrayMenuWindow) {
        theme_bridge::apply(
            &menu.global::<ui::Theme>(),
            &menu.global::<ui::Layout>(),
            self.spec(),
            self.overrides,
        );

        match self.queue.current() {
            Some(track) => {
                menu.set_has_track(true);
                menu.set_now_title(track.name.as_ref().into());
                menu.set_now_artist(track.artists_line().into());
                menu.set_now_cover(cover_image(self.now_cover.1.as_deref()));
                let (initial, hue) = cover_badge(
                    track
                        .album
                        .as_ref()
                        .map(|a| a.name.as_ref())
                        .unwrap_or(track.name.as_ref()),
                );
                menu.set_now_cover_initial(initial);
                menu.set_now_cover_hue(hue);
            }
            None => {
                menu.set_has_track(false);
                menu.set_now_title(Default::default());
                menu.set_now_artist(Default::default());
            }
        }

        menu.set_playing(self.engine.snapshot().state == morune_core::PlaybackState::Playing);
        menu.set_volume(self.volume);
    }

    pub fn toggle_sidebar(&mut self) {
        self.overrides.sidebar_collapsed = !self.overrides.sidebar_collapsed;
    }

    // ---- navegacao ----

    pub fn navigate(&mut self, page: i32) {
        let destino = Page::from_i32(page);
        if destino == Page::Queue && self.page != Page::Queue {
            // O Detalhe e fechado ao sair dele (abaixo), entao voltar para la
            // daria uma pagina vazia; a Home e o retorno seguro nesse caso.
            self.page_before_queue = if self.page == Page::Detail {
                Page::Home
            } else {
                self.page
            };
        }
        self.page = destino;
        if self.page == Page::Settings {
            // Um fone conectado depois de abrir o aplicativo so aparece aqui.
            self.output_devices = morune_spotify::output_devices();
        }
        if self.page != Page::Detail {
            self.detail_loading = false;
            self.detail_complete_requested = false;
            self.detail_pending_play = None;
            self.detail_history.clear();
        }
        self.request_page_data();
    }

    /// Fecha a Fila e volta para a pagina de onde ela foi aberta.
    pub fn close_queue(&mut self) {
        if self.page == Page::Queue {
            self.navigate(self.page_before_queue as i32);
        }
    }

    /// Silencia, ou devolve o volume de antes de silenciar.
    pub fn toggle_mute(&mut self) {
        if self.volume > 0.0 {
            self.volume_before_mute = self.volume;
            self.set_volume(0.0);
        } else {
            // Silenciado desde o inicio (config salva com zero): metade e um
            // volume que da para ouvir sem assustar.
            let volta = if self.volume_before_mute > 0.0 {
                self.volume_before_mute
            } else {
                0.5
            };
            self.set_volume(volta);
        }
    }

    /// Passo de volume (roda do mouse, Ctrl+setas).
    pub fn nudge_volume(&mut self, delta: f32) {
        self.set_volume(self.volume + delta);
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
            self.status = "Entre no Spotify para buscar.".to_string();
            return;
        }

        let Some(browse) = self.session.browse_mut() else {
            self.status = "Não foi possível iniciar o Spotify nesta máquina. Feche e abra o Morune para tentar de novo.".into();
            return;
        };
        self.searching = true;
        self.search = TrackList::default();
        self.search_cards.clear();
        browse.search(query);
        self.pending_retry = Some(RetryTarget::Search(query.to_string()));
        self.status = format!("Buscando \"{query}\"...");
    }

    pub fn clear_status(&mut self) {
        self.status.clear();
    }

    pub fn undo_available(&self) -> bool {
        self.undo.is_some()
    }

    /// `true` quando a mensagem na tela e a que ofereceu repetir.
    ///
    /// A comparacao com `status` e o que impede o botao de sobreviver a propria
    /// mensagem. Ver [`Retry`].
    pub fn retry_available(&self) -> bool {
        retry_belongs_to(self.retry.as_ref(), &self.status)
    }

    pub fn retry_last(&mut self) {
        let Some(retry) = self.retry.take() else {
            return;
        };

        match retry.target {
            RetryTarget::Search(query) => self.search(&query),
            // A pagina vem guardada, e nao lida de `self.page`: repetir tem de
            // refazer o que falhou, mesmo que o usuario ja tenha navegado.
            RetryTarget::Page(page) => {
                match page {
                    Page::Home => self.home_requested = false,
                    Page::Library => self.library_requested = false,
                    _ => {}
                }
                self.request_data_for(page);
            }
            RetryTarget::Detail(tag) => self.open_detail_target(&tag),
        }
    }

    /// Reabre uma lista pela forma textual do alvo.
    fn open_detail_target(&mut self, tag: &str) {
        let Some(target) = Target::parse(tag) else {
            return;
        };
        let Some(browse) = self.session.browse_mut() else {
            self.status = "Não foi possível iniciar o Spotify nesta máquina. Feche e abra o Morune para tentar de novo.".into();
            return;
        };
        browse.open(target);
        self.pending_retry = Some(RetryTarget::Detail(tag.to_string()));
        self.status = "Carregando...".into();
    }

    pub fn dismiss_recovery(&mut self) {
        self.discard_undo();
        self.retry = None;
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
            self.status = "Não ha nenhuma acao recente para desfazer.".into();
            return;
        };

        match action {
            UndoAction::QueueClear(tracks) => {
                let count = tracks.len();
                for track in tracks {
                    self.queue.enqueue(track);
                }
                self.status = format!("Fila restaurada: {} de volta.", faixas(count));
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
                        self.status = format!("Não foi possível restaurar o tema anterior: {e}");
                        return;
                    }
                }
                self.refresh_themes();
                self.select_theme(&previous_id);
                self.status = "Importação desfeita; o tema anterior foi restaurado.".into();
            }
        }
    }

    /// Adiciona ou remove uma faixa das "Músicas curtidas" do Spotify.
    pub fn toggle_favorite(&mut self, tag: &str) {
        let Some(Target::Track(id)) = Target::parse(tag) else {
            self.status = "Não reconheci a faixa que você quer curtir.".into();
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
            // Beco sem saida virou saida: a frase antiga dizia o estado e parava
            // ali, sem dizer o que fazer com ele.
            self.status =
                "Curtida não salva: sem conexão com o Spotify. Feche e abra o Morune.".into();
            return;
        };
        browse.set_track_saved(id, saved);
        self.status = if saved {
            "Adicionando às Músicas curtidas do Spotify...".into()
        } else {
            "Removendo das Músicas curtidas do Spotify...".into()
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
            self.status = "Essa faixa já não está mais na fila.".into();
            return;
        };
        self.status = format!("{} foi removida da fila.", track.name);
    }

    pub fn queue_play_manual(&mut self, index: i32) {
        let Some(track) = usize::try_from(index)
            .ok()
            .and_then(|index| self.queue.remove_from_user_queue(index))
        else {
            self.status = "Essa faixa já não está mais na fila.".into();
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
            self.status = "Não foi possível mover essa faixa na fila.".into();
        }
    }

    pub fn queue_clear(&mut self) {
        let removed: Vec<_> = self.queue.user_queue().cloned().collect();
        let count = removed.len();
        self.queue.clear_user_queue();
        self.status = if count == 0 {
            "A fila já estava vazia.".into()
        } else {
            self.discard_undo();
            self.undo = Some(UndoAction::QueueClear(removed));
            format!(
                "Fila limpa: {} removida{}.",
                faixas(count),
                if count == 1 { "" } else { "s" }
            )
        };
    }

    fn track_from_tag(&mut self, tag: &str) -> Option<Track> {
        let Some(Target::Track(id)) = Target::parse(tag) else {
            self.status = "Não reconheci a faixa escolhida.".into();
            return None;
        };
        let track = self.find_track(&id);
        if track.is_none() {
            self.status = "Essa faixa saiu da lista. Atualize a tela para vê-la de novo.".into();
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
            self.status = "Não reconheci o que você clicou.".into();
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
        self.pending_retry = Some(RetryTarget::Detail(tag.to_string()));
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

    /// Para a reproducao e volta a faixa ao inicio.
    ///
    /// Diferente de pausar: pausar guarda o lugar, parar desfaz o progresso.
    /// A faixa continua carregada na fila -- quem parou quer silencio, nao
    /// perder o que estava ouvindo.
    pub fn stop(&mut self) {
        if self.queue.current().is_none() {
            return;
        }
        self.playing = false;
        self.seek_target = Some(Duration::ZERO);
        self.send(PlayerCommand::Stop);
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

    /// O motor novo nao herda faixa nem preferencias do que perdeu a sessao.
    /// A fila e a intencao de tocar pertencem ao `AppState`, portanto sao elas
    /// que reconstroem o retrato do motor depois do login ou da reconexao.
    fn restore_recreated_engine(&mut self) {
        for command in recreated_engine_commands(
            self.queue.current().cloned(),
            self.playing,
            self.volume,
            self.config.playback.shuffle,
            self.config.playback.repeat,
        ) {
            self.send(command);
        }
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
        self.home_loaded = false;
        self.library_requested = false;
        self.library_loaded = false;

        // As ofertas de recuperacao morrem com a sessao.
        //
        // "Tentar novamente" so faz sentido enquanto ha conta: sem sessao, o
        // pedido refeito falha do mesmo jeito, e o botao vira um convite a
        // repetir um erro. O mesmo vale para o pedido em voo, que se resolvia
        // contra uma conta que nao esta mais aqui.
        //
        // Sem isto o botao ja nao apareceria -- `retry_available` exige que a
        // mensagem na tela seja a que criou a oferta, e aqui ela vira "Sessao
        // encerrada." --, mas o estado morto ficaria guardado. Limpar e o que
        // impede uma mensagem futura de ressuscitar a oferta por coincidencia.
        self.retry = None;
        self.pending_retry = None;
        self.discard_undo();

        self.status = "Sessão encerrada.".into();
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
        window.set_retry_available(self.retry_available());
        window.set_logged_in(self.session.state().is_logged_in());
        let account = self.session.state().account_name();
        window.set_account_name(SharedString::from(account));
        window.set_account_initial(SharedString::from(account_initial(account)));
        // Vazia enquanto a foto nao chega, e vazia para sempre em quem nao tem
        // foto. Nos dois casos o avatar cai na inicial, sem mudar o layout.
        window.set_account_avatar(cover_image(self.account_avatar.1.as_deref()));
        window.set_dev_mode(self.config.developer.enabled);
        window.set_close_to_tray(self.config.window.close_to_tray);
        window.set_start_with_windows(self.start_with_windows);
        window.set_autoplay(self.config.playback.autoplay);
        window.set_bitrate(self.bitrate_code());
        window.set_normalize(self.normalize());
        // **So as listas da pagina visivel sao refeitas.** Montar as linhas de
        // todas as paginas -- curtidas inteiras, fila, busca, biblioteca -- a
        // cada capa que chega era trabalho na thread da interface para telas
        // que ninguem estava vendo, e aparecia como engasgo na rolagem de quem
        // estava na Home. Quem entra numa pagina passa por `navigate`, que
        // chama isto de novo com a pagina ja trocada: nada chega atrasado.
        let pagina = self.page;
        window.set_output_devices(self.listas.output_devices.sincronizar_se(
            pagina == Page::Settings,
            || {
                self.output_devices()
                    .iter()
                    .map(|nome| SharedString::from(nome.as_str()))
                    .collect()
            },
        ));
        window.set_output_device(SharedString::from(self.output_device()));
        let appearance = &self.config.appearance;
        window.set_has_background(!appearance.background_image.is_empty());
        window.set_background_name(SharedString::from(self.background_name()));
        window.set_background_fit(self.background_fit_code());
        window.set_background_opacity(appearance.background_opacity);
        window.set_background_tint(appearance.background_tint_strength);
        window.set_background_blur(appearance.background_blur / 64.0);
        window.set_glass_blur(appearance.glass_blur / crate::wallpaper::RAIO_VIDRO_MAX);
        window.set_font_scale(self.font_scale_slider());
        window.set_reduce_motion(appearance.reduce_motion);
        window.set_hot_reload(self.config.developer.hot_reload);
        window.set_window_opacity(self.window_opacity_slider());
        window.set_search_query(self.search_query.as_str().into());
        window.set_searching(self.searching);
        window.set_home_loaded(self.home_loaded);
        window.set_library_loaded(self.library_loaded);

        window.set_auth_url(SharedString::from(self.auth_url()));

        let (fase, titulo, detalhe, progresso) = self.update_status();
        window.set_update_phase(fase);
        window.set_update_title(SharedString::from(titulo));
        window.set_update_detail(SharedString::from(detalhe));
        window.set_update_progress(progresso);

        self.push_playback(window);

        let current = self.queue.current();
        window.set_sidebar_playlists(self.listas.sidebar_playlists.sincronizar(sidebar_items(
            &self.sidebar_playlists(),
            &self.config.navigation.pinned_playlists,
        )));

        if let Some(detail) = &self.detail {
            window.set_detail_title(detail.title.as_str().into());
            window.set_detail_subtitle(detail.subtitle.as_str().into());
            window.set_detail_kind(detail.kind.as_str().into());
            window.set_detail_cover(cover_image(detail.cover_path.as_deref()));
            let (initial, hue) = cover_badge(&detail.title);
            window.set_detail_cover_initial(initial);
            window.set_detail_cover_hue(hue);
            window
                .set_detail_cover_pending(!detail.cover.is_empty() && detail.cover_path.is_none());
            window.set_detail_tracks(self.listas.detail_tracks.sincronizar_se(
                pagina == Page::Detail,
                || {
                    track_rows(
                        self.detail_tracks(),
                        current,
                        &self.track_covers,
                        &self.liked_ids,
                    )
                },
            ));
            window.set_detail_sort(self.detail_sort as i32);
            window.set_detail_descending(self.detail_desc);
            window.set_detail_filtered(!self.detail_filter.is_empty());
            window.set_detail_has_more(detail.has_more);
            window.set_detail_loading(self.detail_loading);
            window.set_detail_loaded_count(detail.tracks.len() as i32);
            window.set_detail_items(
                self.listas
                    .detail_items
                    .sincronizar_se(pagina == Page::Detail, || card_items(&detail.cards)),
            );
        }
        let l = &self.listas;
        let na_fila = pagina == Page::Queue;
        window.set_queue_manual_tracks(l.queue_manual_tracks.sincronizar_se(na_fila, || {
            track_rows(
                self.queue.user_queue().take(200).collect(),
                current,
                &self.track_covers,
                &self.liked_ids,
            )
        }));
        window.set_queue_context_tracks(l.queue_context_tracks.sincronizar_se(na_fila, || {
            track_rows(
                self.queue.upcoming_context(200),
                current,
                &self.track_covers,
                &self.liked_ids,
            )
        }));
        let nos_ajustes = pagina == Page::Settings;
        window.set_themes(l.themes.sincronizar_se(nos_ajustes, || self.theme_items()));
        window.set_diagnostics(
            l.diagnostics
                .sincronizar_se(nos_ajustes, || self.diagnostics()),
        );
        let na_home = pagina == Page::Home;
        window.set_home_made_for_you(
            l.home_made_for_you
                .sincronizar_se(na_home, || card_items(&self.home_made_for_you)),
        );
        window.set_home_liked(l.home_liked.sincronizar_se(na_home, || {
            track_rows(
                self.liked.tracks.iter().collect(),
                current,
                &self.track_covers,
                &self.liked_ids,
            )
        }));
        window.set_home_playlists(
            l.home_playlists
                .sincronizar_se(na_home, || card_items(&self.home_playlists)),
        );
        window.set_home_stations(
            l.home_stations
                .sincronizar_se(na_home, || card_items(&self.home_stations)),
        );
        window.set_home_retrospectives(
            l.home_retrospectives
                .sincronizar_se(na_home, || card_items(&self.home_retrospectives)),
        );
        window.set_library_items(
            l.library_items
                .sincronizar_se(pagina == Page::Library, || card_items(&self.library)),
        );
        let na_busca = pagina == Page::Search;
        window.set_search_items(
            l.search_items
                .sincronizar_se(na_busca, || card_items(&self.search_cards)),
        );
        window.set_search_tracks(l.search_tracks.sincronizar_se(na_busca, || {
            track_rows(
                self.search.tracks.iter().collect(),
                current,
                &self.track_covers,
                &self.liked_ids,
            )
        }));
    }

    /// Espelha so a barra de reproducao.
    ///
    /// Separado de [`AppState::push_to_ui`] porque este e o caminho que o
    /// usuario sente: volume, seek e play/pause chamam so isto. Sao propriedades
    /// escalares -- nenhum `VecModel` reconstruido, nenhuma linha de faixa, nada
    /// de disco -- entao pode rodar a cada quadro de arraste e a cada tique do
    /// relogio de progresso sem aparecer no medidor de CPU.
    /// Recalcula a cor da capa, se a capa mudou.
    ///
    /// Chamada de onde a capa muda -- e nao de `push_playback`, que roda dez
    /// vezes por segundo. `&mut self` de proposito: quem le a cor le do cache.
    fn refresh_tint(&mut self) {
        if !self.theme.spec.effects.artwork_tint {
            self.now_tint = (None, None);
            return;
        }
        let path = self.now_cover.1.clone();
        if path == self.now_tint.0 {
            return;
        }
        let cor = path.as_deref().and_then(crate::tint::dominant);
        self.now_tint = (path, cor);
    }

    pub fn push_playback(&self, window: &ui::AppWindow) {
        theme_bridge::apply_artwork_color(
            &window.global::<ui::Theme>(),
            self.spec(),
            self.now_tint.1,
        );
        let snapshot = self.engine.snapshot();
        let position = self.shown_position(&snapshot);
        let current = self.queue.current();
        let duration = current.map(|t| t.duration).unwrap_or(snapshot.duration);

        window.set_has_track(current.is_some());
        window.set_now_cover(cover_image(self.now_cover.1.as_deref()));
        // A ficha da faixa tocando sai do album, igual a das linhas de lista.
        let (initial, hue) = cover_badge(
            current
                .and_then(|t| t.album.as_ref().map(|a| a.name.as_ref()))
                .or_else(|| current.map(|t| t.name.as_ref()))
                .unwrap_or(""),
        );
        window.set_now_cover_initial(initial);
        window.set_now_cover_hue(hue);
        window.set_now_cover_pending(!self.now_cover.0.is_empty() && self.now_cover.1.is_none());
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

    fn theme_items(&self) -> Vec<ui::ThemeItem> {
        fn slint_color(c: morune_theme::Color) -> slint::Color {
            slint::Color::from_argb_u8(c.a, c.r, c.g, c.b)
        }

        let active = self.theme_id();
        self.themes
            .iter()
            .map(|entry| ui::ThemeItem {
                id: entry.manifest.id.as_str().into(),
                name: entry.manifest.name.as_str().into(),
                author: entry.manifest.author.as_str().into(),
                builtin: entry.builtin,
                active: entry.manifest.id == active,
                preview_background: slint_color(entry.preview.background),
                preview_surface: slint_color(entry.preview.surface),
                preview_sidebar: slint_color(entry.preview.sidebar),
                preview_player: slint_color(entry.preview.player),
                preview_accent: slint_color(entry.preview.accent),
                preview_text: slint_color(entry.preview.text),
            })
            .collect()
    }

    fn diagnostics(&self) -> Vec<ui::Diagnostic> {
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
        items
    }
}

/// `true` quando a mensagem na tela ja pode sumir sozinha.
///
/// Separado do estado para a regra ficar sob teste: `has_action` marca a
/// mensagem que carrega "Desfazer" ou "Tentar novamente", e essa nunca expira
/// -- e o unico lugar de onde a acao pode ser feita.
/// Traduz a configuracao do usuario para o contrato do backend.
///
/// Existe para que `PlaybackConfig` -- que e formato de arquivo, com campos que
/// nao sao de audio -- nao vaze para dentro do backend, e para que o backend nao
/// precise conhecer o `morune-storage`.
fn audio_settings(config: &morune_storage::config::PlaybackConfig) -> AudioSettings {
    AudioSettings {
        bitrate_kbps: config.bitrate,
        normalize: config.normalize,
        cache_mb: config.audio_cache_mb,
        output_device: config.output_device.clone(),
    }
}

/// Comandos que devolvem ao motor recem-criado o estado que pertence a tela.
fn recreated_engine_commands(
    current: Option<Track>,
    playing: bool,
    volume: f32,
    shuffle: bool,
    repeat: RepeatMode,
) -> Vec<PlayerCommand> {
    let mut commands = vec![
        PlayerCommand::SetVolume(volume),
        PlayerCommand::SetShuffle(shuffle),
        PlayerCommand::SetRepeat(repeat),
    ];
    if let Some(track) = current {
        commands.push(PlayerCommand::Load {
            track,
            start_paused: !playing,
        });
    }
    commands
}

/// A oferta de repetir ainda pertence a mensagem que esta na tela?
///
/// Funcao livre para poder ser testada: e a regra inteira do conserto descrito
/// em [`Retry`], e o `AppState` que a usa nao e construivel sem rede, disco e
/// um motor de audio.
fn retry_belongs_to(retry: Option<&Retry>, status: &str) -> bool {
    retry.is_some_and(|retry| retry.message == status)
}

/// Inicial da conta, para o avatar da barra lateral.
///
/// O caminho de login que o Morune usa nao entrega nome de exibicao nem foto --
/// o `/v1/me` do Web API esta fora do alcance, e o que sobra e o identificador
/// da sessao (ver `morune-spotify/src/auth.rs`). O avatar entao e desenhado a
/// partir do que existe: a primeira letra ou digito do nome.
///
/// Pontuacao no inicio e pulada porque um circulo com "_" nao identifica
/// ninguem, e nomes de usuario comecam com ela com frequencia.
fn account_initial(name: &str) -> String {
    name.chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string())
}

/// Abre um endereco no navegador padrao.
///
/// `explorer.exe` e usado em vez de `cmd /c start` de proposito: `start` abre
/// um console por um instante e interpreta o primeiro argumento entre aspas
/// como titulo da janela, o que ja rendeu bug em outros projetos. O Explorer
/// entrega a URL ao shell direto.
fn open_link(url: &str) -> std::io::Result<()> {
    #[cfg(windows)]
    let mut child = std::process::Command::new("explorer.exe")
        .arg(url)
        .spawn()?;
    #[cfg(not(windows))]
    let mut child = std::process::Command::new("xdg-open").arg(url).spawn()?;

    // Sem isto o processo fica como zumbi ate o Morune sair. Nao esperamos o
    // navegador: `try_wait` so recolhe se ja terminou.
    let _ = child.try_wait();
    Ok(())
}

fn status_expired(age: Duration, has_action: bool, timeout: Duration) -> bool {
    !has_action && age >= timeout
}

fn track_rows(
    tracks: Vec<&Track>,
    current: Option<&Track>,
    covers: &HashMap<String, std::path::PathBuf>,
    liked_ids: &HashSet<TrackId>,
) -> Vec<ui::TrackRow> {
    tracks
        .into_iter()
        .map(|t| {
            let url = track_cover_url(t);
            let arquivo = url.as_deref().and_then(|url| covers.get(url));
            // A ficha da faixa sai do **album**, e nao do titulo: e o album que
            // desenha a capa, entao duas faixas do mesmo disco ganham a mesma
            // cor -- que e o que a capa faria se existisse.
            let (initial, hue) = cover_badge(
                t.album
                    .as_ref()
                    .map(|a| a.name.as_ref())
                    .unwrap_or(t.name.as_ref()),
            );
            ui::TrackRow {
                id: Target::Track(t.id.clone()).tag().into(),
                title: t.name.as_ref().into(),
                artist: t.artists_line().into(),
                album: t
                    .album
                    .as_ref()
                    .map(|a| a.name.as_ref())
                    .unwrap_or("")
                    .into(),
                cover: cover_image(arquivo.map(|p| p.as_path())),
                initial,
                hue,
                cover_pending: url.is_some() && arquivo.is_none(),
                duration: format_time(t.duration).into(),
                playable: t.playable,
                playing: current.is_some_and(|c| c.id == t.id),
                favorite: liked_ids.contains(&t.id),
            }
        })
        .collect()
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

/// Cartoes da barra lateral, com a marca de fixada.
///
/// A marca nao cabe em `card_item` porque o mesmo cartao aparece nas
/// prateleiras do Inicio, onde estar fixada nao significa nada.
fn sidebar_items(cards: &[&Card], pinned: &[String]) -> Vec<ui::CardItem> {
    cards
        .iter()
        .map(|c| ui::CardItem {
            pinned: pinned.iter().any(|tag| tag == &c.tag),
            ..card_item(c)
        })
        .collect()
}

fn card_item(c: &Card) -> ui::CardItem {
    let (initial, hue) = cover_badge(&c.title);
    ui::CardItem {
        id: c.tag.as_str().into(),
        title: c.title.as_str().into(),
        subtitle: c.subtitle.as_str().into(),
        cover: cover_image(c.cover_path.as_deref()),
        initial,
        hue,
        // Ha capa e ela ainda nao chegou ao disco. Sem esta distincao, o item
        // que esta baixando e o que nunca vai ter capa aparecem iguais -- e foi
        // assim que o simbolo do Morune passou a significar as duas coisas.
        cover_pending: !c.cover.is_empty() && c.cover_path.is_none(),
        pinned: false,
    }
}

fn card_items(cards: &[Card]) -> Vec<ui::CardItem> {
    cards.iter().map(card_item).collect()
}

/// Uma lista da interface que sobrevive entre espelhamentos.
///
/// **Por que existe.** `push_to_ui` roda a cada clique -- navegar, pausar,
/// favoritar, mexer no volume -- e ate aqui trocava o `ModelRc` de todas as
/// listas por um novo. Para o Slint, modelo novo e lista nova: cada linha de
/// faixa e cada cartao visivel era destruido e instanciado outra vez, com
/// todas as suas ligacoes, mesmo quando nada neles tinha mudado. Numa Home com
/// as curtidas inteiras e quatro prateleiras, e numa fila de 400 linhas, era
/// isso que aparecia como "o clique corta direto para o resultado" -- e era
/// isso que rolava a lista de volta ao topo.
///
/// Aqui o modelo e um so, criado uma vez. A cada espelhamento a lista nova e
/// **comparada** com a que esta na tela e so as linhas diferentes sao
/// reescritas (`set_row_data`), que para o Slint e uma mudanca de propriedade
/// no item que ja existe, nao um item novo. Linha que nao mudou nao custa nada;
/// comprimento igual e conteudo igual nao acorda a interface.
///
/// Devolver a lista sempre como o **mesmo** `ModelRc` e de proposito: o
/// `set_*` do Slint compara com o valor atual e nao suja a propriedade quando e
/// igual, entao a chamada repetida em `push_to_ui` e gratuita.
struct Lista<T: Clone + PartialEq + 'static> {
    modelo: std::rc::Rc<VecModel<T>>,
}

impl<T: Clone + PartialEq + 'static> Default for Lista<T> {
    fn default() -> Self {
        Self {
            modelo: std::rc::Rc::new(VecModel::default()),
        }
    }
}

impl<T: Clone + PartialEq + 'static> Lista<T> {
    /// O modelo como esta, sem mexer -- para a pagina que nao esta na tela.
    ///
    /// Devolver o mesmo `ModelRc` mantem o `set_*` gratuito; a lista e refeita
    /// quando a pagina voltar a ser a visivel.
    fn como_esta(&self) -> ModelRc<T> {
        ModelRc::from(self.modelo.clone())
    }

    /// Sincroniza so quando `visivel`; caso contrario devolve o modelo como esta.
    fn sincronizar_se(&self, visivel: bool, montar: impl FnOnce() -> Vec<T>) -> ModelRc<T> {
        if visivel {
            self.sincronizar(montar())
        } else {
            self.como_esta()
        }
    }

    /// Deixa o modelo igual a `novo`, mexendo no minimo de linhas.
    fn sincronizar(&self, novo: Vec<T>) -> ModelRc<T> {
        use slint::Model;
        let atual = self.modelo.row_count();
        // Lista que encolheu muito (busca limpa, playlist trocada) e mais barata
        // de trocar inteira do que de remover linha a linha, cada remocao
        // notificando o repetidor.
        if novo.len() * 2 < atual {
            self.modelo.set_vec(novo);
            return ModelRc::from(self.modelo.clone());
        }
        let comum = atual.min(novo.len());
        let mut novo = novo.into_iter();
        for (i, item) in novo.by_ref().take(comum).enumerate() {
            if self.modelo.row_data(i).as_ref() != Some(&item) {
                self.modelo.set_row_data(i, item);
            }
        }
        // O que sobrou em `novo` e o que a lista ganhou (paginacao, capa nova
        // chegando no fim); o excedente do modelo e o que ela perdeu.
        for item in novo {
            self.modelo.push(item);
        }
        for i in (comum..atual).rev() {
            self.modelo.remove(i);
        }
        ModelRc::from(self.modelo.clone())
    }
}

/// Todas as listas que `push_to_ui` espelha. Ver [`Lista`].
#[derive(Default)]
struct Listas {
    output_devices: Lista<SharedString>,
    sidebar_playlists: Lista<ui::CardItem>,
    detail_tracks: Lista<ui::TrackRow>,
    detail_items: Lista<ui::CardItem>,
    queue_manual_tracks: Lista<ui::TrackRow>,
    queue_context_tracks: Lista<ui::TrackRow>,
    themes: Lista<ui::ThemeItem>,
    diagnostics: Lista<ui::Diagnostic>,
    home_made_for_you: Lista<ui::CardItem>,
    home_liked: Lista<ui::TrackRow>,
    home_playlists: Lista<ui::CardItem>,
    home_stations: Lista<ui::CardItem>,
    home_retrospectives: Lista<ui::CardItem>,
    library_items: Lista<ui::CardItem>,
    search_items: Lista<ui::CardItem>,
    search_tracks: Lista<ui::TrackRow>,
}

/// Teto do cache de capas em memoria.
///
/// O cache serve para nao repetir o `stat` de quem esta **na tela**, entao o
/// teto so precisa cobrir a tela mais cara que existe, com folga. As duas
/// piores sao a fila com 400 linhas (capas de 64 px, 16 KB cada: 6,4 MB) e o
/// Inicio cheio de cartoes (capas de 320 px, 400 KB cada: cerca de 12 MB numa
/// tela). 32 MB cobre as duas somadas e ainda sobra para o que veio antes.
///
/// Sem teto, o cache guardava toda capa ja vista pelo resto da sessao. Medido
/// nesta maquina: o cache em disco tem 936 capas, e se a sessao as exibisse
/// todas seriam 81 MB retidos -- num aplicativo cujo orcamento inteiro em
/// repouso e de 78,8 MB.
const COVER_CACHE_MAX_BYTES: usize = 32 * 1024 * 1024;

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
    static COVER_CACHE: std::cell::RefCell<LruCache<std::path::PathBuf, slint::Image>> =
        std::cell::RefCell::new(LruCache::new(COVER_CACHE_MAX_BYTES));
}

/// Cache com teto em bytes, que descarta primeiro o que ha mais tempo nao se usa.
///
/// O teto e em bytes, e nao em numero de entradas, porque as capas variam 25x
/// entre si: a de uma linha de faixa tem 64 px (16 KB) e a de um cartao tem
/// 320 px (400 KB). Um teto por quantidade trataria as duas como iguais e
/// erraria por uma ordem de grandeza em qualquer direcao.
///
/// Sair do cache nao devolve a memoria na hora: enquanto a interface ainda
/// mostrar aquela capa, ela continua viva pela referencia de la. O que o teto
/// garante e que nada fique retido **apenas** por ter sido visto uma vez.
struct LruCache<K, V> {
    entries: HashMap<K, CacheEntry<V>>,
    bytes: usize,
    max_bytes: usize,
    /// Relogio logico: cada acesso recebe o proximo numero, e o menor numero e
    /// o candidato mais antigo. Um `u64` nao da a volta em nenhuma sessao real.
    clock: u64,
}

struct CacheEntry<V> {
    value: V,
    bytes: usize,
    used: u64,
}

impl<K: std::hash::Hash + Eq + Clone, V: Clone> LruCache<K, V> {
    fn new(max_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            bytes: 0,
            max_bytes,
            clock: 0,
        }
    }

    fn get(&mut self, key: &K) -> Option<V> {
        self.clock += 1;
        let clock = self.clock;
        let entry = self.entries.get_mut(key)?;
        entry.used = clock;
        Some(entry.value.clone())
    }

    fn insert(&mut self, key: K, value: V, bytes: usize) {
        self.clock += 1;
        if let Some(old) = self.entries.insert(
            key,
            CacheEntry {
                value,
                bytes,
                used: self.clock,
            },
        ) {
            self.bytes -= old.bytes;
        }
        self.bytes += bytes;
        self.evict();
    }

    /// Descarta os mais antigos ate voltar para dentro do teto.
    ///
    /// A ordenacao e O(n log n), mas so acontece quando o teto e ultrapassado,
    /// e a cada vez ela libera espaco para muitas insercoes seguintes.
    fn evict(&mut self) {
        if self.bytes <= self.max_bytes {
            return;
        }

        let mut idade: Vec<(u64, K)> = self
            .entries
            .iter()
            .map(|(k, e)| (e.used, k.clone()))
            .collect();
        idade.sort_unstable_by_key(|(used, _)| *used);

        for (_, key) in idade {
            if self.bytes <= self.max_bytes {
                break;
            }
            if let Some(removed) = self.entries.remove(&key) {
                self.bytes -= removed.bytes;
            }
        }
    }
}

/// Quanto uma imagem decodificada ocupa, em bytes.
///
/// O Slint guarda os pixels em RGBA de 8 bits. Uma imagem vazia -- capa que
/// nao chegou ou nao decodificou -- tem dimensao zero e nao ocupa nada, o que
/// e correto: ela e so um marcador.
fn image_bytes(image: &slint::Image) -> usize {
    let size = image.size();
    size.width as usize * size.height as usize * 4
}

/// Inicial e matiz da ficha que substitui a capa ausente.
///
/// **Por que isto vive no Rust.** O Slint nao fatia string nem le codigo de
/// caractere, entao nem a inicial nem uma matiz derivada do nome dao para
/// calcular la. A alternativa era mandar uma cor pronta, mas cor pronta ignora
/// o tema -- assim o Rust manda so o angulo e o `Artwork` decide saturacao e
/// brilho conforme o tema for claro ou escuro.
///
/// A matiz e **estavel para o mesmo nome**: a mesma playlist tem sempre a mesma
/// cor, em qualquer sessao e em qualquer maquina. Uma cor sorteada a cada
/// abertura seria pior que o simbolo repetido -- o item deixaria de ser
/// reconhecivel de relance, que e justamente o que a ficha existe para dar.
/// A pessoa pediu ao Windows para nao ver animacao?
///
/// `SPI_GETCLIENTAREAANIMATION` e o que Acessibilidade -> Efeitos visuais ->
/// "Mostrar animacoes no Windows" liga e desliga. O Morune ja tinha um ajuste
/// proprio de movimento; o que faltava era o pedido feito ao sistema chegar
/// ate aqui -- ninguem troca de tema, nem abre as Configuracoes de um tocador
/// de musica, para conseguir usar o computador.
///
/// **Lido uma vez, na abertura.** Reagir a `WM_SETTINGCHANGE` seria escopo
/// proprio, e quase nenhum aplicativo faz.
#[cfg(windows)]
fn system_animation_enabled() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPI_GETCLIENTAREAANIMATION, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    };

    let mut ligada = windows::core::BOOL(1);
    // SAFETY: ponteiro para variavel local valida, do tamanho que a API pede.
    let resultado = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some(&mut ligada as *mut _ as *mut core::ffi::c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };

    // Falha na consulta nao pode desligar o movimento de quem nao pediu isso:
    // sem resposta, vale o comportamento normal.
    resultado.is_err() || ligada.as_bool()
}

#[cfg(not(windows))]
fn system_animation_enabled() -> bool {
    true
}

fn cover_badge(nome: &str) -> (slint::SharedString, f32) {
    // FNV-1a de 32 bits sobre o nome normalizado. Escolhido por ser curto e
    // determinista; nao ha nada criptografico em jogo, so espalhamento.
    let mut hash: u32 = 0x811c_9dc5;
    for byte in nome.trim().to_lowercase().bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }

    let inicial = nome
        .trim()
        .chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_default();

    (inicial.into(), (hash % 360) as f32)
}

/// Carrega a capa do arquivo, ou devolve uma imagem vazia.
///
/// Imagem vazia nao e falta de tratamento: e o que faz o cartao mostrar o
/// bloco neutro no lugar, sem mudar o layout quando a capa de verdade chegar.
fn cover_image(path: Option<&std::path::Path>) -> slint::Image {
    let Some(path) = path else {
        return slint::Image::default();
    };

    if let Some(cached) = COVER_CACHE.with(|c| c.borrow_mut().get(&path.to_path_buf())) {
        return cached;
    }

    let image = slint::Image::load_from_path(path).unwrap_or_else(|e| {
        // Arquivo truncado ou formato inesperado: o cartao fica sem capa e o
        // aplicativo segue. Trocar isto por `expect` derrubaria a tela por
        // causa de um JPEG ruim.
        tracing::debug!(path = %path.display(), error = ?e, "capa nao decodificou");
        slint::Image::default()
    });
    let bytes = image_bytes(&image);
    COVER_CACHE.with(|c| {
        c.borrow_mut()
            .insert(path.to_path_buf(), image.clone(), bytes);
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

/// Ordena apenas o que tem historico. O sort estavel conserva a sequencia de
/// entrada para todas as playlists que ainda nao foram abertas no Morune.
fn playlists_by_recent<'a>(playlists: Vec<&'a Card>, recent: &[String]) -> Vec<&'a Card> {
    let positions: HashMap<&str, usize> = recent
        .iter()
        .enumerate()
        .map(|(position, tag)| (tag.as_str(), position))
        .collect();
    let mut ordered = playlists;
    ordered.sort_by_key(|card| {
        positions
            .get(card.tag.as_str())
            .copied()
            .unwrap_or(usize::MAX)
    });
    ordered
}

/// Teto de playlists fixadas. Mesmo numero que `Config::sanitize` aplica ao
/// arquivo, para que fixar pela interface nunca produza algo que a leitura da
/// configuracao corte depois.
const PINNED_LIMIT: usize = 50;

/// Monta a barra lateral: curtidas, fixadas, depois o historico.
///
/// As curtidas ficam acima ate das fixadas. Nao sao playlist do provedor e nao
/// podem ser fixadas nem desafixadas: sao a colecao da conta, e um topo que
/// muda de dono conforme o que o usuario fixou tiraria delas o unico lugar em
/// que sempre estiveram.
///
/// O filtro de texto entra por ultimo e vale para as tres faixas: uma barra em
/// que as fixadas ignorassem o que foi digitado nao estaria filtrada.
fn sidebar_order<'a>(
    liked: &'a Card,
    playlists: impl Iterator<Item = &'a Card>,
    pinned: &[String],
    recent: &[String],
    filtro: &str,
) -> Vec<&'a Card> {
    let ordem_fixada: HashMap<&str, usize> = pinned
        .iter()
        .enumerate()
        .map(|(posicao, tag)| (tag.as_str(), posicao))
        .collect();

    // Tag fixada de playlist que sumiu da conta fica inerte de graca: a
    // intersecao e sempre contra os cartoes que existem agora.
    let (mut fixadas, resto): (Vec<&Card>, Vec<&Card>) =
        playlists.partition(|card| ordem_fixada.contains_key(card.tag.as_str()));
    fixadas.sort_by_key(|card| ordem_fixada[card.tag.as_str()]);

    std::iter::once(liked)
        .chain(fixadas)
        .chain(playlists_by_recent(resto, recent))
        .filter(|c| filtro.is_empty() || c.title.to_lowercase().contains(filtro))
        .collect()
}

/// Opacidade minima que a janela pode ter.
///
/// Alta de proposito: uma janela quase invisivel deixaria o aplicativo
/// irrecuperavel pelo proprio usuario. E o mesmo piso que `sanitize` aplica ao
/// `window_opacity` do tema.
const OPACIDADE_MINIMA: f32 = 0.2;

/// Converte a posicao do slider em opacidade.
///
/// `0` na esquerda e janela opaca; `1` na direita e o limite. O sentido e o de
/// "quanta transparencia", que e como a pessoa le o rotulo -- e nao o de
/// "quanta opacidade", que sairia invertido na tela.
fn slider_para_opacidade(valor: f32) -> f32 {
    let v = if valor.is_finite() { valor } else { 0.0 };
    (1.0 - v.clamp(0.0, 1.0) * (1.0 - OPACIDADE_MINIMA)).clamp(OPACIDADE_MINIMA, 1.0)
}

/// O caminho de volta, para o slider nascer onde o valor guardado manda.
fn opacidade_para_slider(opacidade: f32) -> f32 {
    if !opacidade.is_finite() {
        return 0.0;
    }
    ((1.0 - opacidade) / (1.0 - OPACIDADE_MINIMA)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod cover_cache_tests {
    use super::LruCache;

    /// Capa de linha de faixa: 64 px em RGBA.
    const PEQUENA: usize = 64 * 64 * 4;
    /// Capa de cartao: 320 px em RGBA. Vinte e cinco vezes a de cima -- e a
    /// razao de o teto ser em bytes, e nao em numero de entradas.
    const GRANDE: usize = 320 * 320 * 4;

    fn cache(max: usize) -> LruCache<String, u32> {
        LruCache::new(max)
    }

    #[test]
    fn what_fits_under_the_ceiling_stays() {
        let mut c = cache(GRANDE * 4);
        for i in 0..4u32 {
            c.insert(format!("capa{i}"), i, GRANDE);
        }

        for i in 0..4u32 {
            assert_eq!(c.get(&format!("capa{i}")), Some(i), "capa{i} deveria estar");
        }
        assert_eq!(c.bytes, GRANDE * 4);
    }

    #[test]
    fn the_ceiling_is_never_exceeded() {
        let mut c = cache(GRANDE * 3);
        for i in 0..40u32 {
            c.insert(format!("capa{i}"), i, GRANDE);
            assert!(
                c.bytes <= GRANDE * 3,
                "cache passou do teto na insercao {i}: {} bytes",
                c.bytes
            );
        }
    }

    #[test]
    fn the_oldest_untouched_entry_is_the_one_to_go() {
        let mut c = cache(GRANDE * 3);
        c.insert("a".into(), 1, GRANDE);
        c.insert("b".into(), 2, GRANDE);
        c.insert("c".into(), 3, GRANDE);

        // "a" volta a ser usada, entao "b" passa a ser a mais antiga.
        assert_eq!(c.get(&"a".to_string()), Some(1));
        c.insert("d".into(), 4, GRANDE);

        assert_eq!(c.get(&"b".to_string()), None, "b era a mais antiga");
        assert_eq!(c.get(&"a".to_string()), Some(1), "a foi usada de novo");
        assert_eq!(c.get(&"c".to_string()), Some(3));
        assert_eq!(c.get(&"d".to_string()), Some(4));
    }

    #[test]
    fn one_big_cover_does_not_evict_the_whole_screen() {
        // O que o teto por bytes protege: uma tela cheia de capas pequenas nao
        // pode ser varrida por causa de uma unica capa grande. Com teto por
        // quantidade de entradas, seria uma saindo para cada uma que entra.
        let mut c = cache(PEQUENA * 400 + GRANDE);
        for i in 0..400u32 {
            c.insert(format!("linha{i}"), i, PEQUENA);
        }
        c.insert("cartao".into(), 999, GRANDE);

        let sobreviventes = (0..400u32)
            .filter(|i| c.get(&format!("linha{i}")).is_some())
            .count();
        assert_eq!(sobreviventes, 400, "a fila inteira deveria caber junto");
    }

    #[test]
    fn replacing_an_entry_does_not_double_count_it() {
        let mut c = cache(GRANDE * 10);
        c.insert("a".into(), 1, GRANDE);
        c.insert("a".into(), 2, GRANDE);

        assert_eq!(c.bytes, GRANDE, "a mesma chave contada duas vezes");
        assert_eq!(c.get(&"a".to_string()), Some(2));
    }

    #[test]
    fn an_empty_image_costs_nothing_and_is_still_remembered() {
        // Capa que nao decodificou entra com zero byte: guardar a falha e o que
        // evita repetir o `stat` para redescobrir que ela nao decodifica.
        let mut c = cache(PEQUENA);
        c.insert("quebrada".into(), 0, 0);
        for i in 0..50u32 {
            c.insert(format!("outra{i}"), i, 0);
        }

        assert_eq!(c.bytes, 0);
        assert_eq!(c.get(&"quebrada".to_string()), Some(0));
    }
}

#[cfg(test)]
mod tests {
    use super::{opacidade_para_slider, slider_para_opacidade, OPACIDADE_MINIMA};

    #[test]
    fn o_slider_de_transparencia_ida_e_volta() {
        for passo in 0..=10 {
            let v = passo as f32 / 10.0;
            let volta = opacidade_para_slider(slider_para_opacidade(v));
            assert!((volta - v).abs() < 0.001, "{v} virou {volta}");
        }
    }

    #[test]
    fn a_esquerda_do_slider_e_janela_opaca() {
        assert_eq!(slider_para_opacidade(0.0), 1.0);
    }

    #[test]
    fn a_direita_do_slider_para_no_piso_e_nao_no_invisivel() {
        // Sem o piso, arrastar o slider ate o fim deixaria o aplicativo
        // irrecuperavel pelo proprio usuario.
        assert_eq!(slider_para_opacidade(1.0), OPACIDADE_MINIMA);
        assert_eq!(slider_para_opacidade(50.0), OPACIDADE_MINIMA);
    }

    #[test]
    fn valor_invalido_vira_janela_opaca() {
        assert_eq!(slider_para_opacidade(f32::NAN), 1.0);
        assert_eq!(opacidade_para_slider(f32::NAN), 0.0);
    }

    use super::*;

    /// A lista na tela recebe so o que mudou: mesmo modelo, linha a linha.
    #[test]
    fn sincronizar_reescreve_so_as_linhas_diferentes_e_mantem_o_modelo() {
        use slint::Model;
        let lista: Lista<SharedString> = Lista::default();
        let m1 = lista.sincronizar(vec!["a".into(), "b".into(), "c".into()]);
        let m2 = lista.sincronizar(vec!["a".into(), "B".into(), "c".into(), "d".into()]);
        assert!(m1 == m2, "o ModelRc devolvido e sempre o mesmo");
        assert_eq!(m2.row_count(), 4);
        assert_eq!(m2.row_data(1).unwrap(), SharedString::from("B"));
        assert_eq!(m2.row_data(3).unwrap(), SharedString::from("d"));

        // Encolher pouco remove do fim; encolher muito troca de uma vez.
        let m3 = lista.sincronizar(vec!["a".into(), "B".into(), "c".into()]);
        assert_eq!(m3.row_count(), 3);
        let m4 = lista.sincronizar(vec!["z".into()]);
        assert_eq!(m4.row_count(), 1);
        assert_eq!(m4.row_data(0).unwrap(), SharedString::from("z"));
        assert!(m1 == m4);
    }

    fn playback_track(id: &str) -> Track {
        Track {
            id: TrackId::spotify(id),
            name: id.into(),
            artists: vec![],
            album: None,
            duration: Duration::from_secs(180),
            track_number: None,
            disc_number: None,
            explicit: false,
            playable: true,
        }
    }

    fn detail(id: &str) -> crate::browse::Detail {
        crate::browse::Detail {
            origin: QueueOrigin::Custom(id.into()),
            title: id.into(),
            subtitle: String::new(),
            kind: "Playlist".into(),
            cover: String::new(),
            cover_path: None,
            tracks: vec![playback_track(id)],
            cards: Vec::new(),
            total_tracks: Some(1),
            source: None,
            has_more: false,
        }
    }

    #[test]
    fn nested_detail_history_preserves_the_parent_reading_state() {
        let parent = detail("playlist-pai");
        let mut history = vec![DetailHistory {
            detail: parent,
            from: Page::Library,
            filter: "ao vivo".into(),
            sort: SortBy::Artist,
            desc: true,
        }];

        let restored = history
            .pop()
            .expect("o detalhe pai deve ficar no historico");

        assert_eq!(restored.detail.title, "playlist-pai");
        assert_eq!(restored.from, Page::Library);
        assert_eq!(restored.filter, "ao vivo");
        assert_eq!(restored.sort, SortBy::Artist);
        assert!(restored.desc);
        assert!(history.is_empty());
    }

    #[test]
    fn recreated_engine_recovers_preferences_and_resumes_the_current_track() {
        let track = playback_track("4cOdK2wGLETKBW3PvgPWqT");
        assert_eq!(
            recreated_engine_commands(Some(track.clone()), true, 0.4, true, RepeatMode::One),
            vec![
                PlayerCommand::SetVolume(0.4),
                PlayerCommand::SetShuffle(true),
                PlayerCommand::SetRepeat(RepeatMode::One),
                PlayerCommand::Load {
                    track,
                    start_paused: false,
                },
            ]
        );
    }

    #[test]
    fn recreated_engine_keeps_a_non_playing_track_ready_at_its_start() {
        let track = playback_track("4cOdK2wGLETKBW3PvgPWqT");
        let commands =
            recreated_engine_commands(Some(track.clone()), false, 1.0, false, RepeatMode::Off);

        assert_eq!(
            commands.last(),
            Some(&PlayerCommand::Load {
                track,
                start_paused: true,
            })
        );
    }

    #[test]
    fn recreated_engine_without_a_track_only_recovers_preferences() {
        assert_eq!(
            recreated_engine_commands(None, false, 0.7, true, RepeatMode::All),
            vec![
                PlayerCommand::SetVolume(0.7),
                PlayerCommand::SetShuffle(true),
                PlayerCommand::SetRepeat(RepeatMode::All),
            ]
        );
    }

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

    fn card(id: &str) -> Card {
        Card {
            tag: format!("playlist/spotify:{id}"),
            title: id.into(),
            subtitle: String::new(),
            cover: String::new(),
            cover_path: None,
        }
    }

    fn liked_card() -> Card {
        Card {
            tag: crate::browse::Target::Liked.tag(),
            title: crate::browse::LIKED_TITLE.into(),
            subtitle: String::new(),
            cover: String::new(),
            cover_path: None,
        }
    }

    fn tag(id: &str) -> String {
        format!("playlist/spotify:{id}")
    }

    /// Titulos na ordem em que a barra lateral os desenharia.
    fn sidebar(cards: &[Card], pinned: &[String], recent: &[String], filtro: &str) -> Vec<String> {
        let liked = liked_card();
        sidebar_order(&liked, cards.iter(), pinned, recent, filtro)
            .into_iter()
            .map(|item| item.title.clone())
            .collect()
    }

    #[test]
    fn sidebar_puts_recent_first_and_preserves_the_rest() {
        let playlists = [card("a"), card("b"), card("c"), card("d")];
        let recent = vec![tag("c"), tag("a")];

        let ordered: Vec<&str> = playlists_by_recent(playlists.iter().collect(), &recent)
            .into_iter()
            .map(|item| item.title.as_str())
            .collect();

        assert_eq!(ordered, ["c", "a", "b", "d"]);
    }

    #[test]
    fn pinned_come_after_liked_and_before_the_rest() {
        let cards = vec![card("a"), card("b"), card("c")];
        let ordem = sidebar(&cards, &[tag("c")], &[tag("b")], "");

        assert_eq!(ordem, [crate::browse::LIKED_TITLE, "c", "b", "a"]);
    }

    #[test]
    fn pinning_keeps_the_order_in_which_things_were_pinned() {
        let cards = vec![card("a"), card("b"), card("c")];
        // `toggle_pin_playlist` insere na frente, entao a mais recente e a
        // primeira da lista guardada.
        let ordem = sidebar(&cards, &[tag("c"), tag("a")], &[], "");

        assert_eq!(ordem, [crate::browse::LIKED_TITLE, "c", "a", "b"]);
    }

    #[test]
    fn an_unpinned_playlist_falls_back_to_its_place_in_the_history() {
        let cards = vec![card("a"), card("b"), card("c")];
        let recent = vec![tag("c"), tag("a")];

        assert_eq!(
            sidebar(&cards, &[], &recent, ""),
            [crate::browse::LIKED_TITLE, "c", "a", "b"]
        );
    }

    #[test]
    fn a_pinned_playlist_never_appears_twice() {
        let cards = vec![card("a"), card("b")];
        let ordem = sidebar(&cards, &[tag("a")], &[tag("a"), tag("b")], "");

        assert_eq!(ordem.iter().filter(|title| *title == "a").count(), 1);
        assert_eq!(ordem, [crate::browse::LIKED_TITLE, "a", "b"]);
    }

    #[test]
    fn a_pin_for_a_playlist_that_no_longer_exists_is_inert() {
        let cards = vec![card("a")];
        let ordem = sidebar(&cards, &[tag("sumiu"), tag("a")], &[], "");

        assert_eq!(ordem, [crate::browse::LIKED_TITLE, "a"]);
    }

    #[test]
    fn the_filter_also_hides_pinned_playlists() {
        let cards = vec![card("abelha"), card("zebra")];
        let ordem = sidebar(&cards, &[tag("abelha")], &[], "zeb");

        assert_eq!(ordem, ["zebra"]);
    }

    /// A barra lateral e o `rootlist` inteiro: mixes, estacoes e retrospectivas
    /// chegam nela pelos vetores que o Inicio separa por prateleira.
    #[test]
    fn every_kind_of_playlist_reaches_the_sidebar() {
        let liked = liked_card();
        let pessoais = [card("minha")];
        let feito = [card("daily-mix")];
        let estacoes = [card("mix-rock")];
        let retro = [card("mais-ouvidas")];

        let todas = pessoais.iter().chain(&feito).chain(&estacoes).chain(&retro);
        let ordem: Vec<&str> = sidebar_order(&liked, todas, &[], &[], "")
            .into_iter()
            .map(|item| item.title.as_str())
            .collect();

        assert_eq!(
            ordem,
            [
                crate::browse::LIKED_TITLE,
                "minha",
                "daily-mix",
                "mix-rock",
                "mais-ouvidas"
            ]
        );
    }

    #[test]
    fn a_inicial_do_avatar_e_a_primeira_letra() {
        assert_eq!(account_initial("seititm"), "S");
        assert_eq!(account_initial("Felipe"), "F");
    }

    #[test]
    fn a_inicial_pula_pontuacao_e_aceita_acento() {
        // Nomes de usuario comecam com underline e ponto com frequencia, e um
        // circulo com "_" dentro nao identifica ninguem.
        assert_eq!(account_initial("_ana"), "A");
        assert_eq!(account_initial(".2pac"), "2");
        assert_eq!(account_initial("álvaro"), "Á");
    }

    /// Sem sessao o avatar nem aparece, mas a funcao nao pode entrar em panico
    /// no caminho que espelha a interface a cada clique.
    #[test]
    fn a_inicial_de_um_nome_vazio_nao_quebra() {
        assert_eq!(account_initial(""), "?");
        assert_eq!(account_initial("___"), "?");
    }

    fn falha(mensagem: &str) -> Retry {
        Retry {
            target: RetryTarget::Page(Page::Home),
            message: mensagem.into(),
        }
    }

    #[test]
    fn a_oferta_de_repetir_vale_enquanto_a_mensagem_dela_esta_na_tela() {
        let retry = falha("Não foi possível carregar o Início.");
        assert!(retry_belongs_to(
            Some(&retry),
            "Não foi possível carregar o Início."
        ));
    }

    /// O defeito que isto conserta: o login sobrescrevia a mensagem de erro e o
    /// botao continuava na tela, colado numa frase de sucesso -- e como
    /// mensagem com acao nao expira, o aviso ficava pregado para sempre.
    #[test]
    fn outra_mensagem_leva_a_oferta_junto() {
        let retry = falha("Não foi possível carregar o Início.");
        assert!(!retry_belongs_to(Some(&retry), "Conectado como seititm."));
    }

    #[test]
    fn sem_falha_nao_ha_o_que_repetir() {
        assert!(!retry_belongs_to(None, "Conectado como seititm."));
        assert!(!retry_belongs_to(None, ""));
    }

    /// Sem a oferta, a mensagem volta a ter prazo -- que e o comportamento que
    /// `status_expired` implementa e que o `bool` solto desligava.
    #[test]
    fn mensagem_sem_oferta_volta_a_expirar() {
        let retry = falha("Não foi possível carregar o Início.");
        let tem_acao = retry_belongs_to(Some(&retry), "Conectado como seititm.");
        assert!(status_expired(
            Duration::from_secs(30),
            tem_acao,
            AppState::STATUS_TIMEOUT
        ));
    }
}
