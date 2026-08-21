//! Motor de reproducao sobre a librespot.
//!
//! A forma deste modulo vem de uma exigencia so, e ela e do produto: o Morune
//! toca musica **enquanto o usuario joga**. Nada aqui pode disputar tempo com o
//! jogo nem segurar a interface.
//!
//! Por isso:
//!
//! - a librespot vive num runtime tokio proprio, numa thread separada. A thread
//!   da interface nunca espera rede, disco ou dispositivo de audio;
//! - comandos entram por canal e nao devolvem resultado. Quem quer saber o que
//!   aconteceu assina [`PlaybackEngine::subscribe`];
//! - [`PlaybackEngine::snapshot`] le um retrato pronto sob mutex, sem tocar na
//!   librespot. A interface chama isso a cada quadro;
//! - a posicao e **calculada**, nao consultada. A librespot avisa a posicao em
//!   transicoes; entre elas, o relogio local interpola. Perguntar a posicao 60
//!   vezes por segundo seria trabalho recorrente para exibir um numero que anda
//!   sozinho.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use librespot_core::SpotifyUri;
use librespot_playback::config::PlayerConfig;
use librespot_playback::mixer::NoOpVolume;
use librespot_playback::player::{Player, PlayerEvent as LibrespotEvent};
use morune_core::model::{Track, TrackId};
use morune_core::playback::{
    EngineCapabilities, PlaybackEngine, PlaybackState, PlayerCommand, PlayerEvent, PlayerSnapshot,
};

use morune_core::{CoreError, CoreResult};
use tokio::sync::{broadcast, mpsc};

use crate::auth::SharedSession;

/// Capacidade do canal de eventos para a interface.
///
/// Eventos sao pequenos e a interface consome rapido; 64 cobre uma rajada de
/// troca de faixa sem que um assinante lento perca transicao de estado.
const EVENT_CAPACITY: usize = 64;

/// Volume compartilhado entre a interface e o misturador da librespot.
///
/// Guardado como inteiro atomico porque o misturador le isto em cada bloco de
/// audio, numa thread de tempo real: um mutex ali dentro seria um risco de
/// engasgo por prioridade invertida, que e exatamente o defeito que este
/// aplicativo promete nao ter.
#[derive(Debug, Default)]
pub(crate) struct SharedVolume(AtomicU64);

impl SharedVolume {
    /// Escala usada para guardar `[0.0, 1.0]` em inteiro sem perder resolucao
    /// audivel.
    const SCALE: f64 = 10_000.0;

    pub(crate) fn set(&self, linear: f32) {
        let clamped = linear.clamp(0.0, 1.0) as f64;
        self.0
            .store((clamped * Self::SCALE) as u64, Ordering::Relaxed);
    }

    pub(crate) fn linear(&self) -> f32 {
        (self.0.load(Ordering::Relaxed) as f64 / Self::SCALE) as f32
    }

    /// Fator a multiplicar pelo audio.
    ///
    /// Curva cubica: a percepcao de volume nao e linear, e um controle linear
    /// parece "tudo ou nada" nos primeiros 20% do cursor.
    ///
    /// Quem aplica isto e o nosso sink, no misturador do rodio, e nao o
    /// `volume_getter` da librespot -- ver [`crate::sink`]. A librespot recebe
    /// `NoOpVolume` justamente para o audio nao ser atenuado duas vezes.
    pub(crate) fn attenuation(&self) -> f64 {
        let linear = self.linear() as f64;
        linear * linear * linear
    }
}

/// Relogio de posicao.
///
/// Guarda a ultima posicao conhecida e o instante em que ela valia. Entre
/// avisos da librespot, a posicao atual e uma soma -- sem I/O, sem trava, sem
/// perguntar nada a ninguem.
#[derive(Debug, Clone, Copy)]
struct PositionClock {
    anchor: Duration,
    since: Instant,
    running: bool,
}

impl Default for PositionClock {
    fn default() -> Self {
        Self {
            anchor: Duration::ZERO,
            since: Instant::now(),
            running: false,
        }
    }
}

impl PositionClock {
    fn set(&mut self, position: Duration, running: bool) {
        self.anchor = position;
        self.since = Instant::now();
        self.running = running;
    }

    fn pause(&mut self) {
        self.anchor = self.now();
        self.since = Instant::now();
        self.running = false;
    }

    fn now(&self) -> Duration {
        if self.running {
            self.anchor + self.since.elapsed()
        } else {
            self.anchor
        }
    }
}

/// Estado compartilhado entre a task da librespot e a interface.
#[derive(Debug)]
struct Shared {
    snapshot: Mutex<PlayerSnapshot>,
    clock: Mutex<PositionClock>,
    volume: Arc<SharedVolume>,
    /// Levantado quando o audio ja enfileirado deixou de valer.
    ///
    /// So o motor sabe distinguir pausar (a fila continua valendo) de trocar de
    /// faixa, parar e mover a posicao (a fila virou lixo). O sink ve `stop()`
    /// nos quatro casos, entao a diferenca chega por aqui.
    flush: crate::sink::FlushRequest,
}

impl Shared {
    /// Aplica uma mudanca ao retrato e devolve o retrato atualizado.
    fn update(&self, f: impl FnOnce(&mut PlayerSnapshot)) {
        let mut snapshot = self.snapshot.lock().unwrap();
        f(&mut snapshot);
    }

    /// Marca o audio ja enfileirado como invalido.
    ///
    /// O sink atende antes de deixar sair o proximo audio -- em `start` ou na
    /// proxima escrita, o que vier primeiro.
    fn request_flush(&self) {
        self.flush.store(true, Ordering::Release);
    }
}

/// Motor de reproducao do Spotify.
pub struct SpotifyEngine {
    commands: mpsc::UnboundedSender<PlayerCommand>,
    events: broadcast::Sender<PlayerEvent>,
    shared: Arc<Shared>,
}

impl std::fmt::Debug for SpotifyEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpotifyEngine")
            .field("snapshot", &self.snapshot())
            .finish()
    }
}

impl SpotifyEngine {
    /// Cria o motor sobre uma sessao ja autenticada.
    ///
    /// `handle` e o runtime onde a librespot vai viver -- o mesmo que executou
    /// o login, para nao haver dois runtimes disputando as mesmas conexoes.
    pub fn new(session: SharedSession, handle: tokio::runtime::Handle) -> CoreResult<Self> {
        let live = session.get().ok_or(CoreError::NotAuthenticated)?;

        let volume = Arc::new(SharedVolume::default());
        volume.set(1.0);
        let flush: crate::sink::FlushRequest = Arc::new(AtomicBool::new(false));

        // A saida de audio e nossa, e nao a `RodioSink` da librespot: a dela
        // faz todo comando esperar o buffer de meio segundo. O motivo completo
        // esta em [`crate::sink`]. Abrir aqui, e nao dentro do construtor do
        // `Player`, e o que permite reportar falha de dispositivo como falha de
        // dispositivo -- a librespot chamaria `unwrap` e derrubaria o processo.
        let sink =
            crate::sink::open(volume.clone(), flush.clone()).map_err(CoreError::AudioDevice)?;

        let player = Player::new(
            PlayerConfig::default(),
            live,
            // O volume e aplicado pelo nosso sink, depois da fila. Atenuar aqui
            // tambem faria o audio passar pela curva duas vezes.
            Box::new(NoOpVolume),
            move || Box::new(sink),
        );

        let shared = Arc::new(Shared {
            snapshot: Mutex::new(PlayerSnapshot::default()),
            clock: Mutex::new(PositionClock::default()),
            volume: volume.clone(),
            flush,
        });
        let (events, _) = broadcast::channel(EVENT_CAPACITY);
        let (commands, command_rx) = mpsc::unbounded_channel();

        handle.spawn(run(player, command_rx, events.clone(), shared.clone()));

        Ok(Self {
            commands,
            events,
            shared,
        })
    }
}

impl PlaybackEngine for SpotifyEngine {
    fn name(&self) -> &'static str {
        "spotify"
    }

    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities {
            seek: true,
            volume: true,
            // A librespot pre-carrega a proxima faixa, mas quem decide qual e a
            // proxima e a fila do Morune, e ela so avisa no fim da atual. Sem
            // isso nao ha reproducao sem intervalo de verdade, e prometer o que
            // nao se cumpre e pior do que nao oferecer.
            gapless: false,
            precise_position: true,
        }
    }

    fn send(&self, command: PlayerCommand) -> CoreResult<()> {
        self.commands
            .send(command)
            .map_err(|_| CoreError::InvalidState("motor de reproducao encerrado".into()))
    }

    fn snapshot(&self) -> PlayerSnapshot {
        let mut snapshot = self.shared.snapshot.lock().unwrap().clone();
        snapshot.position = self.shared.clock.lock().unwrap().now();
        // A posicao interpolada nao pode passar da duracao: no fim da faixa o
        // aviso da librespot chega alguns milissegundos depois do relogio.
        if !snapshot.duration.is_zero() && snapshot.position > snapshot.duration {
            snapshot.position = snapshot.duration;
        }
        snapshot.volume = self.shared.volume.linear();
        snapshot
    }

    fn subscribe(&self) -> broadcast::Receiver<PlayerEvent> {
        self.events.subscribe()
    }
}

/// Converte um id do modelo para o URI que a librespot entende.
fn to_spotify_uri(id: &TrackId) -> CoreResult<SpotifyUri> {
    SpotifyUri::from_uri(&format!("spotify:track:{}", id.id))
        .map_err(|e| CoreError::NotFound(format!("faixa {id}: {e}")))
}

/// Laco do motor: recebe comandos da interface e eventos da librespot.
async fn run(
    player: Arc<Player>,
    mut commands: mpsc::UnboundedReceiver<PlayerCommand>,
    events: broadcast::Sender<PlayerEvent>,
    shared: Arc<Shared>,
) {
    let mut librespot_events = player.get_player_event_channel();

    loop {
        tokio::select! {
            command = commands.recv() => {
                let Some(command) = command else { break };
                if !apply(&player, command, &events, &shared) {
                    break;
                }
            }
            event = librespot_events.recv() => {
                let Some(event) = event else { break };
                translate(event, &events, &shared);
            }
        }
    }

    player.stop();
    tracing::debug!("motor de reproducao encerrado");
}

/// Executa um comando. Devolve `false` quando o motor deve encerrar.
fn apply(
    player: &Arc<Player>,
    command: PlayerCommand,
    events: &broadcast::Sender<PlayerEvent>,
    shared: &Arc<Shared>,
) -> bool {
    match command {
        PlayerCommand::Load {
            track,
            start_paused,
        } => load(player, &track, start_paused, events, shared),
        // Adiantar nao mexe no retrato nem pede descarte: a faixa que esta
        // tocando continua sendo a que toca. A librespot ignora o pedido se ja
        // tiver essa faixa pronta ou em carregamento, entao repetir e barato.
        PlayerCommand::Preload(track) => match to_spotify_uri(&track.id) {
            Ok(uri) => player.preload(uri),
            // Falha aqui nao vira erro na tela: ninguem pediu esta faixa ainda,
            // e ela sera reportada de verdade se e quando for tocada.
            Err(e) => tracing::debug!(error = %e, "faixa nao pode ser adiantada"),
        },
        PlayerCommand::Play => player.play(),
        PlayerCommand::Pause => player.pause(),
        PlayerCommand::TogglePlay => {
            let playing = shared.snapshot.lock().unwrap().state == PlaybackState::Playing;
            if playing {
                player.pause()
            } else {
                player.play()
            }
        }
        PlayerCommand::Stop => {
            shared.request_flush();
            player.stop()
        }
        // Avancar e voltar sao decisao da fila, nao do motor: quem manda o
        // proximo `Load` e a aplicacao. Aceitar em silencio evita que a
        // interface trate erro para um botao que ela mesma ja resolveu.
        PlayerCommand::Next | PlayerCommand::Previous => {}
        PlayerCommand::Seek(position) => {
            // O audio ja enfileirado e do trecho antigo. Sem descartar, ele
            // tocaria inteiro antes de o novo comecar -- a librespot nao mexe na
            // fila ao mover a posicao.
            shared.request_flush();
            player.seek(position.as_millis() as u32);
            shared.clock.lock().unwrap().set(position, true);
        }
        PlayerCommand::SetVolume(level) => {
            // Nada mais a fazer: o sink le este valor a cada bloco e o
            // misturador o reaplica ao audio ja enfileirado.
            shared.volume.set(level);
            let _ = events.send(PlayerEvent::VolumeChanged(shared.volume.linear()));
        }
        // Shuffle e repeticao vivem na fila do Morune, nao na librespot. O
        // motor guarda o valor so para o retrato ficar coerente.
        PlayerCommand::SetShuffle(on) => shared.update(|s| s.shuffle = on),
        PlayerCommand::SetRepeat(mode) => shared.update(|s| s.repeat = mode),
        PlayerCommand::Shutdown => return false,
    }
    true
}

fn load(
    player: &Arc<Player>,
    track: &Track,
    start_paused: bool,
    events: &broadcast::Sender<PlayerEvent>,
    shared: &Arc<Shared>,
) {
    let uri = match to_spotify_uri(&track.id) {
        Ok(uri) => uri,
        Err(e) => {
            let _ = events.send(PlayerEvent::Error(e.to_string()));
            return;
        }
    };

    // A duracao vem do modelo, e nao de uma consulta de metadados: quem pediu
    // para tocar ja tinha a faixa em maos, e uma requisicao a mais aqui atrasa
    // o inicio do som sem acrescentar nada.
    // A faixa anterior ainda tem audio enfileirado, e ele nao pode ser ouvido
    // depois da troca. A librespot pausa o sink aqui, mas nao esvazia a fila.
    shared.request_flush();

    shared.update(|s| {
        s.track = Some(Arc::new(track.clone()));
        s.duration = track.duration;
        s.state = PlaybackState::Loading;
        s.buffering = true;
    });
    shared.clock.lock().unwrap().set(Duration::ZERO, false);

    let _ = events.send(PlayerEvent::TrackChanged(Some(track.id.clone())));
    let _ = events.send(PlayerEvent::StateChanged(PlaybackState::Loading));
    let _ = events.send(PlayerEvent::Buffering(true));

    player.load(uri, !start_paused, 0);
}

/// Traduz um evento da librespot para o vocabulario do core.
fn translate(event: LibrespotEvent, events: &broadcast::Sender<PlayerEvent>, shared: &Arc<Shared>) {
    match event {
        LibrespotEvent::Playing { position_ms, .. } => {
            let position = Duration::from_millis(position_ms as u64);
            shared.clock.lock().unwrap().set(position, true);
            shared.update(|s| {
                s.state = PlaybackState::Playing;
                s.buffering = false;
            });
            let _ = events.send(PlayerEvent::Buffering(false));
            let _ = events.send(PlayerEvent::StateChanged(PlaybackState::Playing));
        }
        LibrespotEvent::Paused { position_ms, .. } => {
            let position = Duration::from_millis(position_ms as u64);
            shared.clock.lock().unwrap().set(position, false);
            shared.update(|s| {
                s.state = PlaybackState::Paused;
                s.buffering = false;
            });
            let _ = events.send(PlayerEvent::StateChanged(PlaybackState::Paused));
        }
        LibrespotEvent::Loading { position_ms, .. } => {
            shared
                .clock
                .lock()
                .unwrap()
                .set(Duration::from_millis(position_ms as u64), false);
            shared.update(|s| {
                s.state = PlaybackState::Loading;
                s.buffering = true;
            });
            let _ = events.send(PlayerEvent::Buffering(true));
            let _ = events.send(PlayerEvent::StateChanged(PlaybackState::Loading));
        }
        LibrespotEvent::Stopped { .. } => {
            shared.clock.lock().unwrap().pause();
            shared.update(|s| {
                s.state = PlaybackState::Stopped;
                s.buffering = false;
            });
            let _ = events.send(PlayerEvent::StateChanged(PlaybackState::Stopped));
        }
        LibrespotEvent::EndOfTrack { .. } => {
            // Quem decide a proxima faixa e a fila. O motor so avisa que esta
            // acabou -- e `Queue::next(false)` sabe que o avanco foi automatico,
            // que e o que faz "repetir uma" funcionar.
            let ended = shared
                .snapshot
                .lock()
                .unwrap()
                .track
                .as_ref()
                .map(|t| t.id.clone());
            shared.clock.lock().unwrap().pause();
            if let Some(id) = ended {
                let _ = events.send(PlayerEvent::EndOfTrack(id));
            }
        }
        LibrespotEvent::Unavailable { track_id, .. } => {
            let _ = events.send(PlayerEvent::Error(format!(
                "faixa indisponivel na sua regiao: {}",
                track_id.to_uri().unwrap_or_else(|_| "desconhecida".into())
            )));
            shared.update(|s| s.state = PlaybackState::Stopped);
            let _ = events.send(PlayerEvent::StateChanged(PlaybackState::Stopped));
        }
        LibrespotEvent::PositionCorrection { position_ms, .. }
        | LibrespotEvent::Seeked { position_ms, .. } => {
            let position = Duration::from_millis(position_ms as u64);
            let running = shared.snapshot.lock().unwrap().state == PlaybackState::Playing;
            shared.clock.lock().unwrap().set(position, running);
            let duration = shared.snapshot.lock().unwrap().duration;
            let _ = events.send(PlayerEvent::Position { position, duration });
        }
        // Os demais eventos existem para o Spotify Connect, que o Morune ainda
        // nao expoe. Ignorar em silencio e correto: nao sao erro.
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use morune_core::queue::RepeatMode;

    #[test]
    fn volume_curve_is_monotonic_and_bounded() {
        let v = Arc::new(SharedVolume::default());

        v.set(0.0);
        assert_eq!(v.attenuation(), 0.0);
        v.set(1.0);
        assert_eq!(v.attenuation(), 1.0);

        v.set(0.25);
        let quarter = v.attenuation();
        v.set(0.5);
        let half = v.attenuation();
        assert!(quarter < half, "curva de volume nao e crescente");
        // O ponto da curva cubica: meio cursor nao e meio volume.
        assert!(half < 0.5, "curva linear demais: {half}");
    }

    #[test]
    fn volume_is_clamped_outside_the_valid_range() {
        let v = Arc::new(SharedVolume::default());
        v.set(-1.0);
        assert_eq!(v.linear(), 0.0);
        v.set(7.0);
        assert_eq!(v.linear(), 1.0);
    }

    #[test]
    fn clock_advances_only_while_running() {
        let mut clock = PositionClock::default();
        clock.set(Duration::from_secs(10), false);
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(clock.now(), Duration::from_secs(10), "avancou parado");

        clock.set(Duration::from_secs(10), true);
        std::thread::sleep(Duration::from_millis(20));
        assert!(clock.now() > Duration::from_secs(10), "nao avancou tocando");
    }

    #[test]
    fn pausing_the_clock_keeps_the_elapsed_position() {
        let mut clock = PositionClock::default();
        clock.set(Duration::from_secs(5), true);
        std::thread::sleep(Duration::from_millis(30));
        clock.pause();

        let paused_at = clock.now();
        assert!(paused_at > Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(clock.now(), paused_at, "continuou andando depois da pausa");
    }

    #[test]
    fn track_ids_become_spotify_uris() {
        let id = TrackId::spotify("4cOdK2wGLETKBW3PvgPWqT");
        let uri = to_spotify_uri(&id).expect("id valido");
        assert_eq!(
            uri.to_uri().unwrap(),
            "spotify:track:4cOdK2wGLETKBW3PvgPWqT"
        );
    }

    #[test]
    fn malformed_track_id_is_reported_not_panicked() {
        // Um id vindo de cache corrompido nao pode derrubar o aplicativo.
        let id = TrackId::spotify("nao-e-base62-valido-!!!");
        assert!(matches!(to_spotify_uri(&id), Err(CoreError::NotFound(_))));
    }

    #[test]
    fn repeat_and_shuffle_live_in_the_snapshot() {
        let shared = Arc::new(Shared {
            snapshot: Mutex::new(PlayerSnapshot::default()),
            clock: Mutex::new(PositionClock::default()),
            volume: Arc::new(SharedVolume::default()),
            flush: Arc::new(AtomicBool::new(false)),
        });

        shared.update(|s| s.repeat = RepeatMode::One);
        shared.update(|s| s.shuffle = true);

        let snapshot = shared.snapshot.lock().unwrap();
        assert_eq!(snapshot.repeat, RepeatMode::One);
        assert!(snapshot.shuffle);
    }
}
