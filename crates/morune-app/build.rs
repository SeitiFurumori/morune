use std::path::{Path, PathBuf};

fn main() {
    compile_ui();
    embed_release_tag();
    embed_brand_pixels();
    embed_exe_resources();
}

/// Grava no binario a tag do lancamento de onde ele saiu.
///
/// **Por que nao basta o `CARGO_PKG_VERSION`:** o `workspace.package.version`
/// fica parado em `0.1.0` enquanto as tags avancam em `v0.1.0-alpha.5`,
/// `alpha.6`, `alpha.7` -- o `release.yml` so exige que a parte numerica da tag
/// bata com a do Cargo.toml, e o sufixo de pre-lancamento nao chega ao codigo.
/// Sem isto, um alpha.5 instalado se apresentaria como `0.1.0`, que por semver
/// e *mais novo* que qualquer alpha, e a verificacao de atualizacao concluiria
/// que nao ha nada a fazer com tres lancamentos novos no ar.
///
/// Quem define a variavel e o workflow de publicacao. Num build local ela nao
/// existe, e ai a tag vira `v0.1.0-dev.<hash>`: um pre-lancamento, e nao a
/// final `v0.1.0`.
///
/// **Por que nao a versao do crate no build local:** `0.1.0` e uma versao
/// *final*, e por semver e mais nova que qualquer `0.1.0-alpha.N`. Um binario
/// compilado aqui se anunciava como a mais nova coisa que existe -- a
/// verificacao nunca encontrava lancamento algum, e a tela dizia "você já está
/// na versão mais recente" para sempre. Pior: todo build local se chamava
/// `0.1.0`, entao nada na tela nem no log distinguia o de hoje do de duas
/// semanas atras.
///
/// `dev` ordena **acima** de `alpha` (comparacao de texto, pelas regras do
/// semver), e isso e a verdade: um build local sai de commits que os alphas
/// publicados ainda nao tem. O efeito colateral desejado e que a verificacao
/// nunca oferece um lancamento publicado a quem esta num build local -- seria
/// um downgrade silencioso por cima do trabalho em andamento.
fn embed_release_tag() {
    println!("cargo:rerun-if-env-changed=MORUNE_RELEASE_TAG");

    let publicado = std::env::var("MORUNE_RELEASE_TAG")
        .ok()
        .filter(|tag| !tag.trim().is_empty());

    let local = publicado.is_none();
    let tag = publicado
        .unwrap_or_else(|| format!("v{}-dev.{}", env!("CARGO_PKG_VERSION"), commit_hash()));

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

fn compile_ui() {
    // O estilo "fluent" do Slint traz widgets prontos que nao usamos: a
    // interface e construida sobre primitivas para que todo pixel venha do
    // tema. Ainda assim e preciso escolher um estilo, e o escuro combina com o
    // tema embutido caso algum widget padrao apareca.
    let config = slint_build::CompilerConfiguration::new().with_style("fluent-dark".into());
    slint_build::compile_with_config("ui/app.slint", config)
        .expect("falha ao compilar a interface");
}

/// Diretorio do sistema de marca, relativo a este crate.
fn brand_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/brand")
}

/// Converte o simbolo de 32 px da marca em pixels crus dentro de `OUT_DIR`.
///
/// A bandeja do Windows quer um buffer RGBA, nao um arquivo. Decodificar aqui,
/// e nao no aplicativo, tira do caminho critico de startup uma decodificacao
/// que so serve para desenhar 32x32 pixels no canto da tela.
///
/// O icone da janela nao passa por aqui: quem o carrega e o proprio Slint, a
/// partir do `@image-url` em ui/app.slint.
fn embed_brand_pixels() {
    const SIZE: u32 = 32;

    let src = brand_dir().join(format!("morune-logo-system/morune-symbol-{SIZE}.png"));
    println!("cargo:rerun-if-changed={}", src.display());

    let (pixels, width, height) = decode_png(&src);
    assert_eq!(
        (width, height),
        (SIZE, SIZE),
        "{} deveria ser {SIZE}x{SIZE}, veio {width}x{height}",
        src.display()
    );

    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    std::fs::write(out.join("brand-tray.rgba"), pixels).expect("gravar pixels da bandeja");
}

/// Decodifica um PNG para RGBA de 8 bits.
fn decode_png(path: &Path) -> (Vec<u8>, u32, u32) {
    let file =
        std::fs::File::open(path).unwrap_or_else(|e| panic!("abrir {}: {e}", path.display()));
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder.read_info().expect("ler cabecalho do PNG");
    let mut buffer = vec![0; reader.output_buffer_size().expect("tamanho do PNG")];
    let info = reader.next_frame(&mut buffer).expect("ler pixels do PNG");
    buffer.truncate(info.buffer_size());

    assert_eq!(
        info.color_type,
        png::ColorType::Rgba,
        "{} nao e RGBA",
        path.display()
    );
    assert_eq!(
        info.bit_depth,
        png::BitDepth::Eight,
        "{} nao e de 8 bits",
        path.display()
    );

    (buffer, info.width, info.height)
}

/// Coloca o icone e os metadados de versao como recursos do executavel.
///
/// O icone e o que o Explorer, o atalho, a barra de tarefas antes de a janela
/// existir e o registro de programas instalados leem. Sem ele o `.exe` fica com
/// o icone generico do Windows, mesmo com a janela ja mostrando a marca.
///
/// O bloco de versao e o que preenche a aba Detalhes das propriedades do
/// arquivo, a coluna Nome do produto no Gerenciador de Tarefas e a linha de
/// editor do aviso do SmartScreen. Um executavel sem ele aparece anonimo em
/// todos esses lugares -- e continuara aparecendo como editor desconhecido ate
/// existir assinatura de codigo, mas anonimo **e** desconhecido e pior.
///
/// Feito com `windres` em vez de uma crate de recursos porque a toolchain do
/// projeto e a GNU (ADR-0002) e ela ja traz o `windres`. Se ele nao estiver no
/// PATH o build segue sem recurso nenhum: faltar identidade visual e ruim, mas
/// nao pode impedir ninguem de compilar.
fn embed_exe_resources() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("gnu") {
        println!("cargo:warning=recursos do executavel so sao embutidos no alvo GNU");
        return;
    }

    let icon = brand_dir().join("morune.ico");
    println!("cargo:rerun-if-changed={}", icon.display());
    if !icon.exists() {
        println!("cargo:warning=assets/brand/morune.ico ausente; rode tools/make-icon.ps1");
        return;
    }

    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    let script = out.join("morune.rc");
    let object = out.join("morune-resources.o");

    // O icone e copiado para junto do script e referenciado pelo nome puro: o
    // compilador de recursos trata a barra invertida como escape, e um caminho
    // absoluto do Windows escrito ali dentro so daria dor de cabeca.
    std::fs::copy(&icon, out.join("morune.ico")).expect("copiar icone para OUT_DIR");
    // O ID 1 e o que o shell do Windows usa como icone principal do executavel.
    let source = format!("1 ICON \"morune.ico\"\n\n{}", version_info());
    std::fs::write(&script, source).expect("gravar script de recurso");

    let status = std::process::Command::new("windres")
        .current_dir(&out)
        .arg(&script)
        .args(["-O", "coff", "-o"])
        .arg(&object)
        .status();

    match status {
        Ok(status) if status.success() => {
            println!("cargo:rustc-link-arg-bins={}", object.display());
        }
        Ok(status) => println!("cargo:warning=windres falhou ({status}); executavel sem recursos"),
        Err(e) => println!("cargo:warning=windres indisponivel ({e}); executavel sem recursos"),
    }
}

/// Bloco `VERSIONINFO` do executavel.
///
/// Sem acento de proposito: o `.rc` e gravado como bytes e o compilador de
/// recursos interpreta o arquivo na codepage do sistema, nao em UTF-8.
fn version_info() -> String {
    let version = env!("CARGO_PKG_VERSION");
    // `FILEVERSION` quer quatro numeros; a versao do crate tem tres.
    let quad = version
        .split('.')
        .map(|part| part.split('-').next().unwrap_or("0"))
        .chain(std::iter::once("0"))
        .take(4)
        .collect::<Vec<_>>()
        .join(",");

    // VS_FF_DEBUG marca o binario de depuracao, para que ninguem confunda um
    // build local com o que foi distribuido.
    let flags = if std::env::var("PROFILE").as_deref() == Ok("debug") {
        "0x1L"
    } else {
        "0x0L"
    };

    format!(
        r#"1 VERSIONINFO
FILEVERSION {quad}
PRODUCTVERSION {quad}
FILEFLAGSMASK 0x3fL
FILEFLAGS {flags}
FILEOS 0x40004L
FILETYPE 0x1L
FILESUBTYPE 0x0L
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904b0"
        BEGIN
            VALUE "CompanyName", "Morune"
            VALUE "FileDescription", "Morune -- cliente de musica nativo para Windows"
            VALUE "FileVersion", "{version}"
            VALUE "InternalName", "morune"
            VALUE "LegalCopyright", "Copyright (c) 2026 Felipe Seiti Furumori. Licenca MIT."
            VALUE "OriginalFilename", "morune.exe"
            VALUE "ProductName", "Morune"
            VALUE "ProductVersion", "{version}"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x409, 1200
    END
END
"#
    )
}
