//! Mede o cache de playlist com a conta real, sem abrir a interface.
//!
//! O cache do `internal.rs` tem teste de unidade, mas teste de unidade nao
//! responde a pergunta que interessa: **da para sentir?** Aqui a medida e feita
//! de ponta a ponta, pelo mesmo `Catalog` que a tela usa, com a sessao que ja
//! esta guardada no cofre do Windows -- sem login interativo, sem tocar audio e
//! sem escrever nada na conta.
//!
//! O roteiro imita o gesto que motivou o cache: abrir uma playlist, ir para
//! outra, e voltar para a primeira.
//!
//! ```text
//! . .\tools\env.ps1
//! cargo run -p morune-app --example cache-playlist
//! ```
//!
//! Cada abertura sao as duas chamadas que a tela faz: `playlist` (nome e
//! tamanho) e `playlist_tracks` (a primeira pagina). A segunda visita a A e a
//! que deve cair no cache.

use std::sync::Arc;
use std::time::{Duration, Instant};

use morune_core::model::PlaylistId;
use morune_spotify::SpotifyBackend;

/// Tamanho da pagina, igual ao que a tela pede ao abrir uma lista.
const PAGINA: u32 = 50;

fn main() {
    let credenciais = morune_storage::platform_store();
    let cache_audio = std::env::temp_dir().join("morune-cache-playlist");

    let backend = SpotifyBackend::new(
        Arc::from(credenciais),
        morune_core::playback::AudioSettings::default(),
        &cache_audio,
    )
    .expect("backend do Spotify");

    let perfil = backend
        .block_on(backend.authenticator().restore())
        .expect("restaurar a sessao guardada");

    let Some(perfil) = perfil else {
        eprintln!(
            "Nao ha sessao guardada no cofre do Windows. Entre no Morune uma vez e rode de novo."
        );
        std::process::exit(1);
    };
    println!(
        "conta: {}\n",
        perfil.display_name.as_deref().unwrap_or(&perfil.id)
    );

    let catalogo = backend.catalog();
    let biblioteca = backend.library();

    // Duas playlists da propria conta: a medida precisa de uma segunda lista
    // para provar que o cache atravessa a navegacao, e nao so repete a ultima.
    let salvas = backend
        .block_on(biblioteca.saved_playlists(0, 10))
        .expect("playlists da conta");

    let candidatas: Vec<_> = salvas
        .items
        .into_iter()
        .filter(|p| p.total_tracks.is_none_or(|total| total > 0))
        .take(2)
        .collect();

    let [a, b] = candidatas.as_slice() else {
        eprintln!("Preciso de duas playlists com faixas na conta para medir.");
        std::process::exit(1);
    };

    println!("A: {} ({:?} faixas)", a.name, a.total_tracks);
    println!("B: {} ({:?} faixas)\n", b.name, b.total_tracks);

    let abrir = |id: &PlaylistId, rotulo: &str| -> Duration {
        let inicio = Instant::now();
        let cabecalho = backend.block_on(catalogo.playlist(id));
        let meio = inicio.elapsed();
        let faixas = backend.block_on(catalogo.playlist_tracks(id, 0, PAGINA));
        let total = inicio.elapsed();

        match (cabecalho, faixas) {
            (Ok(_), Ok(pagina)) => println!(
                "{rotulo:<28} {:>7.0} ms   (cabecalho {:>6.0} ms, {} faixas)",
                total.as_secs_f64() * 1000.0,
                meio.as_secs_f64() * 1000.0,
                pagina.items.len()
            ),
            (Err(e), _) | (_, Err(e)) => println!("{rotulo:<28} FALHOU: {e}"),
        }
        total
    };

    println!("{:<28} {:>10}", "gesto", "tempo");
    println!("{}", "-".repeat(60));

    let primeira = abrir(&a.id, "abre A (rede)");
    abrir(&b.id, "abre B (rede)");
    let revisita = abrir(&a.id, "volta para A (cache?)");

    println!("{}", "-".repeat(60));
    let ganho = primeira.as_secs_f64() - revisita.as_secs_f64();
    println!(
        "\nvolta para A foi {:.0} ms {} que a primeira abertura.",
        ganho.abs() * 1000.0,
        if ganho >= 0.0 {
            "mais rapida"
        } else {
            "MAIS LENTA"
        }
    );
    println!(
        "Lembrando o que o cache guarda: a lista de ids e o cabecalho. O metadado\n\
         das faixas continua indo a rede toda vez, entao o ganho e a diferenca\n\
         entre as duas medidas, e nao a medida inteira."
    );
}
