//! Saida de audio do Morune.
//!
//! **Por que nao a da librespot.** O `RodioSink` que vem na librespot mantem
//! cerca de meio segundo de audio ja decodificado na fila do rodio, e trata
//! toda mudanca como se ela pudesse esperar esse meio segundo:
//!
//! - `stop()` chama `sleep_until_end()` antes de pausar, ou seja, pausar espera
//!   a fila inteira tocar ate o fim antes de silenciar;
//! - o volume e aplicado pelo `volume_getter` **antes** da fila, entao o audio
//!   ja enfileirado continua saindo no volume antigo;
//! - `seek` nem toca na fila: o trecho velho toca inteiro antes do novo.
//!
//! O buffer em si e defensivo e esta certo -- o Morune toca enquanto o usuario
//! joga, e encolher a fila trocaria atraso por falha de audio, que e pior. O
//! que estava errado era o buffer atrasar tambem os *comandos*. Aqui a fila
//! continua do mesmo tamanho, e cada comando passa a agir na hora:
//!
//! | comando | como fica imediato |
//! |---|---|
//! | volume | aplicado no misturador do rodio, que reaplica as fontes ja enfileiradas a cada 5 ms |
//! | pausar | `pause()` puro: silencia no ato **sem** descartar a fila, entao voltar continua de onde parou |
//! | trocar de faixa, parar, seek | o motor pede descarte, e a fila e trocada antes do proximo audio |
//!
//! A distincao entre pausar e trocar de faixa nao da para tirar do trait
//! `Sink`, que ve `stop()` nos dois casos. Quem sabe a diferenca e o motor, e e
//! ele que levanta o pedido de descarte -- ver [`FlushRequest`].

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use librespot_playback::audio_backend::{Sink, SinkError, SinkResult};
use librespot_playback::config::AudioFormat;
use librespot_playback::convert::Converter;
use librespot_playback::decoder::AudioPacket;
use librespot_playback::{NUM_CHANNELS, SAMPLE_RATE};
use rodio::cpal;
use rodio::cpal::traits::{DeviceTrait, HostTrait};

use crate::engine::SharedVolume;

/// Pedido de descarte da fila de audio, levantado pelo motor.
///
/// Trocar de faixa, parar e mover a posicao invalidam o que ja esta enfileirado.
/// Pausar **nao**: a fila preservada e o que faz voltar a tocar continuar de
/// onde o som parou, em vez de pular o pedaco que estava no buffer.
pub(crate) type FlushRequest = Arc<AtomicBool>;

/// Teto de fontes enfileiradas no rodio.
///
/// Herdado da librespot: os pacotes decodificados tem entre 256 e 3000 amostras,
/// e 26 deles dao aproximadamente meio segundo. E a folga que segura um engasgo
/// de decodificacao enquanto o usuario joga, e por isso continua igual -- o
/// atraso que se estava tentando resolver nao vinha do tamanho da fila, e sim
/// de os comandos esperarem por ela.
const MAX_QUEUED: usize = 26;

/// Pausa entre tentativas quando a fila esta cheia.
///
/// Isto roda na thread da librespot, nunca na da interface.
const DRAIN_WAIT: Duration = Duration::from_millis(10);

pub(crate) struct MoruneSink {
    /// Mantem o dispositivo aberto. Trocar a fila nao pode fechar a saida, so
    /// o que esta dentro dela.
    stream: rodio::OutputStream,
    sink: rodio::Sink,
    volume: Arc<SharedVolume>,
    flush: FlushRequest,
}

impl MoruneSink {
    /// Troca a fila por uma vazia, descartando o que estava enfileirado.
    ///
    /// **Nao bloqueia**, e essa e a razao de ser assim em vez de `clear()`: o
    /// `Drop` do `rodio::Sink` so marca as fontes como paradas e volta, enquanto
    /// `clear()` espera o misturador confirmar. Isto roda na thread do player da
    /// librespot, que tambem processa os comandos -- segurar aqui atrasaria o
    /// comando seguinte.
    fn reset_queue(&mut self) {
        self.sink = rodio::Sink::connect_new(self.stream.mixer());
        self.sink.set_volume(self.volume.attenuation() as f32);
    }

    /// Atende um descarte pedido pelo motor, se houver.
    fn take_flush(&mut self) -> bool {
        if self.flush.swap(false, Ordering::AcqRel) {
            self.reset_queue();
            return true;
        }
        false
    }
}

impl Sink for MoruneSink {
    fn start(&mut self) -> SinkResult<()> {
        self.take_flush();
        self.sink.play();
        Ok(())
    }

    /// Silencia sem descartar.
    ///
    /// A librespot chama isto tanto ao pausar quanto ao trocar de faixa, e so o
    /// motor sabe qual dos dois e. Pausar preservando a fila e o comportamento
    /// certo para o caso que nao pede descarte; o outro chega pelo
    /// [`FlushRequest`] e e atendido antes do proximo audio sair.
    ///
    /// A librespot original drenava a fila aqui, o que fazia pausar levar meio
    /// segundo. `pause()` corta o som dentro de um bloco do misturador.
    fn stop(&mut self) -> SinkResult<()> {
        self.sink.pause();
        Ok(())
    }

    fn write(&mut self, packet: AudioPacket, converter: &mut Converter) -> SinkResult<()> {
        self.take_flush();

        // O volume vive aqui, e nao no `volume_getter` da librespot, porque o
        // misturador reaplica este valor as fontes ja enfileiradas a cada 5 ms.
        // Aplicado antes da fila, meio segundo de audio continuaria saindo no
        // volume anterior.
        self.sink.set_volume(self.volume.attenuation() as f32);

        let samples = packet
            .samples()
            .map_err(|e| SinkError::OnWrite(e.to_string()))?;
        let samples_f32: &[f32] = &converter.f64_to_f32(samples);
        self.sink.append(rodio::buffer::SamplesBuffer::new(
            NUM_CHANNELS as cpal::ChannelCount,
            SAMPLE_RATE,
            samples_f32,
        ));

        // Contrapressao: sem isto a decodificacao correria na frente da saida e
        // a fila cresceria sem limite.
        while self.sink.len() > MAX_QUEUED {
            thread::sleep(DRAIN_WAIT);
        }
        Ok(())
    }
}

/// Abre a saida de audio padrao do sistema.
///
/// A negociacao de formato e a mesma da librespot, de proposito: estereo em
/// 44,1 kHz quando o dispositivo aceita, senao a taxa padrao dele, senao o que
/// houver. Sair disso trocaria uma reamostragem que hoje nao acontece por uma
/// que aconteceria.
pub(crate) fn open(volume: Arc<SharedVolume>, flush: FlushRequest) -> Result<MoruneSink, String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "nenhum dispositivo de saida disponivel".to_string())?;

    if let Ok(name) = device.name() {
        tracing::info!(dispositivo = %name, "saida de audio");
    }

    let default_config = device
        .default_output_config()
        .map_err(|e| format!("dispositivo sem configuracao padrao: {e}"))?;
    let config = device
        .supported_output_configs()
        .map_err(|e| format!("dispositivo sem formatos suportados: {e}"))?
        .find(|c| c.channels() == NUM_CHANNELS as cpal::ChannelCount)
        .and_then(|c| {
            c.try_with_sample_rate(cpal::SampleRate(SAMPLE_RATE))
                .or_else(|| c.try_with_sample_rate(default_config.sample_rate()))
        })
        .unwrap_or(default_config);

    let sample_format = match AudioFormat::default() {
        AudioFormat::F64 => cpal::SampleFormat::F64,
        AudioFormat::F32 => cpal::SampleFormat::F32,
        AudioFormat::S32 => cpal::SampleFormat::I32,
        AudioFormat::S24 | AudioFormat::S24_3 => cpal::SampleFormat::I24,
        AudioFormat::S16 => cpal::SampleFormat::I16,
    };

    let mut stream = match rodio::OutputStreamBuilder::default()
        .with_device(device.clone())
        .with_config(&config.config())
        .with_sample_format(sample_format)
        .open_stream()
    {
        Ok(exact) => exact,
        Err(e) => {
            tracing::warn!(error = %e, "formato exato recusado; usando o padrao do dispositivo");
            rodio::OutputStreamBuilder::from_device(device)
                .map_err(|e| format!("nao foi possivel abrir a saida: {e}"))?
                .open_stream_or_fallback()
                .map_err(|e| format!("nao foi possivel abrir a saida: {e}"))?
        }
    };

    // O rodio registra a destruicao do stream no log de saida; aqui isso
    // apareceria como erro no encerramento normal.
    stream.log_on_drop(false);

    let sink = rodio::Sink::connect_new(stream.mixer());
    sink.set_volume(volume.attenuation() as f32);

    Ok(MoruneSink { stream, sink, volume, flush })
}
