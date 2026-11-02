//! Preferencia de inicializacao junto com a sessao do Windows.
//!
//! A entrada vive em HKCU: vale somente para a pessoa atual, nao exige UAC e
//! e a mesma usada pelo instalador. O argumento `--startup` permite iniciar
//! discretamente na bandeja sem mudar a abertura manual do aplicativo.

#[cfg(windows)]
mod platform {
    use std::os::windows::ffi::OsStrExt;

    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
        RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE,
        REG_OPTION_NON_VOLATILE, REG_SZ,
    };

    const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    const VALUE_NAME: PCWSTR = w!("Otomori");
    /// Nome da entrada ate a 1.0. Ver [`migrar_do_morune`].
    const LEGACY_NAME: PCWSTR = w!("Morune");

    pub fn is_enabled() -> bool {
        exists(VALUE_NAME)
    }

    /// Troca a entrada "Morune" da 1.0 pela "Otomori", apontando para este exe.
    ///
    /// O instalador ja faz isso; aqui cobre quem copiou o exe sem instalar.
    /// Sem isto o Windows abriria o `morune.exe` antigo no login, ou nada, se
    /// ele ja tiver sido apagado.
    pub fn migrar_do_morune() {
        if !exists(LEGACY_NAME) {
            return;
        }
        if let Err(error) = enable() {
            tracing::warn!(%error, "nao consegui recriar a entrada de iniciar com o Windows");
            return;
        }
        let _ = delete(LEGACY_NAME);
    }

    fn exists(name: PCWSTR) -> bool {
        let mut key = HKEY::default();
        // SAFETY: ponteiro de saida valido e caminho/flags constantes.
        let opened =
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, None, KEY_QUERY_VALUE, &mut key) };
        if opened != ERROR_SUCCESS {
            return false;
        }
        // Consultar sem buffer verifica somente a existencia do valor.
        // SAFETY: HKEY aberto acima e parametros opcionais nulos sao aceitos.
        let queried = unsafe { RegQueryValueExW(key, name, None, None, None, None) };
        // SAFETY: fecha exatamente a chave aberta nesta funcao.
        let _ = unsafe { RegCloseKey(key) };
        queried == ERROR_SUCCESS
    }

    pub fn set_enabled(enabled: bool) -> Result<(), StartupError> {
        if enabled {
            enable()
        } else {
            disable()
        }
    }

    fn enable() -> Result<(), StartupError> {
        let executable = std::env::current_exe()?;
        let command = command_for(&executable);

        let mut key = HKEY::default();
        // SAFETY: cria/abre somente a chave Run do usuario atual.
        let created = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                RUN_KEY,
                None,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &mut key,
                None,
            )
        };
        if created != ERROR_SUCCESS {
            return Err(StartupError::Windows(created.0));
        }

        // REG_SZ recebe bytes UTF-16 incluindo o terminador nulo.
        let bytes =
            unsafe { std::slice::from_raw_parts(command.as_ptr().cast::<u8>(), command.len() * 2) };
        // SAFETY: chave valida, nome constante e buffer vivo durante a chamada.
        let written = unsafe { RegSetValueExW(key, VALUE_NAME, None, REG_SZ, Some(bytes)) };
        // SAFETY: fecha exatamente a chave criada/aberta acima.
        let _ = unsafe { RegCloseKey(key) };
        if written == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(StartupError::Windows(written.0))
        }
    }

    fn command_for(executable: &std::path::Path) -> Vec<u16> {
        let mut command = Vec::<u16>::new();
        command.push(u16::from(b'"'));
        command.extend(executable.as_os_str().encode_wide());
        command.extend("\" --startup\0".encode_utf16());
        command
    }

    fn disable() -> Result<(), StartupError> {
        delete(LEGACY_NAME)?;
        delete(VALUE_NAME)
    }

    fn delete(name: PCWSTR) -> Result<(), StartupError> {
        let mut key = HKEY::default();
        // SAFETY: abre somente a chave Run do usuario atual para escrita.
        let opened =
            unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, None, KEY_SET_VALUE, &mut key) };
        if opened == ERROR_FILE_NOT_FOUND {
            return Ok(());
        }
        if opened != ERROR_SUCCESS {
            return Err(StartupError::Windows(opened.0));
        }

        // SAFETY: HKEY valido; remover valor ausente tambem conta como sucesso.
        let deleted = unsafe { RegDeleteValueW(key, name) };
        // SAFETY: fecha exatamente a chave aberta acima.
        let _ = unsafe { RegCloseKey(key) };
        if deleted == ERROR_SUCCESS || deleted == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(StartupError::Windows(deleted.0))
        }
    }

    #[derive(Debug, thiserror::Error)]
    pub enum StartupError {
        #[error("nao foi possivel localizar o executavel: {0}")]
        Executable(#[from] std::io::Error),
        #[error("o Windows recusou a alteracao (codigo {0})")]
        Windows(u32),
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn startup_command_quotes_paths_with_spaces_and_marks_the_launch() {
            let command = command_for(std::path::Path::new(
                r"D:\Apps de musica\Otomori\otomori.exe",
            ));
            let decoded = String::from_utf16(&command[..command.len() - 1]).unwrap();
            assert_eq!(
                decoded,
                r#""D:\Apps de musica\Otomori\otomori.exe" --startup"#
            );
        }
    }
}

#[cfg(windows)]
pub use platform::{is_enabled, migrar_do_morune, set_enabled};

#[cfg(not(windows))]
pub fn is_enabled() -> bool {
    false
}

#[cfg(not(windows))]
pub fn set_enabled(_enabled: bool) -> Result<(), std::io::Error> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "inicializacao automatica so esta disponivel no Windows",
    ))
}
