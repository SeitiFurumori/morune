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

/// Inicio e fim (inclusive) de um cabecalho `Range: bytes=a-b`. Sem cabecalho,
/// ou com um que nao se entende, o arquivo todo.
pub fn faixa_pedida(range: Option<&str>, total: u64) -> (u64, u64) {
    let tudo = (0, total.saturating_sub(1));
    let Some(r) = range.and_then(|r| r.trim().strip_prefix("bytes=")) else { return tudo };
    let Some((a, b)) = r.split(',').next().and_then(|r| r.split_once('-')) else { return tudo };
    match (a.trim().parse::<u64>().ok(), b.trim().parse::<u64>().ok()) {
        (Some(a), Some(b)) if b >= a => (a, b),
        (Some(a), None) => (a, total.saturating_sub(1)),
        // `bytes=-500`: os ultimos 500.
        (None, Some(n)) => (total.saturating_sub(n), total.saturating_sub(1)),
        _ => tudo,
    }
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

#[cfg(test)]
mod tests {
    #[test]
    fn entende_os_pedidos_de_pedaco() {
        use super::faixa_pedida;
        assert_eq!(faixa_pedida(None, 100), (0, 99));
        assert_eq!(faixa_pedida(Some("bytes=10-19"), 100), (10, 19));
        assert_eq!(faixa_pedida(Some("bytes=50-"), 100), (50, 99));
        assert_eq!(faixa_pedida(Some("bytes=-30"), 100), (70, 99));
        assert_eq!(faixa_pedida(Some("lixo"), 100), (0, 99));
    }
}
