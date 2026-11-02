//! Sonda: as categorias da página de busca do Spotify ("Navegar por tudo").
//!
//! A tela de busca do Morune ficava vazia antes de digitar. O player web
//! preenche a dele com a consulta persistida `browseAll`; esta sonda grava a
//! resposta crua para escrever o parser contra ela, e nao contra suposicao.
//!
//! ```text
//! cargo run -p morune-desktop --example sonda-navegar
//! ```
//!
//! Usa a sessao guardada no cofre do Windows. Nao toca audio nem escreve nada
//! na conta. Saida em `bench-out/sonda/`, fora do git.

use std::sync::Arc;

use morune_spotify::SpotifyBackend;

/// Hash de `browseAll` no `xpui-routes-search.2aa27db9.js`, 02/11/2026.
const HASH_BROWSE_ALL: &str = "dbd8b55e09a58afc52eab438bc228ba28fd72ac2f2148c6c26354980e4579001";
/// Hash de `browsePage` no `browse-v2.e9ddc72e.js`, 02/11/2026.
const HASH_BROWSE_PAGE: &str = "f5c4e6d668f5716464a231c1cc8b22c1cbf6ad68b09929fd7de813a30581298b";
const SAIDA: &str = "bench-out/sonda";

fn main() {
    let credenciais = morune_storage::platform_store();
    let cache_audio = std::env::temp_dir().join("morune-sonda-navegar");
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
    std::fs::create_dir_all(SAIDA).expect("pasta de saida");

    // O mesmo caminho que a tela usa, para conferir o parser contra a conta.
    match backend.block_on(backend.library().categorias()) {
        Ok(cats) => println!(
            "categorias pelo parser: {} (primeira: {:?})",
            cats.len(),
            cats.first().map(|c| &c.titulo)
        ),
        Err(e) => println!("categorias FALHOU: {e}"),
    }
    match backend.block_on(
        backend
            .library()
            .categoria("spotify:page:0JQ5DAqbMKFDXXwE9BDJAr"),
    ) {
        Ok((titulo, secoes)) => println!(
            "categoria pelo parser: {titulo}, {} prateleiras, {} itens",
            secoes.len(),
            secoes.iter().map(|s| s.items.len()).sum::<usize>()
        ),
        Err(e) => println!("categoria FALHOU: {e}"),
    }

    let variaveis = serde_json::json!({
        "pagePagination": { "offset": 0, "limit": 10 },
        "sectionPagination": { "offset": 0, "limit": 99 },
        "browseEndUserIntegration": "INTEGRATION_WEB_PLAYER",
    });
    match backend.sonda_pathfinder("browseAll", HASH_BROWSE_ALL, &variaveis) {
        Ok(valor) => {
            let caminho = format!("{SAIDA}/browseAll.json");
            let texto = serde_json::to_string_pretty(&valor).unwrap();
            std::fs::write(&caminho, &texto).expect("gravar");
            println!("browseAll OK, {} bytes -> {caminho}", texto.len());
        }
        Err(e) => println!("browseAll FALHOU: {e}"),
    }

    // Uma categoria aberta (Rock), para ver as prateleiras de playlists.
    let pagina = serde_json::json!({
        "pagePagination": { "offset": 0, "limit": 10 },
        "sectionPagination": { "offset": 0, "limit": 10 },
        "uri": "spotify:page:0JQ5DAqbMKFDXXwE9BDJAr",
        "browseEndUserIntegration": "INTEGRATION_WEB_PLAYER",
        "includeEpisodeContentRatingsV2": false,
    });
    match backend.sonda_pathfinder("browsePage", HASH_BROWSE_PAGE, &pagina) {
        Ok(valor) => {
            let caminho = format!("{SAIDA}/browsePage-rock.json");
            let texto = serde_json::to_string_pretty(&valor).unwrap();
            std::fs::write(&caminho, &texto).expect("gravar");
            println!("browsePage OK, {} bytes -> {caminho}", texto.len());
        }
        Err(e) => println!("browsePage FALHOU: {e}"),
    }
}
