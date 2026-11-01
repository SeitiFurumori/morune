//! O desenho da musica para a barra de progresso.
//!
//! Vem da "analise de audio" do Spotify, que o Web API tirou do ar para apps
//! novos em 2024 mas o spclient ainda serve no mesmo formato:
//! `GET /audio-attributes/v1/audio-analysis/{id}` (o cliente oficial chama por
//! `wg://`, o spotatui confirmou logado em 10/2026). Faixa sem analise
//! responde 404, e a tela desenha a barra enquanto toca.
//!
//! Da resposta so interessa o volume de cada segmento (`loudness_max`, em dB,
//! segmentos de 0,2 a 0,5 s). O resto -- notas, timbre, batidas -- e ignorado
//! sem nem ser lido para memoria.

use serde::Deserialize;

/// Quantos picos vao para a tela. A barra tem ~140 tracos; o dobro deixa a tela
/// reamostrar sem ficar quadrada em janela larga.
pub(crate) const PICOS: usize = 280;

/// Faixa de volume desenhada, em dB: do trecho baixo da musica (o 5o percentil,
/// para um silencio no fim nao achatar o resto) ate o pico, nunca menos que
/// isto. Musica pop e mixada quase toda no mesmo volume; medir contra 30 dB
/// fixos deixava todos os tracos da mesma altura.
const FAIXA_MINIMA_DB: f32 = 6.0;
const FAIXA_MAXIMA_DB: f32 = 30.0;

#[derive(Deserialize)]
struct Analise {
    #[serde(default)]
    track: Option<Faixa>,
    #[serde(default)]
    segments: Vec<Segmento>,
}

#[derive(Deserialize)]
struct Faixa {
    #[serde(default)]
    duration: f32,
}

#[derive(Deserialize)]
struct Segmento {
    start: f32,
    duration: f32,
    loudness_max: f32,
}

/// Picos de 0 a 1, um por fatia de tempo, a partir do JSON da analise.
///
/// O volume e medido contra o pico da propria musica: uma faixa baixa nao vira
/// uma barra achatada.
pub(crate) fn picos(json: &[u8]) -> Option<Vec<f32>> {
    let analise: Analise = serde_json::from_slice(json).ok()?;
    let fim = analise
        .segments
        .iter()
        .map(|s| s.start + s.duration)
        .fold(0.0_f32, f32::max);
    let duracao = analise
        .track
        .map(|t| t.duration)
        .filter(|d| *d > 0.0)
        .unwrap_or(fim);
    if analise.segments.is_empty() || duracao <= 0.0 {
        return None;
    }

    let mut volumes: Vec<f32> = analise.segments.iter().map(|s| s.loudness_max).collect();
    volumes.sort_by(f32::total_cmp);
    let alto = volumes[volumes.len() - 1];
    let baixo = volumes[volumes.len() / 20];
    let faixa = (alto - baixo).clamp(FAIXA_MINIMA_DB, FAIXA_MAXIMA_DB);
    let mut saida = vec![0.0_f32; PICOS];
    for s in &analise.segments {
        // Ao quadrado: separa o refrao do verso, que no volume cru ficam perto.
        let v = ((s.loudness_max - (alto - faixa)) / faixa).clamp(0.0, 1.0).powi(2);
        let a = ((s.start / duracao) * PICOS as f32) as usize;
        let b = (((s.start + s.duration) / duracao) * PICOS as f32).ceil() as usize;
        for p in saida.iter_mut().take(b.min(PICOS)).skip(a.min(PICOS - 1)) {
            *p = p.max(v);
        }
    }
    Some(saida)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_trecho_mais_alto_vira_pico_cheio() {
        let json = br#"{"track":{"duration":10.0},"segments":[
            {"start":0.0,"duration":5.0,"loudness_max":-30.0,"pitches":[0.1]},
            {"start":5.0,"duration":5.0,"loudness_max":-3.0}]}"#;
        let p = picos(json).unwrap();
        assert_eq!(p.len(), PICOS);
        assert_eq!(p[PICOS - 1], 1.0);
        // O trecho mais baixo vira o traco minimo.
        assert_eq!(p[0], 0.0);
    }

    #[test]
    fn analise_vazia_nao_desenha() {
        assert!(picos(br#"{"segments":[]}"#).is_none());
        assert!(picos(b"nao e json").is_none());
    }
}
