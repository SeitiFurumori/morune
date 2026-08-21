//! Sonda: despeja em disco as respostas do Spotify que o Morune vai ter de ler.
//!
//! As rodadas anteriores responderam **onde** buscar. Esta responde **em que
//! formato**, porque escrever parser contra amostra de 260 bytes e o mesmo que
//! adivinhar -- e o projeto nao aceita JSON adivinhado.
//!
//! # O que ja se sabe
//!
//! | Caminho | Resultado |
//! |---|---|
//! | `api.spotify.com`, token do OAuth ou do login5 | 429 |
//! | `hm://keymaster/token/authenticated` | 403 |
//! | busca por `searchview` (mercury e spclient) | 404 |
//! | **`pathfinder`, consulta persistida** | **OK** |
//! | `get_rootlist`, metadado, radio, colecao por mercury | OK |
//!
//! # O que sai daqui
//!
//! Arquivos em `bench-out/sonda/`, que nao entra no git. Cada um e a resposta
//! crua de um caminho que a reescrita do catalogo vai precisar ler.
//!
//! ```powershell
//! cargo run --release --example sonda -p morune-spotify
//! ```

use std::path::Path;

use bytes::Bytes;
use http::header::{HeaderName, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use http::{Method, Request};
use librespot_core::authentication::Credentials;
use librespot_core::http_client::HttpClient;
use librespot_core::{Session, SessionConfig, SpotifyUri};
use librespot_oauth::OAuthClientBuilder;

const CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";
const REDIRECT_URI: &str = "http://127.0.0.1:5588/login";
const SAIDA: &str = "bench-out/sonda";

const SCOPES: &[&str] = &[
    "streaming",
    "user-read-email",
    "user-read-private",
    "user-library-read",
    "playlist-read-private",
    "playlist-read-collaborative",
    "user-top-read",
    "user-read-recently-played",
    "user-follow-read",
];

/// Hash da consulta `searchDesktop`. Ver a divida no `HANDOFF.md`: ele
/// acompanha a versao do player web e muda sem aviso.
const HASH_BUSCA: &str = "d9f785900f0710b31c07818d617f4f7600c1e21217e80f5b043d1e78d74e6026";

const FAIXA: &str = "spotify:track:7tFiyTwD0nx5a1eklYtX2J";

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime");

    std::fs::create_dir_all(SAIDA).expect("pasta de saida");
    println!("\n=== SONDA: despejo de formatos ===\n");

    let token = OAuthClientBuilder::new(CLIENT_ID, REDIRECT_URI, SCOPES.to_vec())
        .open_in_browser()
        .build()
        .expect("cliente oauth")
        .get_access_token()
        .expect("login");

    runtime.block_on(async move {
        let session = Session::new(SessionConfig::default(), None);
        if let Err(e) = session.connect(Credentials::with_access_token(&token.access_token), false).await
        {
            println!("sessao FALHOU: {e}");
            return;
        }
        let usuario = session.username();
        let pais = session.country();
        println!("sessao OK -- conta={usuario} pais={pais}\n");

        let bearer = match session.login5().auth_token().await {
            Ok(t) => t.access_token,
            Err(e) => {
                println!("login5 FALHOU: {e}");
                return;
            }
        };
        let client_token = session.spclient().client_token().await.unwrap_or_default();
        let http = HttpClient::new(None);

        // A resposta da busca, inteira. E o parser mais complicado da
        // reescrita: um resultado traz faixa, album, artista e playlist, cada
        // um embrulhado de um jeito diferente.
        let corpo = format!(
            r#"{{"operationName":"searchDesktop","variables":{{"searchTerm":"queen","offset":0,"limit":10,"numberOfTopResults":5,"includeAudiobooks":false}},"extensions":{{"persistedQuery":{{"version":1,"sha256Hash":"{HASH_BUSCA}"}}}}}}"#
        );
        pathfinder(&http, &bearer, &client_token, "busca", &corpo).await;

        // Nome de exibicao e avatar, que sairam da barra lateral junto com o
        // `/v1/me`.
        match session.spclient().get_user_profile(&usuario, Some(5), Some(5)).await {
            Ok(b) => grava("perfil.json", &b),
            Err(e) => println!("  perfil ................ FALHOU: {e}"),
        }

        // O rootlist responde, mas a leitura atual quebra com "ID cannot be
        // parsed" -- provavelmente uma URI que nao e de playlist no meio da
        // lista. Sem o byte cru nao da para saber qual.
        match session.spclient().get_rootlist(0, Some(200)).await {
            Ok(b) => grava("rootlist.protobuf", &b),
            Err(e) => println!("  rootlist .............. FALHOU: {e}"),
        }

        // Curtidas e artistas seguidos.
        for (nome, uri) in [
            ("colecao-faixas.bin", format!("hm://collection/collection/{usuario}/?allowonlytracks=false")),
            ("colecao-artistas.bin", format!("hm://collection/artist/{usuario}/?allowonlytracks=false")),
        ] {
            match session.mercury().get(uri) {
                Ok(fut) => match fut.await {
                    Ok(r) => {
                        let mut todos = Vec::new();
                        for parte in &r.payload {
                            todos.extend_from_slice(parte);
                        }
                        grava(nome, &todos);
                    }
                    Err(e) => println!("  {nome:<22} FALHOU: {e}"),
                },
                Err(e) => println!("  {nome:<22} FALHOU: {e}"),
            }
        }

        // Metadado de faixa e radio, para fechar o mapa de formatos.
        if let Ok(uri) = SpotifyUri::from_uri(FAIXA) {
            match session.spclient().get_track_metadata(&uri).await {
                Ok(b) => grava("faixa.protobuf", &b),
                Err(e) => println!("  faixa ................. FALHOU: {e}"),
            }
            match session.spclient().get_radio_for_track(&uri).await {
                Ok(b) => grava("radio.json", &b),
                Err(e) => println!("  radio ................. FALHOU: {e}"),
            }
        }

        println!("\n=== FIM -- arquivos em {SAIDA}/ ===\n");
    });
}

async fn pathfinder(http: &HttpClient, bearer: &str, client_token: &str, nome: &str, corpo: &str) {
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri("https://api-partner.spotify.com/pathfinder/v1/query")
        .header(AUTHORIZATION, format!("Bearer {bearer}"))
        .header(CONTENT_TYPE, "application/json")
        .header(ACCEPT, "application/json");

    if !client_token.is_empty() {
        builder = builder.header(HeaderName::from_static("client-token"), client_token);
    }

    match http
        .request_body(
            builder
                .body(Bytes::copy_from_slice(corpo.as_bytes()))
                .unwrap(),
        )
        .await
    {
        Ok(body) => grava(&format!("{nome}.json"), &body),
        Err(e) => println!("  {nome:<22} FALHOU: {e}"),
    }
}

fn grava(nome: &str, bytes: &[u8]) {
    let caminho = Path::new(SAIDA).join(nome);
    match std::fs::write(&caminho, bytes) {
        Ok(()) => println!("  {nome:<22} OK, {} bytes", bytes.len()),
        Err(e) => println!("  {nome:<22} nao gravou: {e}"),
    }
}
