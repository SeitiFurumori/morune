//! O desenho da musica para a barra de progresso.
//!
//! Vem da "analise de audio" do Spotify, que o Web API tirou do ar para apps
//! novos em 2024 mas o spclient ainda serve no mesmo formato:
//! `GET /audio-attributes/v1/audio-analysis/{id}` (o cliente oficial chama por
//! `wg://`, o spotatui confirmou logado em 10/2026). Faixa sem analise
//! responde 404, e a tela desenha a barra enquanto toca.
//!
//! Da resposta interessam duas coisas: o volume de cada segmento
//! (`loudness_max`, segmentos de 0,2 a 0,5 s) e o volume medio de cada parte
//! da musica (`sections`: introducao, verso, refrao...). O resto -- notas,
//! timbre, batidas -- e ignorado sem nem ser lido para memoria.
//!
//! **Por que as partes.** Musica pop e mixada quase toda no mesmo volume: so
//! com os segmentos, a barra era uma fileira de tracos parecidos, e nao dava
//! para achar o refrao para pular ate ele. As partes dao os degraus grandes, e
//! os segmentos o detalhe dentro de cada uma.

use serde::Deserialize;

/// Quantos picos vao para a tela. A barra tem ~140 tracos; o dobro deixa a tela
/// reamostrar sem ficar quadrada em janela larga.
pub(crate) const PICOS: usize = 280;

/// Quanto das alturas vem das partes; o resto vem dos segmentos.
const PESO_PARTES: f32 = 0.6;

#[derive(Deserialize)]
struct Analise {
    #[serde(default)]
    track: Option<Faixa>,
    #[serde(default)]
    sections: Vec<Parte>,
    #[serde(default)]
    segments: Vec<Segmento>,
}

#[derive(Deserialize)]
struct Faixa {
    #[serde(default)]
    duration: f32,
}

#[derive(Deserialize)]
struct Parte {
    start: f32,
    duration: f32,
    loudness: f32,
}

#[derive(Deserialize)]
struct Segmento {
    start: f32,
    duration: f32,
    loudness_max: f32,
}

/// Leva valores em dB para 0..1, do percentil `baixo` ao `alto` da propria
/// musica, com pelo menos `minimo` dB de faixa. Medir contra a propria musica
/// e o que impede uma faixa baixa de virar uma barra achatada.
fn normalizador(valores: &[f32], baixo: f32, alto: f32, minimo: f32) -> impl Fn(f32) -> f32 {
    let mut v = valores.to_vec();
    v.sort_by(f32::total_cmp);
    let pega = |p: f32| v[((v.len() - 1) as f32 * p).round() as usize];
    let (a, b) = (pega(baixo), pega(alto));
    let faixa = (b - a).max(minimo);
    let piso = b - faixa;
    move |db| ((db - piso) / faixa).clamp(0.0, 1.0)
}

/// Picos de 0 a 1, um por fatia de tempo, a partir do JSON da analise.
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
    let fatia = |inicio: f32, dur: f32| {
        let a = ((inicio / duracao) * PICOS as f32) as usize;
        let b = (((inicio + dur) / duracao) * PICOS as f32).ceil() as usize;
        a.min(PICOS - 1)..b.min(PICOS)
    };

    // Segmentos: o maior de cada fatia, entre o 10o e o 99o percentil -- a
    // ponta de cima tira um estalo isolado, a de baixo tira o silencio.
    let volumes: Vec<f32> = analise.segments.iter().map(|s| s.loudness_max).collect();
    let norm = normalizador(&volumes, 0.10, 0.99, 6.0);
    let mut detalhe = vec![0.0_f32; PICOS];
    for s in &analise.segments {
        let v = norm(s.loudness_max);
        for p in &mut detalhe[fatia(s.start, s.duration)] {
            *p = p.max(v);
        }
    }

    // Partes: o volume medio de cada uma, da mais baixa a mais alta.
    let medias: Vec<f32> = analise.sections.iter().map(|p| p.loudness).collect();
    let mut partes = detalhe.clone();
    if medias.len() >= 2 {
        let norm = normalizador(&medias, 0.0, 1.0, 3.0);
        for p in &analise.sections {
            let v = norm(p.loudness);
            for x in &mut partes[fatia(p.start, p.duration)] {
                *x = v;
            }
        }
    }

    Some(
        detalhe
            .iter()
            .zip(&partes)
            // Elevado: separa ainda mais o forte do fraco, que e o que o olho
            // usa para achar o trecho.
            .map(|(d, p)| (PESO_PARTES * p + (1.0 - PESO_PARTES) * d).powf(1.6))
            .collect(),
    )
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
    fn o_refrao_fica_acima_do_verso_mesmo_com_volume_parecido() {
        // Segmentos quase iguais (pop mixado alto), partes com 4 dB de
        // diferenca: o refrao tem de sair visivelmente mais alto.
        let json = br#"{"track":{"duration":20.0},
            "sections":[{"start":0.0,"duration":10.0,"loudness":-9.0},
                        {"start":10.0,"duration":10.0,"loudness":-5.0}],
            "segments":[{"start":0.0,"duration":10.0,"loudness_max":-4.0},
                        {"start":10.0,"duration":10.0,"loudness_max":-3.0}]}"#;
        let p = picos(json).unwrap();
        assert!(p[PICOS - 1] - p[0] > 0.5, "verso {} refrao {}", p[0], p[PICOS - 1]);
    }

    #[test]
    fn analise_vazia_nao_desenha() {
        assert!(picos(br#"{"segments":[]}"#).is_none());
        assert!(picos(b"nao e json").is_none());
    }
}
