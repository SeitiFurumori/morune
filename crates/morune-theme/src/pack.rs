//! Importacao e exportacao de pacotes `.musicpack`.
//!
//! Um `.musicpack` e um ZIP com esta estrutura:
//!
//! ```text
//! manifest.toml        obrigatorio
//! theme.toml           opcional (tokens de cor, tipografia, forma, movimento)
//! layout.toml          opcional (composicao da interface)
//! assets/              opcional (imagens e icones)
//! fonts/               opcional (fontes empacotadas)
//! ```
//!
//! Um pacote vem da internet. Tudo aqui parte do principio de que ele e
//! hostil: nomes de arquivo maliciosos, bombas de compressao e tipos de
//! arquivo executaveis sao recusados antes de qualquer escrita em disco.

use std::collections::HashSet;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};

use crate::manifest::ThemeManifest;

/// Numero maximo de arquivos num pacote.
pub const MAX_ENTRIES: usize = 512;
/// Tamanho maximo de um arquivo apos descompactar (8 MiB).
pub const MAX_FILE_SIZE: u64 = 8 * 1024 * 1024;
/// Tamanho maximo do pacote inteiro apos descompactar (64 MiB).
pub const MAX_TOTAL_SIZE: u64 = 64 * 1024 * 1024;
/// Razao maxima entre tamanho descompactado e compactado.
///
/// Uma bomba de compressao classica passa de 1000:1. Texto e TOML raramente
/// passam de 20:1, entao 200:1 deixa margem larga e ainda barra o ataque.
pub const MAX_COMPRESSION_RATIO: u64 = 200;

/// Extensoes aceitas dentro de um pacote. Qualquer outra coisa e recusada.
///
/// Lista de permissao, nunca de bloqueio: um pacote nao tem motivo para conter
/// executavel, script ou biblioteca, e enumerar o que e perigoso e uma corrida
/// que se perde.
const ALLOWED_EXTENSIONS: &[&str] = &[
    "toml", "json", "png", "jpg", "jpeg", "webp", "svg", "ttf", "otf", "woff2", "md", "txt",
];

/// Arquivos aceitos na raiz do pacote.
const ROOT_FILES: &[&str] = &["manifest.toml", "theme.toml", "layout.toml", "README.md", "LICENSE.txt"];

/// Diretorios aceitos no pacote.
const ALLOWED_DIRS: &[&str] = &["assets", "fonts"];

#[derive(Debug, thiserror::Error)]
pub enum PackError {
    #[error("erro de arquivo: {0}")]
    Io(#[from] io::Error),

    #[error("pacote invalido: {0}")]
    Zip(String),

    #[error("manifest.toml ausente no pacote")]
    MissingManifest,

    #[error("manifest.toml invalido: {0}")]
    BadManifest(String),

    #[error("caminho perigoso no pacote: {0:?}")]
    UnsafePath(String),

    #[error("tipo de arquivo nao permitido no pacote: {0:?}")]
    ForbiddenFile(String),

    #[error("pacote grande demais: {what} ({value} > {limit})")]
    TooLarge { what: &'static str, value: u64, limit: u64 },

    #[error("possivel bomba de compressao: {0:?} expande {1}x")]
    CompressionBomb(String, u64),

    #[error("o tema {0:?} ja existe")]
    AlreadyExists(String),
}

/// Resultado de uma importacao bem sucedida.
#[derive(Debug, Clone)]
pub struct ImportedTheme {
    pub manifest: ThemeManifest,
    /// Pasta onde o tema foi instalado.
    pub path: PathBuf,
    pub files_written: usize,
    pub bytes_written: u64,
}

/// Valida um caminho vindo de dentro do ZIP e devolve a forma relativa segura.
///
/// Recusa: caminhos absolutos, prefixos de unidade (`C:`), `..`, separadores
/// invertidos usados para escapar, nomes vazios e componentes ocultos do
/// Windows. Esta funcao e a unica porta de entrada de nomes de arquivo do
/// pacote: qualquer coisa que passe daqui pode ser escrita.
pub fn sanitize_entry_path(raw: &str) -> Result<PathBuf, PackError> {
    let unsafe_path = || PackError::UnsafePath(raw.to_string());

    if raw.is_empty() || raw.len() > 255 {
        return Err(unsafe_path());
    }
    // O ZIP usa `/`; uma `\` so aparece em nome montado a mao para enganar
    // extratores que normalizam separadores no Windows.
    if raw.contains('\\') || raw.contains('\0') {
        return Err(unsafe_path());
    }
    // Fluxo alternativo de dados do NTFS.
    if raw.contains(':') {
        return Err(unsafe_path());
    }

    let path = Path::new(raw);
    let mut clean = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let part = part.to_str().ok_or_else(unsafe_path)?;
                if part == "." || part.starts_with(' ') || part.ends_with(' ') || part.ends_with('.')
                {
                    return Err(unsafe_path());
                }
                clean.push(part);
            }
            // `..`, `/`, `C:\` e afins.
            _ => return Err(unsafe_path()),
        }
    }

    if clean.as_os_str().is_empty() {
        return Err(unsafe_path());
    }
    Ok(clean)
}

/// Verifica se o caminho relativo tem lugar e tipo permitidos no pacote.
pub fn check_entry_allowed(path: &Path) -> Result<(), PackError> {
    let display = path.to_string_lossy().to_string();
    let components: Vec<&str> = path.components().filter_map(|c| c.as_os_str().to_str()).collect();

    match components.len() {
        1 => {
            if !ROOT_FILES.contains(&components[0]) {
                return Err(PackError::ForbiddenFile(display));
            }
        }
        2..=4 => {
            if !ALLOWED_DIRS.contains(&components[0]) {
                return Err(PackError::ForbiddenFile(display));
            }
        }
        _ => return Err(PackError::ForbiddenFile(display)),
    }

    if components.len() > 1 {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();
        if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
            return Err(PackError::ForbiddenFile(display));
        }
    }

    Ok(())
}

/// Importa um `.musicpack` para dentro de `themes_dir`.
///
/// A extracao acontece numa pasta temporaria irma e so e movida para o destino
/// final quando tudo foi validado, para que uma falha no meio nunca deixe um
/// tema pela metade instalado.
pub fn import_pack(
    pack_path: &Path,
    themes_dir: &Path,
    overwrite: bool,
) -> Result<ImportedTheme, PackError> {
    let file = fs::File::open(pack_path)?;
    let compressed_len = file.metadata()?.len();
    let mut archive =
        zip::ZipArchive::new(io::BufReader::new(file)).map_err(|e| PackError::Zip(e.to_string()))?;

    if archive.len() > MAX_ENTRIES {
        return Err(PackError::TooLarge {
            what: "numero de arquivos",
            value: archive.len() as u64,
            limit: MAX_ENTRIES as u64,
        });
    }

    // Primeira passada: valida tudo sem escrever nada.
    let mut plan: Vec<(usize, PathBuf, u64)> = Vec::new();
    let mut total: u64 = 0;
    let mut seen = HashSet::new();

    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| PackError::Zip(e.to_string()))?;
        if entry.is_dir() {
            continue;
        }

        let relative = sanitize_entry_path(entry.name())?;
        check_entry_allowed(&relative)?;

        // Dois arquivos com o mesmo destino significam tentativa de confundir
        // o extrator sobre qual conteudo prevalece.
        if !seen.insert(relative.clone()) {
            return Err(PackError::UnsafePath(entry.name().to_string()));
        }

        let size = entry.size();
        if size > MAX_FILE_SIZE {
            return Err(PackError::TooLarge {
                what: "arquivo",
                value: size,
                limit: MAX_FILE_SIZE,
            });
        }
        let packed = entry.compressed_size().max(1);
        if size / packed > MAX_COMPRESSION_RATIO {
            return Err(PackError::CompressionBomb(entry.name().to_string(), size / packed));
        }

        total = total.saturating_add(size);
        if total > MAX_TOTAL_SIZE {
            return Err(PackError::TooLarge {
                what: "conteudo total",
                value: total,
                limit: MAX_TOTAL_SIZE,
            });
        }

        plan.push((i, relative, size));
    }

    if total / compressed_len.max(1) > MAX_COMPRESSION_RATIO {
        return Err(PackError::CompressionBomb(
            pack_path.to_string_lossy().to_string(),
            total / compressed_len.max(1),
        ));
    }

    let manifest_index = plan
        .iter()
        .find(|(_, p, _)| p == Path::new("manifest.toml"))
        .map(|(i, _, _)| *i)
        .ok_or(PackError::MissingManifest)?;

    let manifest = {
        let mut raw = String::new();
        archive
            .by_index(manifest_index)
            .map_err(|e| PackError::Zip(e.to_string()))?
            .read_to_string(&mut raw)?;
        let manifest: ThemeManifest =
            toml::from_str(&raw).map_err(|e| PackError::BadManifest(e.to_string()))?;
        manifest.validate().map_err(|e| PackError::BadManifest(e.to_string()))?;
        manifest
    };

    let destination = themes_dir.join(&manifest.id);
    if destination.exists() && !overwrite {
        return Err(PackError::AlreadyExists(manifest.id.clone()));
    }

    // Segunda passada: extrai para uma pasta temporaria.
    let staging = themes_dir.join(format!(".{}.importing", manifest.id));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;

    let result = (|| -> Result<(usize, u64), PackError> {
        let mut written_bytes = 0u64;
        for (index, relative, expected) in &plan {
            let target = staging.join(relative);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut entry =
                archive.by_index(*index).map_err(|e| PackError::Zip(e.to_string()))?;
            let mut out = io::BufWriter::new(fs::File::create(&target)?);
            // Copia limitada: se o cabecalho do ZIP mentiu sobre o tamanho, o
            // limite corta antes de encher o disco.
            let copied = io::copy(&mut entry.by_ref().take(MAX_FILE_SIZE + 1), &mut out)?;
            out.flush()?;
            if copied > MAX_FILE_SIZE {
                return Err(PackError::TooLarge {
                    what: "arquivo",
                    value: copied,
                    limit: MAX_FILE_SIZE,
                });
            }
            // O tamanho declarado no indice do ZIP e so uma pista: o que vale e
            // o que realmente saiu do descompactador, ja limitado acima.
            if copied != *expected {
                tracing::debug!(
                    entry = %relative.display(),
                    declared = expected,
                    actual = copied,
                    "tamanho declarado no pacote nao bate com o extraido"
                );
            }
            written_bytes += copied;
        }
        Ok((plan.len(), written_bytes))
    })();

    let (files_written, bytes_written) = match result {
        Ok(v) => v,
        Err(e) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(e);
        }
    };

    if destination.exists() {
        fs::remove_dir_all(&destination)?;
    }
    fs::rename(&staging, &destination)?;

    Ok(ImportedTheme { manifest, path: destination, files_written, bytes_written })
}

/// Exporta a pasta de um tema como `.musicpack`.
pub fn export_pack(theme_dir: &Path, output: &Path) -> Result<u64, PackError> {
    let manifest_path = theme_dir.join("manifest.toml");
    if !manifest_path.is_file() {
        return Err(PackError::MissingManifest);
    }

    let file = fs::File::create(output)?;
    let mut zip = zip::ZipWriter::new(io::BufWriter::new(file));
    let options: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let mut count = 0u64;
    let mut stack = vec![(theme_dir.to_path_buf(), PathBuf::new())];

    while let Some((dir, prefix)) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy().to_string();
            let relative = prefix.join(&name);

            if entry.file_type()?.is_dir() {
                if prefix.as_os_str().is_empty() && !ALLOWED_DIRS.contains(&name.as_str()) {
                    continue;
                }
                stack.push((entry.path(), relative));
                continue;
            }

            // Exportar so o que sabemos importar mantem os dois lados em acordo:
            // um pacote exportado pelo Morune sempre pode ser reimportado.
            if check_entry_allowed(&relative).is_err() {
                continue;
            }

            let zip_name = relative.to_string_lossy().replace('\\', "/");
            zip.start_file(zip_name, options).map_err(|e| PackError::Zip(e.to_string()))?;
            let mut source = fs::File::open(entry.path())?;
            io::copy(&mut source, &mut zip)?;
            count += 1;
        }
    }

    zip.finish().map_err(|e| PackError::Zip(e.to_string()))?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_directory_escapes() {
        for bad in [
            "../evil.toml",
            "../../windows/system32/x.toml",
            "assets/../../evil.toml",
            "a/../../b.toml",
        ] {
            assert!(
                matches!(sanitize_entry_path(bad), Err(PackError::UnsafePath(_))),
                "aceitou {bad:?}"
            );
        }
    }

    #[test]
    fn rejects_absolute_and_drive_paths() {
        for bad in ["/etc/passwd", "C:/windows/x.toml", "C:x.toml", "//server/share/x.toml"] {
            assert!(
                matches!(sanitize_entry_path(bad), Err(PackError::UnsafePath(_))),
                "aceitou {bad:?}"
            );
        }
    }

    #[test]
    fn rejects_backslash_and_null_tricks() {
        for bad in ["..\\evil.toml", "assets\\x.png", "a\0b.toml"] {
            assert!(
                matches!(sanitize_entry_path(bad), Err(PackError::UnsafePath(_))),
                "aceitou {bad:?}"
            );
        }
    }

    #[test]
    fn rejects_windows_trailing_dot_and_space_names() {
        for bad in ["assets/x.png.", "assets/x .png ", "assets/ x.png"] {
            assert!(
                matches!(sanitize_entry_path(bad), Err(PackError::UnsafePath(_))),
                "aceitou {bad:?}"
            );
        }
    }

    #[test]
    fn accepts_ordinary_pack_paths() {
        assert_eq!(sanitize_entry_path("manifest.toml").unwrap(), Path::new("manifest.toml"));
        assert_eq!(
            sanitize_entry_path("assets/icons/play.svg").unwrap(),
            Path::new("assets").join("icons").join("play.svg")
        );
    }

    #[test]
    fn only_whitelisted_files_are_allowed() {
        assert!(check_entry_allowed(Path::new("manifest.toml")).is_ok());
        assert!(check_entry_allowed(Path::new("theme.toml")).is_ok());
        assert!(check_entry_allowed(Path::new("assets/cover.png")).is_ok());
        assert!(check_entry_allowed(Path::new("fonts/Inter.ttf")).is_ok());

        for bad in [
            "evil.exe",
            "assets/payload.exe",
            "assets/script.ps1",
            "assets/lib.dll",
            "assets/run.bat",
            "scripts/hook.lua",
            "assets/a/b/c/d/e.png",
        ] {
            assert!(
                check_entry_allowed(Path::new(bad)).is_err(),
                "arquivo perigoso aceito: {bad:?}"
            );
        }
    }

    #[test]
    fn extension_check_is_case_insensitive() {
        assert!(check_entry_allowed(Path::new("assets/COVER.PNG")).is_ok());
        assert!(check_entry_allowed(Path::new("assets/x.EXE")).is_err());
    }

    #[test]
    fn unknown_root_file_is_refused() {
        assert!(check_entry_allowed(Path::new("secrets.toml")).is_err());
        assert!(check_entry_allowed(Path::new("autorun.inf")).is_err());
    }
}
