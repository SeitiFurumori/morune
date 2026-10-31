//! Protocolo `capa`: a tela pede imagens do disco (capas, fundo, avatar) por
//! endereco, e o motor so entrega arquivos de imagem de pastas conhecidas.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static PASTAS: OnceLock<Vec<PathBuf>> = OnceLock::new();

/// Pastas de onde a tela pode ler imagem. Chamado uma vez, na abertura.
pub fn permitir(pastas: Vec<PathBuf>) {
    let _ = PASTAS.set(pastas);
}

/// Endereco que a tela usa para pedir o arquivo.
pub fn endereco(path: &Path) -> String {
    let mut s = String::from("http://capa.localhost/?p=");
    for b in path.to_string_lossy().bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => s.push(b as char),
            _ => s.push_str(&format!("%{b:02X}")),
        }
    }
    s
}

fn decodificar(q: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(q.len());
    let mut it = q.bytes();
    while let Some(b) = it.next() {
        if b == b'%' {
            let h = [it.next()?, it.next()?];
            bytes.push(u8::from_str_radix(std::str::from_utf8(&h).ok()?, 16).ok()?);
        } else if b == b'+' {
            bytes.push(b' ');
        } else {
            bytes.push(b);
        }
    }
    String::from_utf8(bytes).ok()
}

/// O arquivo pedido, se for imagem e estiver dentro de uma pasta permitida.
pub fn resolver(query: &str) -> Option<PathBuf> {
    let p = query.strip_prefix("p=")?;
    let path = PathBuf::from(decodificar(p)?).canonicalize().ok()?;
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    if !matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp" | "gif" | "bmp" | "mp4" | "webm") {
        return None;
    }
    let pastas = PASTAS.get()?;
    pastas
        .iter()
        .filter_map(|d| d.canonicalize().ok())
        .any(|d| path.starts_with(&d))
        .then_some(path)
}

pub fn tipo(path: &Path) -> &'static str {
    match path.extension().map(|e| e.to_string_lossy().to_lowercase()).as_deref() {
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        Some("bmp") => "image/bmp",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        _ => "image/jpeg",
    }
}
