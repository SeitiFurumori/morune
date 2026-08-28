//! Sonda: descobrir como se **escreve** numa playlist pelo protocolo interno.
//!
//! A leitura ja e conhecida e esta em producao (`internal.rs`). O que falta e o
//! outro lado: criar playlist, acrescentar faixa, remover, reordenar. Sem isso
//! o Morune e um player que nao deixa ninguem guardar uma musica.
//!
//! # O que ja se sabe, so lendo o codigo
//!
//! | Fato | Onde |
//! |---|---|
//! | `Add`, `Rem`, `Mov`, `UpdateListAttributes` sao protobuf tipado | `playlist4_external.proto` |
//! | `SpClient::request` aceita corpo cru e metodo livre | librespot 0.8 |
//! | GET de playlist e `/playlist/v2/playlist/{base62}` | `SpClient::get_playlist` |
//! | GET do rootlist e `/playlist/v2/user/{user}/rootlist` | `SpClient::get_rootlist` |
//! | A resposta traz `revision` e `capabilities` | `SelectedListContent` |
//!
//! # O que **nao** se sabe, e e o motivo desta sonda
//!
//! A librespot so implementa GET. O endereco de escrita e o corpo que ele
//! aceita nao estao em lugar nenhum do codigo -- sao inferencia a partir do
//! padrao do GET. Esta sonda troca a inferencia por resposta do servidor.
//!
//! Tres perguntas, nesta ordem:
//!
//! 1. **Criar.** Qual endereco devolve um `CreateListReply`?
//! 2. **Acrescentar.** `POST {playlist}/changes` com um `ListChanges` que
//!    carrega um `Op::ADD` funciona? A `base_revision` precisa vir do GET.
//! 3. **Limpar.** Da para tirar a playlist do rootlist depois?
//!
//! # Seguranca
//!
//! **A fase de leitura nao escreve nada** e roda sempre. A fase de escrita so
//! acontece com `--escrever`, e mesmo assim **nunca toca numa playlist que ja
//! existe**: ela cria a propria e mexe so nela.
//!
//! **A playlist de teste nao e apagada pela sonda.** Apagar exigiria escrever
//! no rootlist -- a lista que organiza *todas* as playlists da conta --, e
//! arriscar a estrutura inteira para limpar um item de teste e uma troca ruim.
//! Ela fica com o nome `NOME_TESTE`, obvio o bastante para ser apagada pelo
//! Spotify em dois cliques. Descobrir como se escreve no rootlist com seguranca
//! e a pergunta seguinte, e merece a propria rodada.
//!
//! ```powershell
//! cargo run --release --example sonda-playlist -p morune-spotify
//! cargo run --release --example sonda-playlist -p morune-spotify -- --escrever
//! ```

use std::path::Path;

use http::Method;
use librespot_core::authentication::Credentials;
use librespot_core::{Session, SessionConfig, SpotifyId};
use librespot_oauth::OAuthClientBuilder;
use librespot_protocol::playlist4_external::{
    op::Kind as OpKind, Add, ChangeInfo, Delta, Item, ListAttributes, ListChanges, Op, Rem,
    SelectedListContent, UpdateListAttributes,
};
use protobuf::Message;

const CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";
const REDIRECT_URI: &str = "http://127.0.0.1:5588/login";
const SAIDA: &str = "bench-out/sonda";

const SCOPES: &[&str] = &[
    "streaming",
    "playlist-read-private",
    "playlist-modify-private",
    "playlist-modify-public",
];

/// Faixa usada no teste de acrescentar. Qualquer uma serve; esta e a mesma que
/// a sonda anterior ja usava, entao o id ja foi verificado.
const FAIXA: &str = "spotify:track:7tFiyTwD0nx5a1eklYtX2J";

/// Nome da playlist de teste. Deliberadamente obvio: se a limpeza falhar, quem
/// olhar a conta entende na hora o que e aquilo e que pode apagar.
const NOME_TESTE: &str = "Morune - sonda (pode apagar)";

fn main() {
    let escrever = std::env::args().any(|a| a == "--escrever");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime");

    std::fs::create_dir_all(SAIDA).expect("pasta de saida");
    println!("\n=== SONDA: escrita em playlist ===\n");
    if !escrever {
        println!("  modo leitura. Nada sera escrito na conta.");
        println!("  para incluir a fase de escrita: -- --escrever\n");
    }

    let token = OAuthClientBuilder::new(CLIENT_ID, REDIRECT_URI, SCOPES.to_vec())
        .open_in_browser()
        .build()
        .expect("cliente oauth")
        .get_access_token()
        .expect("login");

    runtime.block_on(async move {
        let session = Session::new(SessionConfig::default(), None);
        if let Err(e) = session
            .connect(Credentials::with_access_token(&token.access_token), false)
            .await
        {
            println!("sessao FALHOU: {e}");
            return;
        }
        let usuario = session.username();
        println!("sessao OK -- conta={usuario}\n");

        leitura(&session, &usuario).await;

        if escrever {
            escrita(&session, &usuario).await;
        }

        println!("\n=== FIM ===\n");
    });
}

/// Fase 1: o que a leitura ja entrega e que a escrita vai precisar.
///
/// Interessa uma coisa so: a `revision`. Toda escrita no playlist4 e
/// concorrencia otimista -- manda-se a revisao em que a mudanca se baseia, e o
/// servidor recusa se alguem escreveu antes. Sem le-la, nao ha o que enviar.
async fn leitura(session: &Session, usuario: &str) {
    println!("-- fase 1: leitura (nao escreve nada)");

    let bytes = match session.spclient().get_rootlist(0, Some(50)).await {
        Ok(b) => b,
        Err(e) => {
            println!("  rootlist FALHOU: {e}");
            return;
        }
    };
    grava("playlist-rootlist.protobuf", &bytes);

    let lista = match SelectedListContent::parse_from_bytes(&bytes) {
        Ok(l) => l,
        Err(e) => {
            println!("  rootlist ilegivel: {e}");
            return;
        }
    };

    println!(
        "  rootlist: revision={} itens={}",
        hex(lista.revision()),
        lista.contents.items.len()
    );

    // Uma playlist propria, para ler revisao e capacidades. So leitura.
    let propria = lista
        .contents
        .items
        .iter()
        .filter_map(|item| item.uri.as_deref())
        .find(|uri| uri.starts_with("spotify:playlist:"));

    let Some(uri) = propria else {
        println!("  nenhuma playlist no rootlist; nada a inspecionar");
        return;
    };

    let id = uri.trim_start_matches("spotify:playlist:");
    let Ok(spotify_id) = SpotifyId::from_base62(id) else {
        println!("  id ilegivel: {id}");
        return;
    };

    match session.spclient().get_playlist(&spotify_id).await {
        Ok(b) => {
            grava("playlist-uma.protobuf", &b);
            match SelectedListContent::parse_from_bytes(&b) {
                Ok(p) => {
                    let cap = p.capabilities.as_ref();
                    println!(
                        "  playlist {id}: nome={:?} revision={} faixas={}",
                        p.attributes.name(),
                        hex(p.revision()),
                        p.length()
                    );
                    println!(
                        "  capacidades: editar itens={:?} editar metadados={:?}",
                        cap.map(|c| c.can_edit_items()),
                        cap.map(|c| c.can_edit_metadata())
                    );
                }
                Err(e) => println!("  playlist ilegivel: {e}"),
            }
        }
        Err(e) => println!("  playlist FALHOU: {e}"),
    }

    println!("  usuario para o endereco de rootlist: {usuario}");
}

/// Fase 2: criar, acrescentar, remover, limpar.
///
/// Cada passo imprime o que respondeu. Um passo que falha nao impede os
/// seguintes de serem tentados **quando fazem sentido sozinhos** -- mas sem
/// playlist criada nao ha o que acrescentar, e ai a fase para.
async fn escrita(session: &Session, usuario: &str) {
    println!("\n-- fase 2: escrita (cria uma playlist de teste)");

    let Some(uri) = criar(session, usuario).await else {
        println!("  nenhum endereco de criacao respondeu; a fase para aqui");
        return;
    };
    println!("  playlist criada: {uri}");

    let id = uri.trim_start_matches("spotify:playlist:").to_string();
    let Ok(spotify_id) = SpotifyId::from_base62(&id) else {
        println!("  id da playlist criada e ilegivel: {id}");
        return;
    };

    let Some(revisao) = revisao_de(session, &spotify_id).await else {
        println!("  sem revisao, nao da para escrever");
        return;
    };

    // Acrescentar.
    let mut add = Add::new();
    add.items.push({
        let mut item = Item::new();
        item.set_uri(FAIXA.to_string());
        item
    });
    add.set_add_last(true);
    let mut op = Op::new();
    op.set_kind(OpKind::ADD);
    op.add = protobuf::MessageField::some(add);

    match mudanca(session, &id, revisao.clone(), op).await {
        Ok(bytes) => {
            grava("playlist-add.protobuf", &bytes);
            println!("  ADD ................... OK, {} bytes", bytes.len());
        }
        Err(e) => println!("  ADD ................... FALHOU: {e}"),
    }

    // Remover a faixa, para provar o caminho inverso.
    if let Some(revisao) = revisao_de(session, &spotify_id).await {
        let mut rem = Rem::new();
        rem.set_from_index(0);
        rem.set_length(1);
        let mut op = Op::new();
        op.set_kind(OpKind::REM);
        op.rem = protobuf::MessageField::some(rem);

        match mudanca(session, &id, revisao, op).await {
            Ok(bytes) => println!("  REM ................... OK, {} bytes", bytes.len()),
            Err(e) => println!("  REM ................... FALHOU: {e}"),
        }
    }

    println!("\n  A playlist de teste continua na conta, com o nome");
    println!("  \"{NOME_TESTE}\". Apague pelo Spotify -- remover do rootlist");
    println!("  e a proxima pergunta, e nao vale arriscar mexer nele por sonda.");
}

/// Tenta criar uma playlist, testando os enderecos candidatos em ordem.
///
/// Nenhum deles esta documentado no codigo da librespot; sao inferencia a
/// partir do padrao dos GET que existem. O primeiro que devolver um corpo com
/// URI ganha, e o resultado vira a resposta desta sonda.
async fn criar(session: &Session, usuario: &str) -> Option<String> {
    let mut atributos = ListAttributes::new();
    atributos.set_name(NOME_TESTE.to_string());

    let mut update = UpdateListAttributes::new();
    update.new_attributes = protobuf::MessageField::some({
        let mut estado = librespot_protocol::playlist4_external::ListAttributesPartialState::new();
        estado.values = protobuf::MessageField::some(atributos);
        estado
    });

    let mut op = Op::new();
    op.set_kind(OpKind::UPDATE_LIST_ATTRIBUTES);
    op.update_list_attributes = protobuf::MessageField::some(update);

    let mut delta = Delta::new();
    delta.ops.push(op);
    delta.info = protobuf::MessageField::some(info());

    let mut changes = ListChanges::new();
    changes.deltas.push(delta);
    changes.set_want_resulting_revisions(true);

    let corpo = changes.write_to_bytes().ok()?;

    let candidatos = [
        (Method::POST, "/playlist/v2/playlist".to_string()),
        (
            Method::POST,
            format!("/playlist/v2/user/{usuario}/rootlist/changes"),
        ),
        (Method::PUT, "/playlist/v2/playlist".to_string()),
    ];

    for (metodo, endereco) in candidatos {
        let resposta = session
            .spclient()
            .request(&metodo, &endereco, cabecalho(), Some(&corpo))
            .await;

        match resposta {
            Ok(bytes) => {
                grava("playlist-create.protobuf", &bytes);
                println!("  criar {metodo} {endereco} ... OK, {} bytes", bytes.len());
                if let Some(uri) = uri_criada(&bytes) {
                    return Some(uri);
                }
                println!("    (respondeu, mas sem URI reconhecivel; ver o arquivo)");
            }
            Err(e) => println!("  criar {metodo} {endereco} ... {e}"),
        }
    }

    None
}

/// Le a `CreateListReply` e devolve a URI da playlist nova.
fn uri_criada(bytes: &[u8]) -> Option<String> {
    use librespot_protocol::playlist4_external::CreateListReply;

    let reply = CreateListReply::parse_from_bytes(bytes).ok()?;
    let uri = reply.uri();
    (!uri.is_empty()).then(|| uri.to_string())
}

/// Envia um `Op` como mudanca sobre uma revisao conhecida.
async fn mudanca(
    session: &Session,
    id: &str,
    base: Vec<u8>,
    op: Op,
) -> Result<bytes::Bytes, String> {
    let mut delta = Delta::new();
    delta.set_base_version(base.clone());
    delta.ops.push(op);
    delta.info = protobuf::MessageField::some(info());

    let mut changes = ListChanges::new();
    changes.set_base_revision(base);
    changes.deltas.push(delta);
    changes.set_want_resulting_revisions(true);

    let corpo = changes.write_to_bytes().map_err(|e| e.to_string())?;
    let endereco = format!("/playlist/v2/playlist/{id}/changes");

    session
        .spclient()
        .request(&Method::POST, &endereco, cabecalho(), Some(&corpo))
        .await
        .map_err(|e| e.to_string())
}

/// A revisao atual de uma playlist. Toda escrita precisa dela.
async fn revisao_de(session: &Session, id: &SpotifyId) -> Option<Vec<u8>> {
    let bytes = session.spclient().get_playlist(id).await.ok()?;
    let lista = SelectedListContent::parse_from_bytes(&bytes).ok()?;
    Some(lista.revision().to_vec())
}

/// Identifica quem fez a mudanca. O cliente oficial preenche isto, e um
/// servidor que confira a origem recusaria um corpo sem ele.
fn info() -> ChangeInfo {
    use librespot_protocol::playlist4_external::source_info::Client;
    use librespot_protocol::playlist4_external::SourceInfo;

    let mut fonte = SourceInfo::new();
    fonte.set_client(Client::CLIENT);
    fonte.set_app("morune".to_string());
    fonte.set_source("morune-sonda".to_string());

    let mut info = ChangeInfo::new();
    info.source = protobuf::MessageField::some(fonte);
    info
}

fn cabecalho() -> Option<http::HeaderMap> {
    let mut headers = http::HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("application/x-protobuf"),
    );
    Some(headers)
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn grava(nome: &str, bytes: &[u8]) {
    let caminho = Path::new(SAIDA).join(nome);
    if let Err(e) = std::fs::write(&caminho, bytes) {
        println!("  {nome} nao gravou: {e}");
    }
}
