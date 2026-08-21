//! Busca e biblioteca vistas pela interface.
//!
//! Vale aqui a mesma regra da sessao: a thread da interface nao espera rede.
//! Cada pedido vira uma tarefa no runtime do backend e o resultado e recolhido
//! depois, no temporizador que ja atende bandeja e reproducao.
//!
//! **Um pedido de cada vez, e o ultimo ganha.** Quem digita numa caixa de busca
//! produz um pedido por letra; o unico resultado que interessa e o da ultima.
//! Guardar so um canal faz o anterior ser descartado sozinho, sem cancelamento
//! explicito e sem resultado antigo pintando por cima do novo.

use std::sync::Arc;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::artwork::{ArtworkCache, Ready};
use std::sync::mpsc::{Receiver, TryRecvError};

use morune_core::catalog::{Artwork, Catalog, Library, SearchKind};
use morune_core::model::{AlbumId, ArtistId, PlaylistId, PlaylistKind, Provider, Track, TrackId};
use morune_core::queue::QueueOrigin;
use morune_core::{CoreError, CoreResult};

/// Quantas faixas uma busca traz.
///
/// E o teto do Web API numa requisicao so. Passar disso exigiria paginar a
/// busca, e ninguem rola cinquenta resultados procurando uma musica.
const SEARCH_LIMIT: u32 = 50;

/// Quantos itens cada secao de biblioteca traz de uma vez.
const SECTION_LIMIT: u32 = 50;

/// Quantas faixas aparecem na prateleira de curtidas do Inicio.
///
/// A prateleira e um bloco com rolagem propria, e nao um recorte de amostra:
/// serve para achar e escolher uma musica sem sair do Inicio. 50 e o lote que o
/// provedor entrega numa requisicao, entao pedir isso nao custa nada a mais.
pub(crate) const SHELF_TRACKS: u32 = 50;

/// Titulo da lista de curtidas, num lugar so.
///
/// Aparece na barra lateral, na prateleira do Inicio e como origem da fila.
/// Escrito tres vezes, envelheceria em tres lugares.
pub const LIKED_TITLE: &str = "Musicas curtidas";

/// Quantas faixas a tela pede por vez.
///
/// E pagina, nao teto: a interface pede a proxima quando a pessoa se aproxima
/// do fim. O provedor ainda pode devolver menos (curtidas vem em lotes de 50),
/// e [`morune_core::catalog::Page`] informa se existe continuacao.
const DETAIL_PAGE: u32 = 100;

/// O que a interface pode ativar com um clique.
///
/// A interface trafega tudo como texto (o Slint so tem `string` nos modelos),
/// entao o tipo do recurso viaja junto com o id. Sem isso, `spotify:4cOdK2...`
/// seria faixa, album ou playlist conforme o dia.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Track(TrackId),
    Album(AlbumId),
    Playlist(PlaylistId),
    Artist(ArtistId),
    /// Musicas curtidas.
    ///
    /// Nao tem id porque nao e uma playlist: e a colecao da conta, e o
    /// Spotify a trata como coisa a parte. Aqui ela ganha um alvo proprio
    /// para poder ser ativada como qualquer outra lista.
    Liked,
}

impl Target {
    /// Forma textual usada nos modelos do Slint: `tipo/provedor:id`.
    pub fn tag(&self) -> String {
        match self {
            Target::Track(id) => format!("track/{id}"),
            Target::Album(id) => format!("album/{id}"),
            Target::Playlist(id) => format!("playlist/{id}"),
            Target::Artist(id) => format!("artist/{id}"),
            Target::Liked => "liked".into(),
        }
    }

    /// Recupera o alvo a partir da forma textual.
    ///
    /// Um id sem tipo e lido como faixa: e o que a fila usava antes de haver
    /// catalogo, e continuar aceitando evita quebrar cache antigo.
    pub fn parse(tag: &str) -> Option<Self> {
        if tag == "liked" {
            return Some(Target::Liked);
        }

        let (kind, id) = tag.split_once('/').unwrap_or(("track", tag));
        let (provider, id) = id.split_once(':')?;
        let provider = match provider {
            "spotify" => Provider::Spotify,
            "local" => Provider::Local,
            _ => return None,
        };
        if id.is_empty() {
            return None;
        }
        Some(match kind {
            "album" => Target::Album(AlbumId::new(provider, id)),
            "playlist" => Target::Playlist(PlaylistId::new(provider, id)),
            "artist" => Target::Artist(ArtistId::new(provider, id)),
            _ => Target::Track(TrackId::new(provider, id)),
        })
    }
}

/// Item de grade na interface: playlist, album ou artista.
#[derive(Debug, Clone)]
pub struct Card {
    pub tag: String,
    pub title: String,
    pub subtitle: String,
    /// URL da capa no tamanho que o cartao desenha, ou vazio quando o item nao
    /// tem imagem. Quem resolve isso em arquivo e o cache -- ver
    /// [`crate::artwork`].
    pub cover: String,
    /// Arquivo da capa, quando o cache ja a tem. Preenchido depois, quando o
    /// download termina -- e por isso o cartao nunca reserva espaco que pula.
    pub cover_path: Option<std::path::PathBuf>,
}

/// As prateleiras do Inicio.
///
/// Cada uma e independente: a que falhar chega vazia e as outras aparecem do
/// mesmo jeito. Uma tela inicial que some inteira porque o historico nao
/// respondeu seria pior do que uma tela inicial menor.
#[derive(Debug, Default)]
pub struct Home {
    /// Geradas para esta conta: Daily Mix, Discover Weekly, Blend.
    pub made_for_you: Vec<Card>,
    /// Fluxo continuo em torno de uma semente: os Mix de tema e de artista.
    pub stations: Vec<Card>,
    /// Retrospectivas: Your Top Songs de cada ano, e a de todos os tempos.
    pub retrospectives: Vec<Card>,
    pub liked: Vec<Track>,
    /// Todos os ids curtidos, inclusive os que nao cabem na prateleira.
    pub liked_ids: Vec<TrackId>,
    /// Todas as playlists da conta, para a barra lateral.
    ///
    /// Nao e prateleira: o `rootlist` e a navegacao do usuario, e vive na
    /// lateral. Vem no mesmo pacote porque sai da mesma requisicao.
    pub playlists: Vec<Card>,
}

/// Uma lista aberta na tela de detalhe.
///
/// Comeca com uma pagina e cresce conforme a pessoa rola. Quando filtro ou
/// ordenacao exigem a colecao inteira, o estado pede as paginas restantes em
/// segundo plano antes de tratar o resultado como definitivo.
#[derive(Debug, Clone)]
pub struct Detail {
    /// O que abriu esta tela, para o botao de voltar e para a fila.
    pub origin: QueueOrigin,
    pub title: String,
    /// Dono e tamanho, ou artistas do album.
    pub subtitle: String,
    /// Que tipo de coisa e: "Playlist", "Album", "Artista".
    pub kind: String,
    pub cover: String,
    pub cover_path: Option<std::path::PathBuf>,
    pub tracks: Vec<Track>,
    /// Recursos relacionados, hoje a discografia na tela de artista.
    pub cards: Vec<Card>,
    /// Total informado pelo provedor, antes do teto local da tela.
    pub total_tracks: Option<u32>,
    /// Origem paginavel. Album e artista ja chegam completos e usam `None`.
    pub source: Option<Target>,
    /// Verdadeiro quando o provedor informou que existe outra pagina.
    pub has_more: bool,
}

/// O que um pedido produziu.
pub enum Outcome {
    Search {
        query: String,
        tracks: Vec<Track>,
        cards: Vec<Card>,
    },
    Home(Box<Home>),
    Library(Vec<Card>),
    /// Um album, playlist ou artista pronto para virar fila.
    ///
    /// Continua existindo para o que se toca sem abrir: uma faixa avulsa da
    /// busca, ou a lista inteira acionada pelo botao de tocar.
    Context {
        origin: QueueOrigin,
        title: String,
        tracks: Vec<Track>,
    },
    /// Uma lista aberta para ser lida antes de tocada.
    Detail(Box<Detail>),
    /// Continuacao de uma lista ja aberta. Separada de `Detail` para anexar as
    /// linhas sem perder filtro, ordenacao nem posicao de rolagem.
    DetailMore {
        source: Target,
        tracks: Vec<Track>,
        total: Option<u32>,
        has_more: bool,
    },
    DetailMoreFailed {
        source: Target,
        message: String,
    },
    Failed(String),
}

/// Resultado independente da navegacao para o fim natural da fila.
///
/// Nao usa [`Outcome`] nem o mesmo canal porque uma resposta de autoplay nao
/// pode cancelar uma busca que a pessoa acabou de iniciar.
pub enum AutoplayOutcome {
    Ready { seed: TrackId, tracks: Vec<Track> },
    Failed(String),
}

pub struct LibraryOutcome {
    pub id: TrackId,
    pub saved: bool,
    pub result: Result<(), String>,
}

/// Catalogo e biblioteca ligados a interface.
pub struct Browse {
    catalog: Arc<dyn Catalog>,
    library: Arc<dyn Library>,
    handle: tokio::runtime::Handle,
    pending: Option<Receiver<Outcome>>,
    autoplay_pending: Option<Receiver<AutoplayOutcome>>,
    library_tx: UnboundedSender<LibraryOutcome>,
    library_rx: UnboundedReceiver<LibraryOutcome>,
    artwork: Arc<dyn Artwork>,
    covers: ArtworkCache,
    /// Canal proprio das capas, separado de `pending`: um cartao continua
    /// utilizavel sem a capa, entao a chegada dela nao pode competir com o
    /// pedido da tela nem cancela-lo.
    art_tx: UnboundedSender<Ready>,
    art_rx: UnboundedReceiver<Ready>,
}

impl std::fmt::Debug for Browse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Browse")
            .field("provedor", &self.catalog.name())
            .field("aguardando", &self.pending.is_some())
            .finish()
    }
}

impl Browse {
    pub fn new(
        catalog: Arc<dyn Catalog>,
        library: Arc<dyn Library>,
        artwork: Arc<dyn Artwork>,
        covers_dir: std::path::PathBuf,
        handle: tokio::runtime::Handle,
    ) -> Self {
        let (art_tx, art_rx) = tokio::sync::mpsc::unbounded_channel();
        let (library_tx, library_rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            catalog,
            library,
            handle,
            pending: None,
            autoplay_pending: None,
            library_tx,
            library_rx,
            artwork,
            covers: ArtworkCache::new(covers_dir),
            art_tx,
            art_rx,
        }
    }

    /// Resolve a capa de cada cartao, pedindo o que ainda nao esta em disco.
    ///
    /// O que ja estiver em cache entra no primeiro quadro; o resto chega
    /// depois por [`Browse::poll_artwork`].
    /// Resolve uma capa avulsa, pedindo-a se ainda nao estiver em disco.
    ///
    /// Usado pela faixa tocando, que nao e um cartao de nenhuma tela.
    pub fn cover(&mut self, url: &str) -> Option<std::path::PathBuf> {
        if let Some(path) = self.covers.cached(url) {
            return Some(path);
        }
        self.covers
            .request(url, &self.artwork, &self.handle, self.art_tx.clone());
        None
    }

    pub fn resolve_covers(&mut self, cards: &mut [Card]) {
        for card in cards {
            if card.cover.is_empty() {
                continue;
            }
            match self.covers.cached(&card.cover) {
                Some(path) => card.cover_path = Some(path),
                None => {
                    self.covers.request(
                        &card.cover,
                        &self.artwork,
                        &self.handle,
                        self.art_tx.clone(),
                    );
                }
            }
        }
    }

    /// Recolhe as capas que terminaram de baixar.
    ///
    /// Devolve as prontas desde a ultima leitura. Vazio no caso comum, que e
    /// o que mantem barato rodar isto a cada 100 ms.
    pub fn poll_artwork(&mut self) -> Vec<Ready> {
        let mut prontas = Vec::new();
        while let Ok(ready) = self.art_rx.try_recv() {
            self.covers.settle(&ready);
            prontas.push(ready);
        }
        prontas
    }

    /// Esquece o pedido em andamento. Usado ao sair da conta.
    pub fn cancel(&mut self) {
        self.pending = None;
        self.autoplay_pending = None;
    }

    /// Pede uma continuacao para a fila sem competir com pedidos de tela.
    pub fn autoplay(&mut self, seed: TrackId) -> bool {
        if self.autoplay_pending.is_some() {
            return false;
        }
        let catalog = self.catalog.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let task_seed = seed.clone();
        self.handle.spawn(async move {
            let outcome = match catalog.radio(&task_seed, AUTOPLAY_TRACKS).await {
                Ok(page) => AutoplayOutcome::Ready {
                    seed: task_seed,
                    tracks: page.items,
                },
                Err(error) => AutoplayOutcome::Failed(describe(&error)),
            };
            let _ = tx.send(outcome);
        });
        self.autoplay_pending = Some(rx);
        true
    }

    pub fn poll_autoplay(&mut self) -> Option<AutoplayOutcome> {
        let rx = self.autoplay_pending.as_ref()?;
        match rx.try_recv() {
            Ok(outcome) => {
                self.autoplay_pending = None;
                Some(outcome)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.autoplay_pending = None;
                Some(AutoplayOutcome::Failed("o radio foi interrompido".into()))
            }
        }
    }

    /// Altera uma curtida sem competir com busca ou navegacao em andamento.
    pub fn set_track_saved(&self, id: TrackId, saved: bool) {
        let library = self.library.clone();
        let tx = self.library_tx.clone();
        self.handle.spawn(async move {
            let result = library
                .set_track_saved(&id, saved)
                .await
                .map_err(|error| describe(&error));
            let _ = tx.send(LibraryOutcome { id, saved, result });
        });
    }

    pub fn poll_library(&mut self) -> Option<LibraryOutcome> {
        self.library_rx.try_recv().ok()
    }

    pub fn search(&mut self, query: &str) {
        let catalog = self.catalog.clone();
        let query = query.trim().to_string();
        self.spawn(move |tx| async move {
            let outcome = match catalog.search(&query, SearchKind::All, SEARCH_LIMIT).await {
                Ok(results) => {
                    let mut cards = Vec::with_capacity(
                        results.albums.len() + results.artists.len() + results.playlists.len(),
                    );
                    cards.extend(results.albums.iter().map(album_card));
                    cards.extend(results.artists.iter().map(artist_card));
                    cards.extend(results.playlists.iter().map(playlist_card));
                    Outcome::Search {
                        query,
                        tracks: results.tracks,
                        cards,
                    }
                }
                Err(e) => Outcome::Failed(describe(&e)),
            };
            let _ = tx.send(outcome);
        });
    }

    /// As quatro prateleiras do Inicio, numa tarefa so.
    ///
    /// Sequencial e nao paralelo pelo mesmo motivo da biblioteca: sao quatro
    /// requisicoes numa tela que abre uma vez, e disparar as quatro juntas
    /// arrisca o limite de requisicoes do Spotify para ganhar milissegundos.
    /// Carrega o Inicio e a lista da barra lateral.
    ///
    /// Duas requisicoes, nao cinco: as playlists saem todas de um `rootlist`
    /// so, e a classificacao separa o que vai para cada prateleira. As que
    /// morreram com o Web API -- historico recente e artistas mais ouvidos --
    /// nao sao nem pedidas.
    pub fn load_home(&mut self) {
        let library = self.library.clone();
        self.spawn(move |tx| async move {
            let mut home = Home::default();
            let mut failure = None;

            match library.made_for_you(ROOTLIST_LIMIT).await {
                Ok(page) => {
                    for playlist in &page.items {
                        let card = playlist_card(playlist);
                        match playlist.kind {
                            PlaylistKind::MadeForYou => home.made_for_you.push(card),
                            PlaylistKind::Station => home.stations.push(card),
                            PlaylistKind::Retrospective => home.retrospectives.push(card),
                            // Vitrine nao entra em prateleira nenhuma: e igual
                            // para todo mundo, e o Inicio e sobre esta conta.
                            PlaylistKind::Editorial => {}
                            PlaylistKind::Personal => {}
                        }

                        // A lateral mostra o que o usuario navega: o que ele
                        // criou, seguiu, e as vitrines que ele escolheu seguir.
                        if matches!(
                            playlist.kind,
                            PlaylistKind::Personal | PlaylistKind::Editorial
                        ) {
                            home.playlists.push(playlist_card(playlist));
                        }
                    }
                }
                Err(e) => failure = note(failure, &e, "playlists"),
            }

            match library.saved_tracks(0, SHELF_TRACKS).await {
                Ok(page) => home.liked = page.items,
                Err(e) => failure = note(failure, &e, "musicas curtidas"),
            }
            match library.saved_track_ids().await {
                Ok(ids) => home.liked_ids = ids,
                Err(e) => failure = note(failure, &e, "estado das curtidas"),
            }

            let vazio = home.made_for_you.is_empty()
                && home.stations.is_empty()
                && home.retrospectives.is_empty()
                && home.liked.is_empty()
                && home.playlists.is_empty();

            let _ = tx.send(match failure {
                Some(message) if vazio => Outcome::Failed(message),
                _ => Outcome::Home(Box::new(home)),
            });
        });
    }
    /// Albuns salvos e artistas seguidos.
    ///
    /// As tres requisicoes sao sequenciais e nao paralelas: a biblioteca abre
    /// uma vez por sessao, e tres conexoes simultaneas para economizar
    /// milissegundos numa tela que ja apareceu nao pagam o risco de bater no
    /// limite de requisicoes do Spotify.
    pub fn load_library(&mut self) {
        let library = self.library.clone();
        self.spawn(move |tx| async move {
            let mut cards = Vec::new();
            let mut failure = None;

            match library.saved_albums(0, SECTION_LIMIT).await {
                Ok(page) => cards.extend(page.items.iter().map(album_card)),
                Err(e) => failure = note(failure, &e, "albuns salvos"),
            }
            match library.followed_artists(0, SECTION_LIMIT).await {
                Ok(page) => cards.extend(page.items.iter().map(artist_card)),
                Err(e) => failure = note(failure, &e, "artistas seguidos"),
            }

            // Uma secao que falhou nao apaga as que vieram: a tela mostra o que
            // deu certo e a barra de status conta o resto.
            let _ = tx.send(match failure {
                Some(message) if cards.is_empty() => Outcome::Failed(message),
                _ => Outcome::Library(cards),
            });
        });
    }

    /// Carrega o que foi ativado numa grade e devolve as faixas dele.
    pub fn open(&mut self, target: Target) {
        let catalog = self.catalog.clone();
        let library = self.library.clone();
        self.spawn(move |tx| async move {
            let _ = tx.send(match resolve(catalog, library, target).await {
                Ok(outcome) => outcome,
                Err(e) => Outcome::Failed(describe(&e)),
            });
        });
    }

    /// Pede a pagina seguinte da lista aberta sem permitir requisicoes
    /// duplicadas. O gesto de rolagem pode disparar varias vezes na mesma
    /// borda; somente a primeira deve chegar ao Spotify.
    pub fn load_more(&mut self, source: Target, offset: u32) -> bool {
        if self.pending.is_some() {
            return false;
        }
        let catalog = self.catalog.clone();
        let library = self.library.clone();
        self.spawn(move |tx| async move {
            let result = match &source {
                Target::Playlist(id) => catalog.playlist_tracks(id, offset, DETAIL_PAGE).await,
                Target::Liked => library.saved_tracks(offset, DETAIL_PAGE).await,
                _ => Err(CoreError::Unsupported("esta lista nao possui continuacao")),
            };
            let outcome = match result {
                Ok(page) => Outcome::DetailMore {
                    source,
                    has_more: !page.items.is_empty() && page.has_more(),
                    total: page.total,
                    tracks: page.items,
                },
                Err(error) => Outcome::DetailMoreFailed {
                    source,
                    message: describe(&error),
                },
            };
            let _ = tx.send(outcome);
        });
        true
    }

    /// Recolhe o resultado do pedido em andamento, se ja houver.
    pub fn poll(&mut self) -> Option<Outcome> {
        let rx = self.pending.as_ref()?;
        match rx.try_recv() {
            Ok(outcome) => {
                self.pending = None;
                Some(outcome)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.pending = None;
                Some(Outcome::Failed("a consulta foi interrompida".into()))
            }
        }
    }

    fn spawn<F, Fut>(&mut self, task: F)
    where
        F: FnOnce(std::sync::mpsc::Sender<Outcome>) -> Fut,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        self.handle.spawn(task(tx));
        self.pending = Some(rx);
    }
}

async fn resolve(
    catalog: Arc<dyn Catalog>,
    library: Arc<dyn Library>,
    target: Target,
) -> CoreResult<Outcome> {
    Ok(match target {
        // Uma faixa avulsa vira uma fila de uma faixa so. Quem ativou uma faixa
        // de uma lista nao passa por aqui: a lista inteira ja e o contexto.
        Target::Track(id) => {
            let track = catalog.track(&id).await?;
            let title = track.name.to_string();
            Outcome::Context {
                origin: QueueOrigin::Custom(title.clone()),
                title,
                tracks: vec![track],
            }
        }
        Target::Album(id) => {
            let album = catalog.album(&id).await?;
            Outcome::Detail(Box::new(Detail {
                origin: QueueOrigin::Album(album.id.canonical()),
                title: album.name.to_string(),
                subtitle: album
                    .artists
                    .iter()
                    .map(|a| a.name.as_ref())
                    .collect::<Vec<_>>()
                    .join(", "),
                kind: "Album".into(),
                cover: cover(&album.images),
                cover_path: None,
                total_tracks: album.total_tracks,
                source: None,
                has_more: false,
                tracks: album.tracks,
                cards: Vec::new(),
            }))
        }
        Target::Playlist(id) => {
            let playlist = catalog.playlist(&id).await?;
            let page = catalog.playlist_tracks(&id, 0, DETAIL_PAGE).await?;
            let total = page.total.or(playlist.total_tracks);
            let has_more = !page.items.is_empty() && page.has_more();
            let loaded = page.items.len() as u32;
            Outcome::Detail(Box::new(Detail {
                origin: QueueOrigin::Playlist(playlist.id.canonical()),
                title: playlist.name.to_string(),
                subtitle: match playlist.owner.as_deref() {
                    Some(dono) => {
                        format!("{dono} — {} faixas", total.unwrap_or(loaded))
                    }
                    None => format!("{} faixas", total.unwrap_or(loaded)),
                },
                kind: "Playlist".into(),
                cover: cover(&playlist.images),
                cover_path: None,
                tracks: page.items,
                total_tracks: total,
                source: Some(Target::Playlist(id)),
                has_more,
                cards: Vec::new(),
            }))
        }
        // Diferente das outras, esta nao vem do catalogo: curtidas sao da
        // conta, e quem responde por elas e a biblioteca.
        Target::Liked => {
            let page = library.saved_tracks(0, DETAIL_PAGE).await?;
            let total = page.total.unwrap_or(page.items.len() as u32);
            let has_more = !page.items.is_empty() && page.has_more();
            Outcome::Detail(Box::new(Detail {
                origin: QueueOrigin::Custom(LIKED_TITLE.into()),
                title: LIKED_TITLE.into(),
                subtitle: format!("{total} faixas"),
                kind: "Colecao".into(),
                cover: String::new(),
                cover_path: None,
                tracks: page.items,
                total_tracks: Some(total),
                source: Some(Target::Liked),
                has_more,
                cards: Vec::new(),
            }))
        }
        Target::Artist(id) => {
            let artist = catalog.artist(&id).await?;
            let cards = artist.albums.iter().map(album_ref_card).collect();
            Outcome::Detail(Box::new(Detail {
                origin: QueueOrigin::Artist(artist.id.canonical()),
                title: artist.name.to_string(),
                subtitle: String::new(),
                kind: "Artista".into(),
                cover: cover(&artist.images),
                cover_path: None,
                tracks: artist.top_tracks,
                total_tracks: None,
                source: None,
                has_more: false,
                cards,
            }))
        }
    })
}

/// URL da capa no tamanho que o cartao desenha.
///
/// `best_for_width` devolve a menor imagem que ainda serve. Baixar 640 px para
/// desenhar 160 seria dezesseis vezes mais bytes por cartao numa grade que
/// mostra dezenas deles -- e o criterio do projeto e nao disputar recurso com
/// quem esta jogando.
fn cover(images: &morune_core::model::ImageSet) -> String {
    images
        .best_for_width(CARD_ARTWORK_WIDTH)
        .map(|i| i.url.to_string())
        .unwrap_or_default()
}

fn playlist_card(p: &morune_core::model::Playlist) -> Card {
    Card {
        tag: Target::Playlist(p.id.clone()).tag(),
        title: p.name.to_string(),
        subtitle: match (p.owner.as_deref(), p.total_tracks) {
            (Some(owner), Some(total)) => format!("{owner} — {total} faixas"),
            (Some(owner), None) => owner.to_string(),
            (None, Some(total)) => format!("{total} faixas"),
            (None, None) => String::new(),
        },
        cover: cover(&p.images),
        cover_path: None,
    }
}

fn album_card(a: &morune_core::model::Album) -> Card {
    Card {
        tag: Target::Album(a.id.clone()).tag(),
        title: a.name.to_string(),
        subtitle: a
            .artists
            .iter()
            .map(|x| x.name.as_ref())
            .collect::<Vec<_>>()
            .join(", "),
        cover: cover(&a.images),
        cover_path: None,
    }
}

fn album_ref_card(a: &morune_core::model::AlbumRef) -> Card {
    Card {
        tag: Target::Album(a.id.clone()).tag(),
        title: a.name.to_string(),
        subtitle: "Album".into(),
        cover: cover(&a.images),
        cover_path: None,
    }
}

fn artist_card(a: &morune_core::model::Artist) -> Card {
    Card {
        tag: Target::Artist(a.id.clone()).tag(),
        title: a.name.to_string(),
        subtitle: a
            .genres
            .first()
            .map(|g| g.to_string())
            .unwrap_or_else(|| "Artista".into()),
        cover: cover(&a.images),
        cover_path: None,
    }
}

/// Largura em que a grade desenha uma capa.
///
/// Nao e a largura exata do cartao: e o piso que `best_for_width` usa para nao
/// escolher uma imagem borrada. Telas de alta densidade desenham maior, e a
/// diferenca nao se ve num quadrado pequeno.
const CARD_ARTWORK_WIDTH: u32 = 300;

/// Quantas playlists o rootlist entrega de uma vez.
///
/// Nao e teto de prateleira: e a lista inteira da conta, porque a barra
/// lateral mostra todas e o filtro precisa ver todas para filtrar.
const ROOTLIST_LIMIT: u32 = u32::MAX;

/// Uma estacao chega em blocos curtos: o proximo fim de fila pede outro bloco.
const AUTOPLAY_TRACKS: u32 = 50;

/// Guarda a primeira falha e registra as demais no log.
///
/// A tela tem uma linha de status, nao quatro. Mostrar a primeira falha e
/// honesto; empilhar as quatro so faz o usuario parar de ler.
fn note(previous: Option<String>, error: &CoreError, secao: &str) -> Option<String> {
    tracing::debug!(secao, error = %error, "prateleira do inicio nao carregou");

    // `Unsupported` nao e falha: e uma prateleira que o Spotify deixou de
    // expor por qualquer caminho que o Morune alcance. Mostrar erro por isso
    // faria a tela reclamar em toda abertura, para sempre, de algo que o
    // usuario nao pode resolver. A prateleira simplesmente nao aparece.
    if matches!(error, CoreError::Unsupported(_)) {
        return previous;
    }

    previous.or_else(|| Some(describe(error)))
}

/// Traduz um erro para uma frase que ajuda quem esta olhando a tela.
fn describe(error: &CoreError) -> String {
    match error {
        CoreError::NotAuthenticated | CoreError::AuthExpired => {
            "Entre na sua conta do Spotify para ver isto.".into()
        }
        CoreError::AccountPlan(message) => message.clone(),
        CoreError::Network(_) => "Sem conexao com o Spotify. Verifique a internet.".into(),
        CoreError::NotFound(_) => "Isso nao existe mais no Spotify.".into(),
        CoreError::Decode(_) => "O Spotify respondeu de um jeito que o Morune nao entendeu.".into(),
        other => format!("Nao foi possivel consultar o Spotify: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A forma textual e o unico contrato entre a interface e o Rust: a lista
    /// manda o que o `tag` produziu, e o clique volta com a mesma string. Se as
    /// duas pontas divergirem, o clique nao acha a faixa e nada acontece -- que
    /// e exatamente o defeito que a tela de detalhe teve: comparar o texto
    /// recebido com o id cru nunca casava.
    #[test]
    fn what_the_list_sends_is_what_parse_understands() {
        let id = TrackId::spotify("2JiDi0qAXsPwhPqA2qaKGt");
        let enviado = Target::Track(id.clone()).tag();

        assert_eq!(enviado, "track/spotify:2JiDi0qAXsPwhPqA2qaKGt");
        assert_eq!(Target::parse(&enviado), Some(Target::Track(id)));
    }

    #[test]
    fn every_target_survives_the_round_trip() {
        // Cada alvo da tela passa pelo mesmo caminho. Um que nao volte inteiro
        // vira um clique que nao faz nada, sem erro nenhum na tela.
        for alvo in [
            Target::Track(TrackId::spotify("abc")),
            Target::Album(AlbumId::spotify("def")),
            Target::Playlist(PlaylistId::spotify("ghi")),
            Target::Artist(ArtistId::spotify("jkl")),
            Target::Liked,
        ] {
            assert_eq!(Target::parse(&alvo.tag()), Some(alvo));
        }
    }

    use std::sync::Arc;
    use std::time::Duration;

    use morune_core::model::{AlbumRef, ArtistRef, ImageSet};

    #[test]
    fn a_tag_survives_the_round_trip_through_the_interface() {
        for target in [
            Target::Track(TrackId::spotify("4cOdK2wGLETKBW3PvgPWqT")),
            Target::Album(AlbumId::spotify("6eUW0wxWtzkFdaEFsTJto6")),
            Target::Playlist(PlaylistId::spotify("37i9dQZF1DXcBWIGoYBM5M")),
            Target::Artist(ArtistId::spotify("0gxyHStUsqpMadRV0Di1Qt")),
            Target::Track(TrackId::local("D:/Musica/faixa.flac")),
        ] {
            assert_eq!(Target::parse(&target.tag()), Some(target));
        }
    }

    #[test]
    fn an_id_without_a_kind_is_read_as_a_track() {
        // E a forma que a fila usava antes de haver catalogo.
        assert_eq!(
            Target::parse("spotify:4cOdK2wGLETKBW3PvgPWqT"),
            Some(Target::Track(TrackId::spotify("4cOdK2wGLETKBW3PvgPWqT")))
        );
    }

    #[test]
    fn a_tag_the_interface_never_produced_is_refused() {
        // Um clique nao pode virar `unwrap` em nada: id vazio, provedor
        // desconhecido e texto solto acontecem com cache antigo.
        assert_eq!(Target::parse(""), None);
        assert_eq!(Target::parse("album/"), None);
        assert_eq!(Target::parse("album/spotify:"), None);
        assert_eq!(Target::parse("album/desconhecido:musica.flac"), None);
    }

    fn playlist(
        name: &str,
        owner: Option<&str>,
        total: Option<u32>,
    ) -> morune_core::model::Playlist {
        morune_core::model::Playlist {
            id: PlaylistId::spotify("37i9"),
            kind: PlaylistKind::default(),
            name: name.into(),
            owner: owner.map(Arc::from),
            description: None,
            images: ImageSet::default(),
            total_tracks: total,
            tracks: Vec::new(),
        }
    }

    #[test]
    fn a_playlist_card_says_who_made_it_and_how_big_it_is() {
        let card = playlist_card(&playlist("Descobertas", Some("Spotify"), Some(30)));
        assert_eq!(card.title, "Descobertas");
        assert_eq!(card.subtitle, "Spotify — 30 faixas");
        assert_eq!(card.tag, "playlist/spotify:37i9");
    }

    #[test]
    fn a_card_without_owner_or_total_still_has_a_usable_subtitle() {
        // Playlist colaborativa as vezes chega sem dono; a grade nao pode
        // mostrar um travessao solto por causa disso.
        assert_eq!(playlist_card(&playlist("x", None, None)).subtitle, "");
        assert_eq!(
            playlist_card(&playlist("x", None, Some(4))).subtitle,
            "4 faixas"
        );
        assert_eq!(
            playlist_card(&playlist("x", Some("Felipe"), None)).subtitle,
            "Felipe"
        );
    }

    #[test]
    fn an_album_card_credits_every_artist() {
        let album = morune_core::model::Album {
            id: AlbumId::spotify("6eUW"),
            name: "Colaboracao".into(),
            artists: vec![
                ArtistRef {
                    id: ArtistId::spotify("a"),
                    name: "Um".into(),
                },
                ArtistRef {
                    id: ArtistId::spotify("b"),
                    name: "Outro".into(),
                },
            ],
            images: ImageSet::default(),
            release_date: None,
            total_tracks: Some(1),
            tracks: Vec::new(),
        };
        assert_eq!(album_card(&album).subtitle, "Um, Outro");
    }

    #[test]
    fn an_artist_card_never_shows_an_empty_line() {
        let mut artist = morune_core::model::Artist {
            id: ArtistId::spotify("0gxy"),
            name: "Rick Astley".into(),
            images: ImageSet::default(),
            genres: Vec::new(),
            top_tracks: Vec::new(),
            albums: Vec::new(),
        };
        assert_eq!(artist_card(&artist).subtitle, "Artista");

        artist.genres = vec!["new wave".into()];
        assert_eq!(artist_card(&artist).subtitle, "new wave");
    }

    #[test]
    fn only_the_first_failure_reaches_the_status_line() {
        // Quatro prateleiras podem falhar juntas quando a rede cai. A tela tem
        // uma linha, e a primeira mensagem ja diz o que houve.
        let primeira = note(None, &CoreError::Network("timeout".into()), "historico");
        let depois = note(
            primeira.clone(),
            &CoreError::NotFound("x".into()),
            "curtidas",
        );
        assert_eq!(depois, primeira);
        assert!(depois.unwrap().contains("internet"));
    }

    #[test]
    fn a_home_with_nothing_in_it_is_detectable() {
        let home = Home::default();
        assert!(home.made_for_you.is_empty() && home.stations.is_empty());
        assert!(home.retrospectives.is_empty() && home.liked.is_empty());
        assert!(home.playlists.is_empty());
    }

    #[test]
    fn error_messages_say_what_to_do_next() {
        assert!(describe(&CoreError::NotAuthenticated).contains("Entre na sua conta"));
        assert!(describe(&CoreError::Network("timeout".into())).contains("internet"));
        assert!(describe(&CoreError::NotFound("faixa".into())).contains("nao existe"));
    }

    // Mantem o modulo honesto quanto ao tipo que ele espera do catalogo: se o
    // modelo mudar, isto para de compilar antes de a tela quebrar.
    #[test]
    fn tracks_carry_what_a_card_needs() {
        let track = Track {
            id: TrackId::spotify("t"),
            name: "faixa".into(),
            artists: Vec::new(),
            album: Some(AlbumRef {
                id: AlbumId::spotify("a"),
                name: "album".into(),
                images: ImageSet::default(),
            }),
            duration: Duration::from_secs(1),
            track_number: None,
            disc_number: None,
            explicit: false,
            playable: true,
        };
        assert_eq!(Target::Track(track.id.clone()).tag(), "track/spotify:t");
    }
}
