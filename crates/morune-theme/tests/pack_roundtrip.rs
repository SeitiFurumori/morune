//! Testes de ponta a ponta dos pacotes `.musicpack`.
//!
//! Os testes unitarios em `pack.rs` cobrem a validacao de nomes isoladamente.
//! Aqui os pacotes sao ZIPs de verdade, escritos em disco e importados pelo
//! caminho real -- inclusive os maliciosos, para que as defesas descritas em
//! `SECURITY.md` sejam exercitadas e nao apenas afirmadas.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use morune_theme::pack::{export_pack, import_pack, PackError};

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "morune-pack-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const MANIFEST: &str = r#"schema_version = 1
id = "importado"
name = "Importado"
author = "Teste"
"#;

const THEME: &str = r##"[color]
accent = "#ff0066"
"##;

/// Escreve um ZIP com os pares (nome, conteudo) exatamente como informados,
/// sem nenhuma sanitizacao -- e o papel deste auxiliar produzir pacotes que um
/// atacante produziria.
fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
    let file = fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for (name, contents) in entries {
        zip.start_file(*name, options).unwrap();
        zip.write_all(contents).unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn imports_a_well_formed_pack() {
    let dir = TempDir::new("ok");
    let pack = dir.path().join("tema.musicpack");
    let themes = dir.path().join("themes");
    fs::create_dir_all(&themes).unwrap();

    write_zip(
        &pack,
        &[
            ("manifest.toml", MANIFEST.as_bytes()),
            ("theme.toml", THEME.as_bytes()),
            ("assets/capa.png", b"\x89PNG\r\n\x1a\n falso"),
        ],
    );

    let imported = import_pack(&pack, &themes, false).unwrap();
    assert_eq!(imported.manifest.id, "importado");
    assert_eq!(imported.files_written, 3);
    assert!(themes.join("importado").join("theme.toml").is_file());
    assert!(themes.join("importado").join("assets").join("capa.png").is_file());

    // E o tema importado precisa carregar de verdade, nao so existir em disco.
    let loaded = morune_theme::load(&themes, "importado");
    assert!(loaded.is_healthy(), "{:?}", loaded.errors);
    assert_eq!(loaded.spec.colors.accent, morune_theme::Color::rgb(0xff, 0x00, 0x66));
}

#[test]
fn export_then_import_round_trips() {
    let dir = TempDir::new("roundtrip");
    let themes = dir.path().join("themes");
    let origem = themes.join("origem");
    fs::create_dir_all(origem.join("assets")).unwrap();

    fs::write(
        origem.join("manifest.toml"),
        "schema_version = 1\nid = \"origem\"\nname = \"Origem\"\n",
    )
    .unwrap();
    fs::write(origem.join("theme.toml"), THEME).unwrap();
    fs::write(origem.join("assets").join("x.png"), b"png").unwrap();
    // Arquivo que o importador recusaria: o exportador nao pode inclui-lo, ou o
    // proprio pacote que geramos seria irrecuperavel.
    fs::write(origem.join("assets").join("payload.exe"), b"MZ").unwrap();

    let pack = dir.path().join("origem.musicpack");
    let count = export_pack(&origem, &pack).unwrap();
    assert_eq!(count, 3, "o executavel nao deveria ter sido empacotado");

    let destino = dir.path().join("outros-temas");
    fs::create_dir_all(&destino).unwrap();
    let imported = import_pack(&pack, &destino, false).unwrap();

    assert_eq!(imported.manifest.id, "origem");
    assert!(destino.join("origem").join("assets").join("x.png").is_file());
    assert!(!destino.join("origem").join("assets").join("payload.exe").exists());
}

#[test]
fn refuses_path_traversal_and_writes_nothing() {
    let dir = TempDir::new("traversal");
    let pack = dir.path().join("mau.musicpack");
    let themes = dir.path().join("themes");
    fs::create_dir_all(&themes).unwrap();

    write_zip(
        &pack,
        &[
            ("manifest.toml", MANIFEST.as_bytes()),
            ("../../../fora.toml", b"escapou"),
        ],
    );

    let err = import_pack(&pack, &themes, false).unwrap_err();
    assert!(matches!(err, PackError::UnsafePath(_)), "erro inesperado: {err}");

    // Nada foi escrito: nem o alvo, nem a pasta temporaria de importacao.
    assert!(!dir.path().join("fora.toml").exists());
    assert!(!themes.join("importado").exists());
    assert_eq!(fs::read_dir(&themes).unwrap().count(), 0);
}

#[test]
fn refuses_executable_payloads() {
    let dir = TempDir::new("exe");
    let pack = dir.path().join("mau.musicpack");
    let themes = dir.path().join("themes");
    fs::create_dir_all(&themes).unwrap();

    write_zip(
        &pack,
        &[("manifest.toml", MANIFEST.as_bytes()), ("assets/instalar.exe", b"MZ")],
    );

    assert!(matches!(
        import_pack(&pack, &themes, false).unwrap_err(),
        PackError::ForbiddenFile(_)
    ));
    assert_eq!(fs::read_dir(&themes).unwrap().count(), 0);
}

#[test]
fn refuses_a_compression_bomb() {
    let dir = TempDir::new("bomba");
    let pack = dir.path().join("bomba.musicpack");
    let themes = dir.path().join("themes");
    fs::create_dir_all(&themes).unwrap();

    // 4 MiB de zeros comprimem para alguns KB: bem acima da razao de 200:1.
    let zeros = vec![0u8; 4 * 1024 * 1024];
    write_zip(
        &pack,
        &[("manifest.toml", MANIFEST.as_bytes()), ("assets/grande.png", &zeros)],
    );

    let err = import_pack(&pack, &themes, false).unwrap_err();
    assert!(matches!(err, PackError::CompressionBomb(_, _)), "erro inesperado: {err}");
    assert_eq!(fs::read_dir(&themes).unwrap().count(), 0);
}

#[test]
fn refuses_a_pack_without_manifest() {
    let dir = TempDir::new("sem-manifesto");
    let pack = dir.path().join("x.musicpack");
    let themes = dir.path().join("themes");
    fs::create_dir_all(&themes).unwrap();

    write_zip(&pack, &[("theme.toml", THEME.as_bytes())]);

    assert!(matches!(
        import_pack(&pack, &themes, false).unwrap_err(),
        PackError::MissingManifest
    ));
}

#[test]
fn refuses_a_manifest_with_an_unsafe_id() {
    let dir = TempDir::new("id-mau");
    let pack = dir.path().join("x.musicpack");
    let themes = dir.path().join("themes");
    fs::create_dir_all(&themes).unwrap();

    write_zip(
        &pack,
        &[(
            "manifest.toml",
            b"schema_version = 1\nid = \"../fuga\"\nname = \"Fuga\"\n",
        )],
    );

    assert!(matches!(
        import_pack(&pack, &themes, false).unwrap_err(),
        PackError::BadManifest(_)
    ));
}

#[test]
fn existing_theme_is_kept_unless_overwrite_is_asked() {
    let dir = TempDir::new("existente");
    let pack = dir.path().join("x.musicpack");
    let themes = dir.path().join("themes");
    fs::create_dir_all(themes.join("importado")).unwrap();
    fs::write(themes.join("importado").join("marca.txt"), b"original").unwrap();

    write_zip(&pack, &[("manifest.toml", MANIFEST.as_bytes())]);

    assert!(matches!(
        import_pack(&pack, &themes, false).unwrap_err(),
        PackError::AlreadyExists(_)
    ));
    assert!(themes.join("importado").join("marca.txt").is_file());

    // Com sobrescrita explicita o diretorio e substituido por inteiro.
    import_pack(&pack, &themes, true).unwrap();
    assert!(!themes.join("importado").join("marca.txt").exists());
    assert!(themes.join("importado").join("manifest.toml").is_file());
}

// A checagem de destinos duplicados em `import_pack` nao tem teste aqui de
// proposito: nao ha como produzi-la com um escritor de ZIP normal (o crate
// `zip` recusa nomes repetidos) nem com dois nomes distintos, porque o
// sanitizador de caminho ja recusa `./x` e `a/../x` antes de compara-los. Ela
// permanece como defesa em profundidade contra um ZIP montado byte a byte.
