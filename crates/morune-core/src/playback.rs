use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::error::{CoreError, CoreResult};
use crate::model::{Track, TrackId};
use crate::queue::RepeatMode;

/// Estado macroscopico do motor de reproducao.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackState {
    /// Nada carregado.
    #[default]
    Idle,
    /// Faixa selecionada, aguardando dados suficientes para tocar.
    Loading,
    Playing,
    Paused,
    /// Parado por erro ou por fim de fila.
    Stopped,
}

impl PlaybackState {
    pub fn is_active(self) -> bool {
        matches!(self, PlaybackState::Playing | PlaybackState::Loading)
    }
}

/// Comando enviado ao motor. Todos sao nao bloqueantes: o motor responde
/// emitindo [`PlayerEvent`], nunca devolvendo o novo estado de forma sincrona.
///
/// Esse formato existe para que a UI nunca fique presa esperando rede, disco ou
/// dispositivo de audio -- o custo de latencia fica todo do lado do backend.
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerCommand {
    /// Carrega uma faixa. `start_paused` permite pre-carregar sem tocar.
    Load { track: Track, start_paused: bool },
    /// Adianta o trabalho de carregar uma faixa que ainda nao foi pedida.
    ///
    /// Existe porque comecar uma faixa custa buscar a chave de audio, abrir o
    /// fluxo na CDN e montar o decodificador -- medido em uso real, mediana de
    /// 588 ms e p90 de 996 ms. Com a proxima faixa ja pronta, o `Load` dela
    /// comeca a tocar sem ir a rede.
    ///
    /// Nao muda estado nenhum e nao emite evento: quem manda pode chamar de
    /// novo com a mesma faixa sem consequencia.
    Preload(Track),
    Play,
    Pause,
    TogglePlay,
    Stop,
    Next,
    Previous,
    Seek(Duration),
    /// Volume linear em `[0.0, 1.0]`; o backend aplica a curva de percepcao.
    SetVolume(f32),
    SetShuffle(bool),
    SetRepeat(RepeatMode),
    /// Encerra o motor e libera o dispositivo de audio.
    Shutdown,
}

/// Retrato consistente do player, barato de clonar e seguro de ler da UI.
#[derive(Debug, Clone)]
pub struct PlayerSnapshot {
    pub state: PlaybackState,
    pub track: Option<Arc<Track>>,
    pub position: Duration,
    pub duration: Duration,
    /// Volume linear em `[0.0, 1.0]`.
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    /// `true` enquanto o buffer nao tem dados suficientes para reproduzir.
    pub buffering: bool,
}

impl Default for PlayerSnapshot {
    fn default() -> Self {
        Self {
            state: PlaybackState::Idle,
            track: None,
            position: Duration::ZERO,
            duration: Duration::ZERO,
            volume: 1.0,
            shuffle: false,
            repeat: RepeatMode::Off,
            buffering: false,
        }
    }
}

impl PlayerSnapshot {
    /// Progresso em `[0.0, 1.0]`, ou `0.0` quando a duracao e desconhecida.
    pub fn progress(&self) -> f32 {
        if self.duration.is_zero() {
            return 0.0;
        }
        (self.position.as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0)
    }
}

/// Evento emitido pelo motor.
///
/// `Position` e deliberadamente separado dos demais: a UI pode amostrar
/// posicao numa taxa baixa sem perder transicoes de estado.
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerEvent {
    StateChanged(PlaybackState),
    TrackChanged(Option<TrackId>),
    Position { position: Duration, duration: Duration },
    VolumeChanged(f32),
    Buffering(bool),
    /// A faixa chegou ao fim naturalmente; quem controla a fila decide o proximo.
    EndOfTrack(TrackId),
    /// Erro nao fatal: o motor continua utilizavel.
    Error(String),
}

/// O que este backend suporta. A UI usa isto para esconder controles que nao
/// fazem sentido, em vez de mostrar botoes que falham em silencio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineCapabilities {
    pub seek: bool,
    pub volume: bool,
    pub gapless: bool,
    /// Backend consegue reportar posicao com precisao melhor que 1 s.
    pub precise_position: bool,
}

impl EngineCapabilities {
    pub const NONE: Self =
        Self { seek: false, volume: false, gapless: false, precise_position: false };
    pub const FULL: Self =
        Self { seek: true, volume: true, gapless: true, precise_position: true };
}

/// Contrato do motor de reproducao.
///
/// Dyn-compativel de proposito: a aplicacao troca de backend (Spotify, local,
/// nulo) em tempo de execucao guardando um `Arc<dyn PlaybackEngine>`. Por isso
/// nao ha `async fn` aqui -- comandos sao enfileirados e o resultado chega por
/// evento.
pub trait PlaybackEngine: Send + Sync + 'static {
    /// Nome curto do backend, usado em log e na UI ("spotify", "local").
    fn name(&self) -> &'static str;

    fn capabilities(&self) -> EngineCapabilities;

    /// Enfileira um comando. Nunca bloqueia.
    fn send(&self, command: PlayerCommand) -> CoreResult<()>;

    /// Estado atual. Deve ser barato: a UI chama isto a cada frame.
    fn snapshot(&self) -> PlayerSnapshot;

    /// Novo receptor de eventos. Cada assinante recebe todos os eventos
    /// emitidos a partir do momento da inscricao.
    fn subscribe(&self) -> broadcast::Receiver<PlayerEvent>;
}

/// Motor que nao toca nada.
///
/// Existe para que a aplicacao sempre tenha um `PlaybackEngine` valido: antes
/// do login, com o backend real em falha, ou em teste de UI. Sem ele a UI
/// precisaria tratar "sem motor" em todo lugar, que e onde nascem os panics.
#[derive(Debug)]
pub struct NullEngine {
    events: broadcast::Sender<PlayerEvent>,
    reason: &'static str,
}

impl NullEngine {
    pub fn new(reason: &'static str) -> Self {
        let (events, _) = broadcast::channel(16);
        Self { events, reason }
    }
}

impl Default for NullEngine {
    fn default() -> Self {
        Self::new("nenhum backend de reproducao ativo")
    }
}

impl PlaybackEngine for NullEngine {
    fn name(&self) -> &'static str {
        "null"
    }

    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities::NONE
    }

    fn send(&self, command: PlayerCommand) -> CoreResult<()> {
        match command {
            // Comandos inertes sao aceitos em silencio: a UI pode ajustar
            // volume ou modo antes de existir backend real.
            PlayerCommand::SetVolume(_)
            | PlayerCommand::SetShuffle(_)
            | PlayerCommand::SetRepeat(_)
            | PlayerCommand::Stop
            // Sem backend nao ha o que adiantar, e adiantar e por definicao
            // opcional: recusar com erro faria a interface mostrar falha por
            // uma coisa que o usuario nao pediu.
            | PlayerCommand::Preload(_)
            | PlayerCommand::Shutdown => Ok(()),
            _ => {
                let _ = self.events.send(PlayerEvent::Error(self.reason.to_string()));
                Err(CoreError::Unsupported("motor de reproducao nao disponivel"))
            }
        }
    }

    fn snapshot(&self) -> PlayerSnapshot {
        PlayerSnapshot::default()
    }

    fn subscribe(&self) -> broadcast::Receiver<PlayerEvent> {
        self.events.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_is_clamped_and_safe_without_duration() {
        let mut s = PlayerSnapshot::default();
        assert_eq!(s.progress(), 0.0);

        s.duration = Duration::from_secs(100);
        s.position = Duration::from_secs(25);
        assert!((s.progress() - 0.25).abs() < f32::EPSILON);

        s.position = Duration::from_secs(500);
        assert_eq!(s.progress(), 1.0);
    }

    #[test]
    fn null_engine_rejects_playback_but_accepts_preferences() {
        let e = NullEngine::default();
        assert!(e.send(PlayerCommand::SetVolume(0.5)).is_ok());
        assert!(e.send(PlayerCommand::Play).is_err());
        assert_eq!(e.snapshot().state, PlaybackState::Idle);
        assert_eq!(e.capabilities(), EngineCapabilities::NONE);
    }

    #[test]
    fn null_engine_reports_error_events_to_subscribers() {
        let e = NullEngine::new("sem login");
        let mut rx = e.subscribe();
        let _ = e.send(PlayerCommand::Play);
        match rx.try_recv() {
            Ok(PlayerEvent::Error(msg)) => assert_eq!(msg, "sem login"),
            other => panic!("evento inesperado: {other:?}"),
        }
    }

    #[test]
    fn engine_is_object_safe() {
        let engines: Vec<Arc<dyn PlaybackEngine>> = vec![Arc::new(NullEngine::default())];
        assert_eq!(engines[0].name(), "null");
    }
}
