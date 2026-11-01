//! Retomar de onde parou: a fila e a posicao vao para um arquivo enquanto a
//! musica toca, e o app abre com a mesma faixa, pausada, no mesmo ponto.
//!
//! Pausada de proposito: abrir o computador e o som sair sozinho assusta. O
//! Felipe inicia o Morune com o Windows e escolhe a musica para trabalhar; com
//! isto, a escolha vira um clique em tocar.

use std::path::{Path, PathBuf};
use std::time::Duration;

use morune_core::queue::QueueOrigin;
use morune_core::Track;
use serde::{Deserialize, Serialize};

/// Mais que isso nao cabe numa fila que alguem ouve; e o arquivo fica pequeno.
const MAX_FAIXAS: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Retomada {
    pub origem: QueueOrigin,
    pub faixas: Vec<Track>,
    /// Indice da faixa atual em `faixas`.
    pub atual: usize,
    pub posicao_ms: u64,
}

impl Retomada {
    /// Fotografa a fila. `None` quando nao ha faixa atual.
    ///
    /// Fila maior que [`MAX_FAIXAS`] guarda uma janela em volta da atual.
    pub fn de(origem: &QueueOrigin, faixas: &[Track], atual: usize, posicao: Duration) -> Option<Self> {
        if atual >= faixas.len() {
            return None;
        }
        let inicio = atual.saturating_sub(MAX_FAIXAS / 2);
        let fim = (inicio + MAX_FAIXAS).min(faixas.len());
        Some(Self {
            origem: origem.clone(),
            faixas: faixas[inicio..fim].to_vec(),
            atual: atual - inicio,
            posicao_ms: posicao.as_millis() as u64,
        })
    }

    pub fn posicao(&self) -> Duration {
        Duration::from_millis(self.posicao_ms)
    }
}

pub fn arquivo(pasta_dados: &Path) -> PathBuf {
    pasta_dados.join("retomar.json")
}

pub fn ler(pasta_dados: &Path) -> Option<Retomada> {
    let texto = std::fs::read_to_string(arquivo(pasta_dados)).ok()?;
    match serde_json::from_str::<Retomada>(&texto) {
        Ok(r) if r.atual < r.faixas.len() => Some(r),
        Ok(_) => None,
        Err(error) => {
            tracing::warn!(%error, "arquivo de retomar ilegivel; ignorado");
            None
        }
    }
}

/// Grava num arquivo temporario e troca: queda no meio nao deixa meio arquivo.
pub fn gravar(pasta_dados: &Path, r: &Retomada) {
    let destino = arquivo(pasta_dados);
    let temp = destino.with_extension("json.tmp");
    let resultado = serde_json::to_vec(r)
        .map_err(std::io::Error::other)
        .and_then(|bytes| std::fs::write(&temp, bytes))
        .and_then(|_| std::fs::rename(&temp, &destino));
    if let Err(error) = resultado {
        tracing::warn!(%error, "nao foi possivel gravar onde a musica parou");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use morune_core::{Provider, TrackId};

    fn faixa(n: usize) -> Track {
        Track {
            id: TrackId::new(Provider::Spotify, format!("t{n}")),
            name: format!("Faixa {n}").into(),
            artists: vec![],
            album: None,
            duration: Duration::from_secs(200),
            track_number: None,
            disc_number: None,
            explicit: false,
            playable: true,
        }
    }

    #[test]
    fn fila_grande_guarda_janela_em_volta_da_atual() {
        let faixas: Vec<Track> = (0..2000).map(faixa).collect();
        let r = Retomada::de(&QueueOrigin::None, &faixas, 1500, Duration::from_secs(42)).unwrap();
        assert_eq!(r.faixas.len(), MAX_FAIXAS);
        assert_eq!(r.faixas[r.atual].name.as_ref(), "Faixa 1500");
        assert_eq!(r.posicao(), Duration::from_secs(42));
    }

    #[test]
    fn grava_e_le_de_volta() {
        let pasta = std::env::temp_dir().join(format!("morune-retomar-{}", std::process::id()));
        std::fs::create_dir_all(&pasta).unwrap();
        let faixas: Vec<Track> = (0..3).map(faixa).collect();
        let r = Retomada::de(&QueueOrigin::Playlist("p".into()), &faixas, 2, Duration::from_millis(1234)).unwrap();
        gravar(&pasta, &r);
        let lida = ler(&pasta).unwrap();
        assert_eq!(lida.atual, 2);
        assert_eq!(lida.posicao_ms, 1234);
        assert_eq!(lida.origem, QueueOrigin::Playlist("p".into()));
        let _ = std::fs::remove_dir_all(&pasta);
    }
}
