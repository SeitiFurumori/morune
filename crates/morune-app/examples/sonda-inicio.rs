//! Sonda: o que o Spotify devolve para montar a tela inicial dele.
//!
//! O Inicio do Morune so mostrava as playlists que a conta segue, separadas por
//! tipo -- e "Feito para voce" parava em cinco. O player web monta o Inicio
//! com a consulta persistida `home`, que traz as secoes prontas. Esta sonda
//! grava a resposta crua para escrever o parser contra ela, e nao contra
//! suposicao.
//!
//! ```text
//! cargo run -p morune-app --example sonda-inicio
//! ```
//!
//! Usa a sessao guardada no cofre do Windows. Nao toca audio nem escreve nada
//! na conta. Saida em `bench-out/sonda/`, fora do git.

use std::sync::Arc;

use morune_spotify::SpotifyBackend;

const HASH_HOME: &str = "76243c78b0e20ecdbe41b794dec8cbe73f75e585b0a7201b8d2e84578412847a";
const HASH_LIBRARY_V3: &str = "390c78e5b951029bad359785e69b07b536a509c581cbcd0aded5e5067f187455";
const SAIDA: &str = "bench-out/sonda";

fn main() {
    let credenciais = morune_storage::platform_store();
    let cache_audio = std::env::temp_dir().join("morune-sonda-inicio");
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
    match backend.block_on(backend.library().home_feed()) {
        Ok(secoes) => {
            for s in &secoes {
                println!("secao: {} ({} itens)", s.title, s.items.len());
            }
        }
        Err(e) => println!("home_feed FALHOU: {e}"),
    }

    let consultas = [
        (
            "home",
            HASH_HOME,
            serde_json::json!({
                "homeEndUserIntegration": "INTEGRATION_WEB_PLAYER",
                "timeZone": "America/Sao_Paulo",
                "sp_t": "",
                "facet": "",
                "sectionItemsLimit": 20,
                "includeEpisodeContentRatingsV2": false,
            }),
        ),
        (
            "libraryV3",
            HASH_LIBRARY_V3,
            serde_json::json!({
                "filters": [],
                "order": null,
                "textFilter": null,
                "features": ["LikedSongs", "YourEpisodesV2"],
                "limit": 50,
                "offset": 0,
                "flatten": false,
                "expandedFolders": [],
                "folderUri": null,
                "includeFoldersWhenFlattening": true,
            }),
        ),
    ];

    for (operacao, hash, variaveis) in consultas {
        match backend.sonda_pathfinder(operacao, hash, &variaveis) {
            Ok(valor) => {
                let caminho = format!("{SAIDA}/{operacao}.json");
                let texto = serde_json::to_string_pretty(&valor).unwrap();
                std::fs::write(&caminho, &texto).expect("gravar");
                println!("{operacao:<12} OK, {} bytes -> {caminho}", texto.len());
            }
            Err(e) => println!("{operacao:<12} FALHOU: {e}"),
        }
    }
}
