//! Copiar texto para a area de transferencia do Windows.
//!
//! Existe por um motivo so: o link de autorizacao do login. Quem tem mais de um
//! navegador -- ou mais de um perfil no mesmo navegador -- precisa levar aquele
//! endereco para o lugar certo, e digitar uma URL de OAuth a mao nao e opcao.
//!
//! O Slint 1.17 nao expoe area de transferencia programatica: ele resolve copia
//! e cola dentro de campos de texto, e nada alem disso. Por isso a chamada e
//! direto na API do Windows, que sao quatro passos e nenhuma dependencia nova.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use windows::Win32::Foundation::{HANDLE, HGLOBAL};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::System::Ole::CF_UNICODETEXT;

/// Fecha a area de transferencia ao sair do escopo.
///
/// A area de transferencia e um recurso global do sistema: deixa-la aberta
/// impede **todos** os outros programas de copiar qualquer coisa, ate este
/// processo morrer. Um `?` no meio do caminho nao pode causar isso.
struct ClipboardGuard;

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        // SAFETY: so existe um guard, criado apos um `OpenClipboard` bem
        // sucedido, e ele fecha exatamente uma vez.
        let _ = unsafe { CloseClipboard() };
    }
}

/// Copia `texto` para a area de transferencia.
pub fn copy(texto: &str) -> Result<(), String> {
    // UTF-16 terminado em nulo: e o formato que o `CF_UNICODETEXT` espera.
    let utf16: Vec<u16> = OsStr::new(texto).encode_wide().chain(Some(0)).collect();
    let bytes = std::mem::size_of_val(utf16.as_slice());

    // SAFETY: janela nula pede a area de transferencia para a thread atual.
    unsafe { OpenClipboard(None) }.map_err(|e| format!("area de transferencia ocupada: {e}"))?;
    let _guard = ClipboardGuard;

    // SAFETY: a area esta aberta por este processo.
    unsafe { EmptyClipboard() }.map_err(|e| format!("nao foi possivel limpar: {e}"))?;

    // `GMEM_MOVEABLE` e exigencia do formato: quem recebe o bloco e o sistema,
    // e ele so aceita memoria movivel.
    // SAFETY: tamanho calculado a partir do proprio vetor.
    let bloco: HGLOBAL =
        unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes) }.map_err(|e| format!("sem memoria: {e}"))?;

    // SAFETY: `bloco` acabou de ser alocado com o tamanho de `utf16`.
    unsafe {
        let destino = GlobalLock(bloco).cast::<u16>();
        if destino.is_null() {
            let _ = GlobalUnlock(bloco);
            return Err("nao foi possivel escrever no bloco".into());
        }
        std::ptr::copy_nonoverlapping(utf16.as_ptr(), destino, utf16.len());
        let _ = GlobalUnlock(bloco);
    }

    // SAFETY: o bloco esta preenchido e no formato declarado. A partir daqui
    // ele pertence ao sistema, e liberar por conta propria seria erro.
    unsafe { SetClipboardData(CF_UNICODETEXT.0 as u32, Some(HANDLE(bloco.0))) }
        .map_err(|e| format!("o Windows recusou o texto: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exercita o caminho Win32 de verdade.
    ///
    /// `#[ignore]` porque **sobrescreve a area de transferencia de quem roda a
    /// suite** -- um teste nao tem o direito de jogar fora o que a pessoa
    /// acabou de copiar. Rode a mao quando mexer neste modulo:
    ///
    /// ```text
    /// cargo test -p morune-app -- --ignored copia_para_a_area_de_transferencia
    /// ```
    #[test]
    #[ignore = "sobrescreve a area de transferencia do sistema"]
    fn copia_para_a_area_de_transferencia() {
        // Acentos e um `/` para cobrir o que uma URL de OAuth realmente tem.
        copy("morune-teste: ação/https://exemplo").expect("copiar");
    }
}
