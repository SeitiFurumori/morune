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
//!
//! **A saida fecha quando ninguem esta ouvindo.** Um `rodio::OutputStream`
//! aberto e uma thread do WASAPI acordando a cada bloco de audio para misturar
//! silencio -- medido em 06/09/2026, `cpal_wasapi_out` sozinha custava 0,47% de
//! um nucleo com o Morune parado, mais da metade de todo o gasto em repouso.
//! Para um aplicativo cujo criterio e ser indistinguivel de um processo parado
//! enquanto alguem joga, isso e caro e nao compra nada: nao ha som saindo.
//!
//! Por isso o dispositivo vira [`Option`]: aberto enquanto toca, fechado
//! [`GRACA_ATE_FECHAR`] depois que o som para, reaberto sozinho no proximo
//! audio. Quem fecha e um vigia proprio, porque quando ninguem esta tocando
//! tambem ninguem chama este modulo -- nao ha em que pegar carona.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

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

/// Quanto tempo o dispositivo continua aberto depois que o som para.
///
/// Abrir a saida custa dezenas de milissegundos, e pausar e trocar de faixa
/// acontecem o tempo todo: fechar na hora colocaria essa espera no meio da
/// escuta. Cinco segundos passam folgados por qualquer pausa entre faixas e
/// ainda assim fecham a saida muito antes de alguem reparar no aplicativo
/// parado.
const GRACA_ATE_FECHAR: Duration = Duration::from_secs(5);

/// De quanto em quanto tempo o vigia confere se ja da para fechar.
///
/// Uma vez por segundo contra as centenas de vezes por segundo que a thread do
/// WASAPI acorda: o vigia custa perto de nada comparado ao que ele desliga.
const RONDA: Duration = Duration::from_secs(1);

/// Mensagem de um cadeado envenenado. So acontece se outra thread entrar em
/// panico segurando o estado, e nesse caso o audio ja acabou de qualquer jeito.
const ENVENENADO: &str = "estado da saida de audio envenenado";

/// O dispositivo aberto e a fila ligada a ele.
///
/// Os dois nascem e morrem juntos: a fila do rodio se conecta ao misturador
/// deste `stream`, entao guardar uma sem a outra daria uma fila que aceita
/// audio e nao toca em lugar nenhum.
struct Saida {
    stream: rodio::OutputStream,
    sink: rodio::Sink,
}

impl Saida {
    /// Troca a fila por uma vazia, descartando o que estava enfileirado.
    ///
    /// **Nao bloqueia**, e essa e a razao de ser assim em vez de `clear()`: o
    /// `Drop` do `rodio::Sink` so marca as fontes como paradas e volta, enquanto
    /// `clear()` espera o misturador confirmar. Isto roda na thread do player da
    /// librespot, que tambem processa os comandos -- segurar aqui atrasaria o
    /// comando seguinte.
    fn reset_queue(&mut self, volume: &SharedVolume) {
        self.sink = rodio::Sink::connect_new(self.stream.mixer());
        self.sink.set_volume(volume.attenuation() as f32);
    }
}

/// Tudo que o vigia e a thread do player compartilham.
struct Estado {
    /// `None` quer dizer dispositivo fechado, e nao dispositivo com defeito.
    saida: Option<Saida>,
    /// Desde quando nao sai som. `None` enquanto esta tocando.
    parado_desde: Option<Instant>,
    /// Nome pedido nas Configuracoes, guardado para reabrir no mesmo lugar.
    preferido: String,
    volume: Arc<SharedVolume>,
}

impl Estado {
    /// Devolve a saida, abrindo o dispositivo se ele estiver fechado.
    fn aberta(&mut self) -> Result<&mut Saida, String> {
        if self.saida.is_none() {
            let nova = abrir_saida(&self.preferido, &self.volume)?;
            tracing::debug!("saida de audio reaberta");
            self.saida = Some(nova);
        }
        Ok(self
            .saida
            .as_mut()
            .expect("a saida acabou de ser aberta acima"))
    }
}

/// Decide se o dispositivo ja pode fechar.
///
/// Separado do vigia para poder ser testado sem placa de som.
fn ja_pode_fechar(parado_desde: Option<Instant>, aberta: bool, agora: Instant) -> bool {
    aberta
        && parado_desde
            .is_some_and(|t| agora.saturating_duration_since(t) >= GRACA_ATE_FECHAR)
}

pub(crate) struct MoruneSink {
    estado: Arc<Mutex<Estado>>,
    flush: FlushRequest,
}

impl MoruneSink {
    /// Atende um descarte pedido pelo motor, se houver.
    fn take_flush(&self, saida: &mut Saida, volume: &SharedVolume) {
        if self.flush.swap(false, Ordering::AcqRel) {
            saida.reset_queue(volume);
        }
    }
}

impl Sink for MoruneSink {
    fn start(&mut self) -> SinkResult<()> {
        let mut estado = self.estado.lock().expect(ENVENENADO);
        estado.parado_desde = None;
        let volume = estado.volume.clone();
        let saida = estado.aberta().map_err(SinkError::ConnectionRefused)?;
        self.take_flush(saida, &volume);
        saida.sink.play();
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
    ///
    /// Aqui tambem comeca a contagem para fechar o dispositivo: silencio que
    /// dura e silencio que nao precisa de saida aberta.
    fn stop(&mut self) -> SinkResult<()> {
        let mut estado = self.estado.lock().expect(ENVENENADO);
        if let Some(saida) = estado.saida.as_ref() {
            saida.sink.pause();
        }
        estado.parado_desde = Some(Instant::now());
        Ok(())
    }

    fn write(&mut self, packet: AudioPacket, converter: &mut Converter) -> SinkResult<()> {
        // A conversao nao depende do dispositivo e nao precisa do cadeado.
        let samples = packet
            .samples()
            .map_err(|e| SinkError::OnWrite(e.to_string()))?;
        let samples_f32: &[f32] = &converter.f64_to_f32(samples);

        {
            let mut estado = self.estado.lock().expect(ENVENENADO);
            // Chegou audio: a contagem para fechar recomeca do zero.
            estado.parado_desde = None;
            let volume = estado.volume.clone();
            let saida = estado.aberta().map_err(SinkError::ConnectionRefused)?;
            self.take_flush(saida, &volume);

            // O volume vive aqui, e nao no `volume_getter` da librespot, porque
            // o misturador reaplica este valor as fontes ja enfileiradas a cada
            // 5 ms. Aplicado antes da fila, meio segundo de audio continuaria
            // saindo no volume anterior.
            saida.sink.set_volume(volume.attenuation() as f32);
            saida.sink.append(rodio::buffer::SamplesBuffer::new(
                NUM_CHANNELS as cpal::ChannelCount,
                SAMPLE_RATE,
                samples_f32,
            ));
        }

        // Contrapressao: sem isto a decodificacao correria na frente da saida e
        // a fila cresceria sem limite. O cadeado e solto antes de cada espera --
        // segurar aqui travaria o vigia e qualquer comando por meio segundo.
        loop {
            {
                let estado = self.estado.lock().expect(ENVENENADO);
                let cheia = estado
                    .saida
                    .as_ref()
                    .is_some_and(|s| s.sink.len() > MAX_QUEUED);
                if !cheia {
                    break;
                }
            }
            thread::sleep(DRAIN_WAIT);
        }
        Ok(())
    }
}

/// Fecha a saida sozinho depois de [`GRACA_ATE_FECHAR`] em silencio.
///
/// Precisa de thread propria: quando nada esta tocando, nada chama este modulo,
/// entao nao existe evento em que pendurar a verificacao. Ela segura uma
/// referencia fraca de proposito -- assim o vigia morre junto com a saida, sem
/// mante-la viva nem precisar de sinal de encerramento.
fn vigiar(estado: &Arc<Mutex<Estado>>) {
    let fraco = Arc::downgrade(estado);
    let resultado = thread::Builder::new()
        .name("morune-audio-ocioso".to_string())
        .spawn(move || loop {
            thread::sleep(RONDA);
            let Some(estado) = fraco.upgrade() else { return };
            let Ok(mut estado) = estado.lock() else { return };
            if ja_pode_fechar(estado.parado_desde, estado.saida.is_some(), Instant::now()) {
                // O `Drop` do `OutputStream` e que fecha o dispositivo e
                // encerra a thread do WASAPI.
                estado.saida = None;
                estado.parado_desde = None;
                tracing::debug!("saida de audio fechada por ociosidade");
            }
        });
    if let Err(e) = resultado {
        tracing::warn!(error = %e, "sem vigia de ociosidade; a saida de audio fica aberta");
    }
}

/// Nomes dos dispositivos de saida disponiveis, na ordem em que o sistema os
/// entrega.
///
/// Serve a tela de Configuracoes. Nao inclui a opcao "padrao do sistema" --
/// essa e a ausencia de escolha, representada por nome vazio, e quem monta a
/// lista e que decide como apresenta-la.
///
/// Um dispositivo pode sumir entre listar e abrir (fone desconectado, monitor
/// desligado). Por isso [`open`] volta para o padrao em vez de falhar: uma
/// escolha que deixou de existir nao pode impedir a musica de tocar.
pub fn output_devices() -> Vec<String> {
    let host = cpal::default_host();
    let Ok(devices) = host.output_devices() else {
        return Vec::new();
    };
    devices.filter_map(|d| d.name().ok()).collect()
}

/// Abre o dispositivo e conecta uma fila vazia a ele.
///
/// `preferido` vazio significa "o padrao do sistema", que e o que a maioria
/// quer: o Windows ja tem um dispositivo padrao e trocar la deve trocar aqui.
/// Um nome que nao esta mais presente cai no padrao **com aviso no log**, e
/// nao em erro -- ver [`output_devices`].
///
/// A negociacao de formato e a mesma da librespot, de proposito: estereo em
/// 44,1 kHz quando o dispositivo aceita, senao a taxa padrao dele, senao o que
/// houver. Sair disso trocaria uma reamostragem que hoje nao acontece por uma
/// que aconteceria.
fn abrir_saida(preferido: &str, volume: &SharedVolume) -> Result<Saida, String> {
    let host = cpal::default_host();

    let escolhido = if preferido.is_empty() {
        None
    } else {
        let achado = host
            .output_devices()
            .ok()
            .and_then(|mut ds| ds.find(|d| d.name().is_ok_and(|n| n == preferido)));
        if achado.is_none() {
            tracing::warn!(
                dispositivo = %preferido,
                "dispositivo escolhido nao esta disponivel; usando o padrao do sistema"
            );
        }
        achado
    };

    let device = escolhido
        .or_else(|| host.default_output_device())
        .ok_or_else(|| "nenhum dispositivo de saida disponivel".to_string())?;

    // Em `debug`, e nao em `info`: agora o dispositivo reabre a cada retomada,
    // e uma linha por play encheria o log sem contar nada novo. Quem aparece
    // uma vez, no nivel normal, e a abertura de [`open`].
    if let Ok(name) = device.name() {
        tracing::debug!(dispositivo = %name, "saida de audio aberta");
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
    // apareceria como erro no encerramento normal -- e agora a destruicao
    // acontece toda vez que a musica para, nao so ao fechar o aplicativo.
    stream.log_on_drop(false);

    let sink = rodio::Sink::connect_new(stream.mixer());
    sink.set_volume(volume.attenuation() as f32);

    Ok(Saida { stream, sink })
}

/// Monta a saida de audio do Morune.
///
/// O dispositivo e aberto **aqui**, e nao na primeira faixa, para que "esse
/// dispositivo nao abre" apareca como falha de dispositivo na hora de ligar o
/// motor. Depois disso ele passa a ir e vir sozinho: o vigia fecha na
/// ociosidade e o proximo audio reabre.
pub(crate) fn open(
    volume: Arc<SharedVolume>,
    flush: FlushRequest,
    preferido: &str,
) -> Result<MoruneSink, String> {
    let saida = abrir_saida(preferido, &volume)?;
    tracing::info!(
        preferido = if preferido.is_empty() { "padrao do sistema" } else { preferido },
        "saida de audio pronta"
    );

    let estado = Arc::new(Mutex::new(Estado {
        saida: Some(saida),
        // Nada tocou ainda: a contagem ja comeca, e o dispositivo aberto so
        // para conferencia fecha sozinho em segundos se ninguem pedir musica.
        parado_desde: Some(Instant::now()),
        preferido: preferido.to_string(),
        volume,
    }));
    vigiar(&estado);

    Ok(MoruneSink { estado, flush })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saida_fechada_nao_fecha_de_novo() {
        let parado = Instant::now() - GRACA_ATE_FECHAR * 2;
        assert!(!ja_pode_fechar(Some(parado), false, Instant::now()));
    }

    #[test]
    fn tocando_a_saida_nunca_fecha() {
        assert!(!ja_pode_fechar(None, true, Instant::now()));
    }

    #[test]
    fn a_graca_e_respeitada_antes_de_fechar() {
        let agora = Instant::now();
        let parado = agora - GRACA_ATE_FECHAR + Duration::from_millis(500);
        assert!(!ja_pode_fechar(Some(parado), true, agora));
    }

    #[test]
    fn passada_a_graca_em_silencio_a_saida_fecha() {
        let agora = Instant::now();
        let parado = agora - GRACA_ATE_FECHAR;
        assert!(ja_pode_fechar(Some(parado), true, agora));
    }
}
