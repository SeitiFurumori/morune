//! Build do Morune com tela web: o Tauri embute a pasta `ui/dist`, e aqui
//! entra tambem o rotulo de versao que o motor mostra e o log registra.
use std::path::Path;

fn main() {
    embed_release_tag();
    tauri_build::build();
}

fn versao_visivel() -> String {
    std::env::var("MORUNE_RELEASE_TAG")
        .ok()
        .filter(|tag| !tag.trim().is_empty())
        .unwrap_or_else(|| format!("v{}-dev.{}", env!("CARGO_PKG_VERSION"), commit_hash()))
}

fn embed_release_tag() {
    println!("cargo:rerun-if-env-changed=MORUNE_RELEASE_TAG");

    let local = std::env::var("MORUNE_RELEASE_TAG")
        .ok()
        .filter(|tag| !tag.trim().is_empty())
        .is_none();
    let tag = versao_visivel();

    println!("cargo:rustc-env=MORUNE_RELEASE={tag}");
    // De onde a tag veio. O teste do rotulo precisa saber: exigir
    // pre-lancamento de toda tag reprovaria a primeira release final publicada.
    println!(
        "cargo:rustc-env=MORUNE_RELEASE_SOURCE={}",
        if local { "local" } else { "workflow" }
    );
}

/// Hash curto do commit de onde este build saiu.
///
/// `local` quando nao ha repositorio -- um tarball do codigo-fonte compila do
/// mesmo jeito, e travar o build por causa de um rotulo seria trocar uma
/// conveniencia por um erro. Nao carrega marca de arvore suja: o interesse aqui
/// e "de qual commit isto saiu", e um `-sujo` grudado no identificador entraria
/// na comparacao de versao sem significar nada para ela.
fn commit_hash() -> String {
    // O hash muda a cada commit sem que nenhum arquivo do crate mude, entao o
    // cargo precisa ser avisado de onde olhar.
    //
    // **`HEAD` sozinho nao bastava, e por isso o rotulo ficou parado.** Num
    // commit comum o `.git/HEAD` nao e reescrito: ele continua com o mesmo
    // texto, `ref: refs/heads/main`. Quem muda e o arquivo da ref apontada. Na
    // maquina onde isto foi encontrado, `.git/HEAD` estava com data de 28/08 e
    // `.git/refs/heads/main` com a data do commit daquele minuto -- ou seja, o
    // build nao era refeito havia onze dias e a tela de Configuracoes mostrava
    // um commit antigo. E justamente a protecao que existe para saber qual
    // build esta instalado, quebrada em silencio.
    //
    // Agora vao os tres: `HEAD` (cobre troca de branch), a ref que ele aponta
    // (cobre commit no mesmo branch) e o `packed-refs` (cobre a ref empacotada,
    // caso em que o arquivo solto nem existe).
    let mut vigiar = vec![
        "../../.git/HEAD".to_string(),
        "../../.git/packed-refs".to_string(),
    ];
    if let Ok(head) = std::fs::read_to_string("../../.git/HEAD") {
        if let Some(ref_rel) = head.trim().strip_prefix("ref: ") {
            vigiar.push(format!("../../.git/{ref_rel}"));
        }
    }
    for caminho in vigiar {
        if Path::new(&caminho).exists() {
            println!("cargo:rerun-if-changed={caminho}");
        }
    }

    std::process::Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .output()
        .ok()
        .filter(|saida| saida.status.success())
        .and_then(|saida| String::from_utf8(saida.stdout).ok())
        .map(|hash| hash.trim().to_string())
        .filter(|hash| !hash.is_empty() && hash.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or_else(|| "local".into())
}

