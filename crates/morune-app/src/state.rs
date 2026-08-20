//! Estado da aplicacao e as acoes que a interface dispara.
//!
//! Concentrar tudo aqui deixa `main.rs` sendo so fiacao, e mantem a interface
//! sem nenhuma decisao de produto: cada callback do Slint chama um metodo deste
//! tipo e depois pede um `push_to_ui`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use morune_core::playback::{NullEngine, PlaybackEngine, PlayerCommand, PlayerEvent};
use morune_core::queue::{Queue, QueueOrigin, RepeatMode};
use morune_core::Track;
use morune_storage::{AppPaths, Config};
use morune_theme::{loader, ThemeSpec};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use tokio::sync::broadcast;

use crate::browse::{AutoplayOutcome, Card, Home, Outcome, Target};
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
            session: Session::new(Arc::from(morune_storage::platform_store()), covers_dir),
            player_events: None,
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
            playlist_filter: String::new(),
            detail: None,
            detail_filter: String::new(),
            detail_sort: SortBy::Original,
            detail_desc: false,
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

        changed
    }

    /// Aplica o que a busca ou a biblioteca trouxeram.
    fn apply_browse(&mut self, outcome: Outcome) {
        match outcome {
            Outcome::Search { query, tracks, cards } => {
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
                    playlists,
                } = *home;
                self.home_made_for_you = made_for_you;

                self.liked = TrackList {
                    origin: QueueOrigin::Custom("Musicas curtidas".into()),
                    tracks: liked,
                };
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
                self.detail = Some(*detail);
                self.detail_from = self.page;
                self.page = Page::Detail;
                self.status.clear();
                self.resolve_detail_cover();
                self.resolve_detail_cards();
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

    /// Guarda o texto digitado no filtro da barra lateral.
    pub fn set_playlist_filter(&mut self, texto: &str) {
        self.playlist_filter = texto.trim().to_lowercase();
    }

    /// Playlists da barra lateral, ja filtradas.
    ///
    /// A ordem e a que o Spotify mandou, e isso e deliberado: o `rootlist` e a
    /// barra lateral do cliente oficial, entao aquela e a ordem que o usuario
    /// arrumou. Reordenar seria descartar informacao real -- e nao ha por que
    /// reordenar, porque nenhuma playlist traz data.
    fn sidebar_playlists(&self) -> Vec<&Card> {
        std::iter::once(&self.liked_card)
            .chain(self.home_playlists.iter())
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
    }

    /// Fecha a tela de detalhe e volta de onde ela foi aberta.
    pub fn close_detail(&mut self) {
        self.page = self.detail_from;
        self.detail = None;
    }

    /// Toca a lista aberta a partir da primeira faixa visivel.
    ///
    /// "Visivel" e deliberado: com filtro aplicado, tocar tem de tocar o que
    /// esta na tela, e nao a lista inteira que o usuario acabou de esconder.
    pub fn play_detail(&mut self) {
        self.play_detail_from(0);
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

        self.play_detail_from(index);
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
        [&self.search, &self.liked].into_iter().find_map(|list| {
            let index = list.tracks.iter().position(|t| t.id == *id)?;
            Some((list.origin.clone(), list.tracks.clone(), index))
        })
    }

    /// Carrega a faixa selecionada na fila e comeca a tocar.
    fn play_current(&mut self) {
        if let Some(track) = self.queue.current().cloned() {
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
        window
            .window()
            .set_size(slint::LogicalSize::new(width, height));
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
        if query.trim().is_empty() {
            return;
        }
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
        self.search_query = query.trim().to_string();
        self.searching = true;
        self.search = TrackList::default();
        self.search_cards.clear();
        browse.search(query);
        self.status = format!("Buscando \"{query}\"...");
    }

    pub fn clear_status(&mut self) {
        self.status.clear();
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

        if let Target::Track(id) = &target {
            // Na fila: e so pular para ela, mantendo o contexto que ja estava
            // tocando.
            if let Some(index) = self.queue.tracks().iter().position(|t| t.id == *id) {
                self.queue.jump_to(index);
                self.play_current();
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
        self.status = "Carregando...".into();
    }

    pub fn toggle_play(&mut self) {
        self.send(PlayerCommand::TogglePlay);
    }

    pub fn next_track(&mut self) {
        if let Some(track) = self.queue.next(true).cloned() {
            self.send(PlayerCommand::Load {
                track,
                start_paused: false,
            });
        } else {
            self.send(PlayerCommand::Stop);
        }
    }

    pub fn previous_track(&mut self) {
        if let Some(track) = self.queue.previous().cloned() {
            self.send(PlayerCommand::Load {
                track,
                start_paused: false,
            });
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
        self.autoplay_seed = None;
        // Busca, inicio e biblioteca sao da conta que saiu: deixa-los na tela
        // mostraria a playlist de alguem que nao esta mais conectado.
        self.search = TrackList::default();
        self.liked = TrackList::default();
        self.home_made_for_you.clear();
        self.home_stations.clear();
        self.home_retrospectives.clear();
        self.home_playlists.clear();
        self.library.clear();
        self.search = TrackList::default();
        self.search_cards.clear();
        self.search_query.clear();
        self.searching = false;
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
        window.set_logged_in(self.session.state().is_logged_in());
        window.set_account_name(SharedString::from(self.session.state().account_name()));
        window.set_dev_mode(self.config.developer.enabled);
        window.set_close_to_tray(self.config.window.close_to_tray);
        window.set_autoplay(self.config.playback.autoplay);
        window.set_search_query(self.search_query.as_str().into());
        window.set_searching(self.searching);

        let snapshot = self.engine.snapshot();
        let current = self.queue.current();
        window.set_has_track(current.is_some());
        window.set_now_cover(cover_image(self.now_cover.1.as_deref()));
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
            ));
            window.set_detail_sort(self.detail_sort as i32);
            window.set_detail_descending(self.detail_desc);
            window.set_detail_filtered(!self.detail_filter.is_empty());
            window.set_detail_truncated(
                detail
                    .total_tracks
                    .is_some_and(|total| total as usize > detail.tracks.len()),
            );
            window.set_detail_loaded_count(detail.tracks.len() as i32);
            window.set_detail_items(card_items(&detail.cards));
        }
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

        window.set_queue_tracks(track_rows(
            self.queue.upcoming(200),
            current,
            &self.track_covers,
        ));
        window.set_themes(self.theme_items());
        window.set_diagnostics(self.diagnostics());
        window.set_home_made_for_you(card_items(&self.home_made_for_you));
        window.set_home_liked(track_rows(
            self.liked.tracks.iter().collect(),
            current,
            &self.track_covers,
        ));
        window.set_home_stations(card_items(&self.home_stations));
        window.set_home_retrospectives(card_items(&self.home_retrospectives));
        window.set_library_items(card_items(&self.library));
        window.set_search_items(card_items(&self.search_cards));
        window.set_search_tracks(track_rows(
            self.search.tracks.iter().collect(),
            current,
            &self.track_covers,
        ));
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

fn track_rows(
    tracks: Vec<&Track>,
    current: Option<&Track>,
    covers: &HashMap<String, std::path::PathBuf>,
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

/// Carrega a capa do arquivo, ou devolve uma imagem vazia.
///
/// Imagem vazia nao e falta de tratamento: e o que faz o cartao mostrar o
/// bloco neutro no lugar, sem mudar o layout quando a capa de verdade chegar.
///
/// A decodificacao acontece na thread da interface, e e barata o bastante para
/// isso: sao arquivos de 300 px que o Slint mantem em cache proprio depois do
/// primeiro carregamento.
fn cover_image(path: Option<&std::path::Path>) -> slint::Image {
    let Some(path) = path else {
        return slint::Image::default();
    };

    slint::Image::load_from_path(path).unwrap_or_else(|e| {
        // Arquivo truncado ou formato inesperado: o cartao fica sem capa e o
        // aplicativo segue. Trocar isto por `expect` derrubaria a tela por
        // causa de um JPEG ruim.
        tracing::debug!(path = %path.display(), error = ?e, "capa nao decodificou");
        slint::Image::default()
    })
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
