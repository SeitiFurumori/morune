//! Ordem propria das Musicas curtidas, que o Spotify nao tem.
//!
//! O Spotify guarda as curtidas so na ordem em que foram curtidas, e nao aceita
//! outra. A ordem que o Felipe monta arrastando fica aqui, num arquivo do
//! Morune: no celular a lista continua como o Spotify mostra.
//!
//! O arquivo guarda so ids. Quem decide o que esta curtido continua sendo o
//! Spotify: curtida nova (que nao esta no arquivo) entra no topo, na ordem do
//! Spotify, e a que foi descurtida em outro lugar some sem mexer no resto.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use morune_core::TrackId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OrdemCurtidas {
    ids: Vec<TrackId>,
    /// A ultima lista de curtidas que veio do Spotify. Arrastar usa esta, e
    /// nao um pedido novo: gravar a ordem nao pode depender da conexao.
    #[serde(skip)]
    conhecidas: Vec<TrackId>,
}

impl OrdemCurtidas {
    /// `true` enquanto ninguem arrastou nada: a lista e a do Spotify.
    pub fn vazia(&self) -> bool {
        self.ids.is_empty()
    }

    /// Guarda a lista que acabou de vir do Spotify.
    pub fn lembrar(&mut self, atuais: &[TrackId]) {
        self.conhecidas = atuais.to_vec();
    }

    pub fn conhecidas(&self) -> &[TrackId] {
        &self.conhecidas
    }

    /// A lista do Spotify (`atuais`) na ordem do usuario.
    pub fn aplicar(&self, atuais: &[TrackId]) -> Vec<TrackId> {
        let arrumadas: HashSet<&TrackId> = self.ids.iter().collect();
        let curtidas: HashSet<&TrackId> = atuais.iter().collect();
        atuais
            .iter()
            .filter(|id| !arrumadas.contains(id))
            .chain(self.ids.iter().filter(|id| curtidas.contains(id)))
            .cloned()
            .collect()
    }

    /// Leva `id` para a posicao `para` da lista ja ordenada. A ordem inteira
    /// passa a valer, inclusive as novas que estavam no topo sem lugar fixo.
    pub fn mover(&mut self, atuais: &[TrackId], id: &TrackId, para: usize) -> bool {
        let mut lista = self.aplicar(atuais);
        let Some(de) = lista.iter().position(|item| item == id) else {
            return false;
        };
        let item = lista.remove(de);
        lista.insert(para.min(lista.len()), item);
        self.ids = lista;
        true
    }

    /// Descurtida: sai do arquivo, para que curtir de novo a traga ao topo e
    /// nao ao lugar antigo.
    pub fn esquecer(&mut self, id: &TrackId) -> bool {
        let antes = self.ids.len();
        self.ids.retain(|item| item != id);
        self.ids.len() != antes
    }
}

pub fn arquivo(pasta_dados: &Path) -> PathBuf {
    pasta_dados.join("ordem-curtidas.json")
}

pub fn ler(pasta_dados: &Path) -> OrdemCurtidas {
    let Ok(texto) = std::fs::read_to_string(arquivo(pasta_dados)) else {
        return OrdemCurtidas::default();
    };
    serde_json::from_str(&texto).unwrap_or_else(|error| {
        tracing::warn!(%error, "ordem das curtidas ilegivel; ignorada");
        OrdemCurtidas::default()
    })
}

/// Grava num arquivo temporario e troca: queda no meio nao deixa meio arquivo.
pub fn gravar(pasta_dados: &Path, ordem: &OrdemCurtidas) {
    let destino = arquivo(pasta_dados);
    let temp = destino.with_extension("json.tmp");
    let resultado = serde_json::to_vec(ordem)
        .map_err(std::io::Error::other)
        .and_then(|bytes| std::fs::write(&temp, bytes))
        .and_then(|_| std::fs::rename(&temp, &destino));
    if let Err(error) = resultado {
        tracing::warn!(%error, "nao foi possivel gravar a ordem das curtidas");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(nomes: &[&str]) -> Vec<TrackId> {
        nomes.iter().map(|n| TrackId::spotify(*n)).collect()
    }

    #[test]
    fn sem_arrastar_a_lista_e_a_do_spotify() {
        let ordem = OrdemCurtidas::default();
        assert_eq!(ordem.aplicar(&ids(&["c", "b", "a"])), ids(&["c", "b", "a"]));
    }

    #[test]
    fn arrastar_fixa_a_ordem_inteira() {
        let mut ordem = OrdemCurtidas::default();
        assert!(ordem.mover(&ids(&["c", "b", "a"]), &TrackId::spotify("a"), 0));
        assert_eq!(ordem.aplicar(&ids(&["c", "b", "a"])), ids(&["a", "c", "b"]));
    }

    #[test]
    fn curtida_nova_entra_no_topo_sem_mexer_no_resto() {
        let mut ordem = OrdemCurtidas::default();
        ordem.mover(&ids(&["c", "b", "a"]), &TrackId::spotify("a"), 0);
        assert_eq!(
            ordem.aplicar(&ids(&["e", "d", "c", "b", "a"])),
            ids(&["e", "d", "a", "c", "b"])
        );
    }

    #[test]
    fn descurtida_em_outro_lugar_some_sem_mexer_no_resto() {
        let mut ordem = OrdemCurtidas::default();
        ordem.mover(&ids(&["c", "b", "a"]), &TrackId::spotify("a"), 0);
        assert_eq!(ordem.aplicar(&ids(&["b", "a"])), ids(&["a", "b"]));
    }

    #[test]
    fn descurtir_e_curtir_de_novo_volta_ao_topo() {
        let mut ordem = OrdemCurtidas::default();
        ordem.mover(&ids(&["c", "b", "a"]), &TrackId::spotify("c"), 2);
        assert!(ordem.esquecer(&TrackId::spotify("c")));
        assert_eq!(ordem.aplicar(&ids(&["c", "b", "a"])), ids(&["c", "b", "a"]));
    }

    #[test]
    fn grava_e_le_de_volta() {
        let pasta = std::env::temp_dir().join(format!("morune-ordem-{}", std::process::id()));
        std::fs::create_dir_all(&pasta).unwrap();
        let mut ordem = OrdemCurtidas::default();
        ordem.mover(&ids(&["b", "a"]), &TrackId::spotify("a"), 0);
        gravar(&pasta, &ordem);
        assert_eq!(ler(&pasta), ordem);
        let _ = std::fs::remove_dir_all(&pasta);
    }
}
