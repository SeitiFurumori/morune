//! Sonda: "Os fãs também curtem" na página de artista.
//!
//! Pede alguns artistas pelo mesmo caminho da tela e conta quantos
//! relacionados vieram. Prateleira vazia aqui é prateleira vazia no app.
//!
//! ```text
//! cargo run -p morune-desktop --example sonda-artista [id ...]
//! ```
//!
//! Usa a sessao guardada no cofre do Windows. Nao toca audio nem escreve nada
//! na conta.

use std::sync::Arc;

use morune_core::model::ArtistId;
use morune_spotify::SpotifyBackend;

/// Radiohead, Daft Punk e Hikaru Utada: artistas grandes, que sempre têm
/// relacionados no player web.
const PADRAO: &[&str] = &[
    "4Z8W4fKeB5YxbusRsdQVPb",
    "4tZwfgrHOc3mvqYlEYSvVi",
    "7lbSsjYACZHn1MSDXPxNF2",
];

fn main() {
    let credenciais = morune_storage::platform_store();
    let cache_audio = std::env::temp_dir().join("morune-sonda-artista");
    let backend = SpotifyBackend::new(
        Arc::from(credenciais),
        morune_core::playback::AudioSettings::default(),
        &cache_audio,
    )
    .expect("backend do Spotify");

    let perfil = backend
        .block_on(backend.authenticator().restore())
        .expect("restaurar a sessao guardada");
    if perfil.is_none() {
        eprintln!("Nao ha sessao guardada no cofre do Windows.");
        std::process::exit(1);
    }

    let pedidos: Vec<String> = std::env::args().skip(1).collect();
    let ids: Vec<&str> = if pedidos.is_empty() {
        PADRAO.to_vec()
    } else {
        pedidos.iter().map(String::as_str).collect()
    };

    let catalogo = backend.catalog();
    let mut vazios = 0;
    for id in ids {
        match backend.block_on(catalogo.artist(&ArtistId::spotify(id))) {
            Ok(artista) => {
                let nomes: Vec<&str> = artista
                    .related
                    .iter()
                    .take(4)
                    .map(|a| a.name.as_ref())
                    .collect();
                println!(
                    "{}: {} relacionados {:?}",
                    artista.name,
                    artista.related.len(),
                    nomes
                );
                if artista.related.is_empty() {
                    vazios += 1;
                }
            }
            Err(e) => {
                println!("{id}: FALHOU: {e}");
                vazios += 1;
            }
        }
    }
    if vazios > 0 {
        std::process::exit(2);
    }
}
