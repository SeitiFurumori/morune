//! Registro local de travamento (minidump), sem enviar nada.
//!
//! **Por que existe.** O `panic` do Rust ja vai para o log (`log_panics` em
//! `main.rs`), mas uma queda nativa -- driver de video, DLL de terceiro,
//! acesso invalido dentro de uma dependencia -- fecha o processo sem uma
//! linha. Quem reporta "o Morune fechou sozinho" nao tinha nada para anexar.
//!
//! **O que faz.** Um filtro de excecao nao tratada grava
//! `morune-queda-<unix>.dmp` na pasta de dados, com o estado das threads
//! (`MiniDumpNormal`, dezenas a centenas de KB). Guarda so as 3 mais recentes.
//! **Nada sai do computador**: telemetria esta fora do produto (ROADMAP).
//!
//! **Custo.** Zero ate o processo cair: e so um ponteiro de funcao registrado.

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::sync::OnceLock;

    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, GENERIC_WRITE};
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_NONE,
    };
    use windows::Win32::System::Diagnostics::Debug::{
        MiniDumpNormal, MiniDumpWriteDump, SetUnhandledExceptionFilter, EXCEPTION_POINTERS,
        MINIDUMP_EXCEPTION_INFORMATION,
    };
    use windows::Win32::System::Threading::{
        GetCurrentProcess, GetCurrentProcessId, GetCurrentThreadId,
    };

    /// Quantos arquivos de queda ficam guardados.
    const GUARDAR: usize = 3;

    static PASTA: OnceLock<PathBuf> = OnceLock::new();

    pub fn instalar(pasta: &Path) {
        let _ = std::fs::create_dir_all(pasta);
        limpar_antigos(pasta);
        if PASTA.set(pasta.to_path_buf()).is_ok() {
            // SAFETY: registra um ponteiro para funcao `extern "system"` com a
            // assinatura exigida; nao ha estado compartilhado alem de `PASTA`,
            // que e so lida depois de inicializada.
            unsafe {
                SetUnhandledExceptionFilter(Some(ao_cair));
            }
        }
    }

    fn limpar_antigos(pasta: &Path) {
        let Ok(entradas) = std::fs::read_dir(pasta) else {
            return;
        };
        let mut dumps: Vec<_> = entradas
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().starts_with("morune-queda-"))
            .collect();
        dumps.sort_by_key(|e| std::cmp::Reverse(e.file_name()));
        for velho in dumps.into_iter().skip(GUARDAR) {
            let _ = std::fs::remove_file(velho.path());
        }
    }

    unsafe extern "system" fn ao_cair(info: *const EXCEPTION_POINTERS) -> i32 {
        const EXCEPTION_CONTINUE_SEARCH: i32 = 0;
        let Some(pasta) = PASTA.get() else {
            return EXCEPTION_CONTINUE_SEARCH;
        };
        let agora = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let caminho = pasta.join(format!("morune-queda-{agora}.dmp"));
        let largo: Vec<u16> = caminho
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // SAFETY: dentro do filtro de excecao o processo ja esta caindo; as
        // chamadas usam o caminho local terminado em nulo e o `info` que o
        // proprio Windows entregou.
        unsafe {
            let Ok(arquivo) = CreateFileW(
                PCWSTR(largo.as_ptr()),
                GENERIC_WRITE.0,
                FILE_SHARE_NONE,
                None,
                CREATE_ALWAYS,
                FILE_ATTRIBUTE_NORMAL,
                None,
            ) else {
                return EXCEPTION_CONTINUE_SEARCH;
            };
            let excecao = MINIDUMP_EXCEPTION_INFORMATION {
                ThreadId: GetCurrentThreadId(),
                ExceptionPointers: info as *mut _,
                ClientPointers: false.into(),
            };
            let _ = MiniDumpWriteDump(
                GetCurrentProcess(),
                GetCurrentProcessId(),
                arquivo,
                MiniDumpNormal,
                Some(&excecao),
                None,
                None,
            );
            let _ = CloseHandle(arquivo);
        }
        // Deixa o Windows seguir o caminho normal da queda (relatorio de erro,
        // codigo de saida) -- o arquivo e so um extra.
        EXCEPTION_CONTINUE_SEARCH
    }
}

/// Liga o registro de queda. A pasta costuma ser a de dados do Morune.
pub fn instalar(pasta: &std::path::Path) {
    #[cfg(windows)]
    imp::instalar(pasta);
    #[cfg(not(windows))]
    let _ = pasta;
}
