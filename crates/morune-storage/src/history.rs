use std::fs;
use std::path::Path;

use morune_core::{Track, TrackId};
use serde::{Deserialize, Serialize};

const HISTORY_VERSION: u32 = 1;

/// Quantas faixas o historico guarda.
///
/// O suficiente para uma prateleira "Tocadas recentemente" com folga, pequeno o
/// bastante para o arquivo nunca passar de algumas dezenas de KB.
pub const MAX_TRACKS: usize = 50;

/// Quantas buscas recentes aparecem sob o campo de busca.
pub const MAX_SEARCHES: usize = 8;

/// O que o Morune lembra do uso, **so no computador da pessoa**.
///
/// Existe porque o protocolo interno do Spotify nao tem rota conhecida para
/// "tocadas recentemente" (`recently_played` responde sem caminho, ver
/// `morune-spotify/src/catalog.rs`). Em vez de uma prateleira vazia, o Morune
/// guarda o proprio historico. Nada disto sai da maquina.
///
/// **Custo:** uma gravacao por faixa que comeca (a cada ~3 min tocando) e uma
/// por busca confirmada. Nada roda por quadro nem em repouso.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct History {
    version: u32,
    /// Mais recente primeiro, sem repeticao.
    tracks: Vec<Track>,
    /// Mais recente primeiro, sem repeticao, sem diferenciar maiusculas.
    searches: Vec<String>,
    /// Faixas que a pessoa pediu para nao tocar no radio/autoplay.
    hidden: Vec<TrackId>,
}

impl Default for History {
    fn default() -> Self {
        Self {
            version: HISTORY_VERSION,
            tracks: Vec::new(),
            searches: Vec::new(),
            hidden: Vec::new(),
        }
    }
}

impl History {
    /// Le o historico. Arquivo ausente ou corrompido vira historico vazio: e
    /// conveniencia, nao dado que valha travar a abertura.
    pub fn load(path: &Path) -> Self {
        let Ok(raw) = fs::read_to_string(path) else {
            return Self::default();
        };
        match toml::from_str::<Self>(&raw) {
            Ok(mut h) => {
                h.version = HISTORY_VERSION;
                h.tracks.truncate(MAX_TRACKS);
                h.searches.truncate(MAX_SEARCHES);
                h
            }
            Err(e) => {
                tracing::warn!(error = %e, "historico local invalido; comecando vazio");
                Self::default()
            }
        }
    }

    /// Grava por arquivo temporario e renomeacao: uma queda no meio deixa o
    /// arquivo anterior inteiro.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let temp = path.with_extension("toml.tmp");
        fs::write(&temp, text)?;
        fs::rename(&temp, path)
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn searches(&self) -> &[String] {
        &self.searches
    }

    /// Registra uma faixa que comecou. Devolve `false` se ela ja era a mais
    /// recente -- nesse caso nao ha o que gravar.
    pub fn record_track(&mut self, track: &Track) -> bool {
        if self.tracks.first().is_some_and(|t| t.id == track.id) {
            return false;
        }
        self.tracks.retain(|t| t.id != track.id);
        self.tracks.insert(0, track.clone());
        self.tracks.truncate(MAX_TRACKS);
        true
    }

    /// Registra uma busca confirmada. Texto vazio nao entra.
    pub fn record_search(&mut self, query: &str) -> bool {
        let query = query.trim();
        if query.is_empty() {
            return false;
        }
        if self
            .searches
            .first()
            .is_some_and(|s| s.eq_ignore_ascii_case(query))
        {
            return false;
        }
        self.searches.retain(|s| !s.eq_ignore_ascii_case(query));
        self.searches.insert(0, query.to_string());
        self.searches.truncate(MAX_SEARCHES);
        true
    }

    pub fn clear_searches(&mut self) -> bool {
        let had = !self.searches.is_empty();
        self.searches.clear();
        had
    }

    pub fn is_hidden(&self, id: &TrackId) -> bool {
        self.hidden.contains(id)
    }

    /// Esconde ou volta a mostrar. Devolve o estado novo (`true` = escondida).
    pub fn toggle_hidden(&mut self, id: &TrackId) -> bool {
        if let Some(i) = self.hidden.iter().position(|h| h == id) {
            self.hidden.remove(i);
            false
        } else {
            self.hidden.push(id.clone());
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    fn faixa(id: &str) -> Track {
        Track {
            id: TrackId::spotify(id),
            name: Arc::from(id),
            artists: Vec::new(),
            album: None,
            duration: Duration::from_secs(60),
            track_number: None,
            disc_number: None,
            explicit: false,
            playable: true,
        }
    }

    #[test]
    fn faixa_repetida_sobe_para_o_topo_sem_duplicar() {
        let mut h = History::default();
        assert!(h.record_track(&faixa("a")));
        assert!(h.record_track(&faixa("b")));
        assert!(
            !h.record_track(&faixa("b")),
            "a mais recente de novo nao grava"
        );
        assert!(h.record_track(&faixa("a")));
        let ids: Vec<_> = h.tracks().iter().map(|t| t.id.canonical()).collect();
        assert_eq!(ids, ["spotify:a", "spotify:b"]);
    }

    #[test]
    fn historico_tem_teto() {
        let mut h = History::default();
        for i in 0..(MAX_TRACKS + 10) {
            h.record_track(&faixa(&i.to_string()));
        }
        assert_eq!(h.tracks().len(), MAX_TRACKS);
    }

    #[test]
    fn busca_ignora_vazio_e_maiusculas() {
        let mut h = History::default();
        assert!(!h.record_search("   "));
        assert!(h.record_search("Frank Ocean"));
        assert!(!h.record_search("frank ocean"));
        assert!(h.record_search("Djavan"));
        assert!(h.record_search("FRANK OCEAN"));
        assert_eq!(h.searches(), ["FRANK OCEAN", "Djavan"]);
    }

    #[test]
    fn esconder_alterna() {
        let mut h = History::default();
        let id = TrackId::spotify("x");
        assert!(h.toggle_hidden(&id));
        assert!(h.is_hidden(&id));
        assert!(!h.toggle_hidden(&id));
        assert!(!h.is_hidden(&id));
    }

    #[test]
    fn grava_e_le_de_volta() {
        let dir = std::env::temp_dir().join(format!("morune-hist-{}", std::process::id()));
        let path = dir.join("history.toml");
        let mut h = History::default();
        h.record_track(&faixa("a"));
        h.record_search("teste");
        h.save(&path).unwrap();
        assert_eq!(History::load(&path), h);
        let _ = fs::remove_dir_all(dir);
    }
}
