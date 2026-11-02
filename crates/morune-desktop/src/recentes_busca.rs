//! O que a pessoa abriu pela busca, com capa, para a busca vazia mostrar.
//!
//! As palavras buscadas continuam no `history.toml`; este arquivo guarda o
//! cartao do que foi clicado depois (artista, album, playlist ou musica). Fica
//! num arquivo proprio, e nao no historico, porque uma versao antiga do app
//! recusaria um `history.toml` com campo que ela nao conhece.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::browse::Card;

/// Quantos cartoes a busca guarda.
pub const MAXIMO: usize = 12;

#[derive(Debug, Serialize, Deserialize)]
struct Recente {
    tag: String,
    title: String,
    subtitle: String,
    cover: String,
}

fn arquivo(pasta: &Path) -> PathBuf {
    pasta.join("recentes-busca.json")
}

/// Os cartoes gravados. Arquivo ausente ou estragado vira lista vazia.
pub fn ler(pasta: &Path) -> Vec<Card> {
    let Ok(texto) = std::fs::read_to_string(arquivo(pasta)) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<Recente>>(&texto)
        .unwrap_or_default()
        .into_iter()
        .take(MAXIMO)
        .map(|r| Card {
            tag: r.tag,
            title: r.title,
            subtitle: r.subtitle,
            cover: r.cover,
            cover_path: None,
        })
        .collect()
}

pub fn gravar(pasta: &Path, cards: &[Card]) {
    let lista: Vec<Recente> = cards
        .iter()
        .map(|c| Recente {
            tag: c.tag.clone(),
            title: c.title.clone(),
            subtitle: c.subtitle.clone(),
            cover: c.cover.clone(),
        })
        .collect();
    let gravado = serde_json::to_string_pretty(&lista)
        .map_err(std::io::Error::other)
        .and_then(|texto| std::fs::write(arquivo(pasta), texto));
    if let Err(error) = gravado {
        tracing::warn!(%error, "buscas recentes nao gravadas");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grava_e_le_de_volta() {
        let pasta = std::env::temp_dir().join(format!("otomori-recentes-{}", std::process::id()));
        std::fs::create_dir_all(&pasta).unwrap();
        let cards = vec![Card {
            tag: "artist/spotify:abc".into(),
            title: "Wham!".into(),
            subtitle: "Artista".into(),
            cover: "https://i.scdn.co/image/x".into(),
            cover_path: None,
        }];
        gravar(&pasta, &cards);
        let lidos = ler(&pasta);
        assert_eq!(lidos.len(), 1);
        assert_eq!(lidos[0].title, "Wham!");
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn arquivo_estragado_vira_lista_vazia() {
        let pasta =
            std::env::temp_dir().join(format!("otomori-recentes-ruim-{}", std::process::id()));
        std::fs::create_dir_all(&pasta).unwrap();
        std::fs::write(arquivo(&pasta), "{nao e json").unwrap();
        assert!(ler(&pasta).is_empty());
        let _ = std::fs::remove_dir_all(&pasta);
    }
}
