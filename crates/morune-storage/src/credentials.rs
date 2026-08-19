//! Cofre de credenciais.
//!
//! No Windows usa o Gerenciador de Credenciais (`CredWriteW`/`CredReadW`), que
//! guarda o segredo cifrado com a chave do perfil do usuario. O aplicativo
//! nunca escreve token nem senha em arquivo proprio, e nunca ve a senha do
//! Spotify -- o fluxo de login e OAuth, ver `SECURITY.md`.

use morune_core::auth::CredentialStore;
use morune_core::error::{CoreError, CoreResult};

/// Prefixo das entradas criadas no cofre do sistema.
///
/// Aparece para o usuario no painel de credenciais do Windows, entao precisa
/// ser reconhecivel.
pub const TARGET_PREFIX: &str = "Morune";

fn target_name(key: &str) -> String {
    format!("{TARGET_PREFIX}:{key}")
}

/// Tamanho maximo de um segredo aceito pelo Gerenciador de Credenciais.
const MAX_SECRET_LEN: usize = 2560;

#[cfg(windows)]
mod windows_impl {
    use super::*;

    use std::ffi::c_void;

    use windows::core::PWSTR;
    use windows::Win32::Foundation::{ERROR_NOT_FOUND, FILETIME};
    use windows::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
        CRED_TYPE_GENERIC,
    };

    /// Cofre respaldado pelo Gerenciador de Credenciais do Windows.
    #[derive(Debug, Default)]
    pub struct WindowsCredentialStore;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    impl CredentialStore for WindowsCredentialStore {
        fn store(&self, key: &str, secret: &[u8]) -> CoreResult<()> {
            if secret.len() > MAX_SECRET_LEN {
                return Err(CoreError::Storage(format!(
                    "segredo de {} bytes excede o limite de {MAX_SECRET_LEN}",
                    secret.len()
                )));
            }

            let mut target = wide(&target_name(key));
            let mut comment = wide("Credencial do cliente de musica Morune");

            let credential = CREDENTIALW {
                Flags: Default::default(),
                Type: CRED_TYPE_GENERIC,
                TargetName: PWSTR(target.as_mut_ptr()),
                Comment: PWSTR(comment.as_mut_ptr()),
                LastWritten: FILETIME::default(),
                CredentialBlobSize: secret.len() as u32,
                CredentialBlob: secret.as_ptr() as *mut u8,
                Persist: CRED_PERSIST_LOCAL_MACHINE,
                AttributeCount: 0,
                Attributes: std::ptr::null_mut(),
                TargetAlias: PWSTR::null(),
                UserName: PWSTR::null(),
            };

            // SAFETY: `credential` aponta para buffers vivos durante a chamada;
            // a API copia o conteudo antes de retornar.
            unsafe { CredWriteW(&credential, 0) }
                .map_err(|e| CoreError::Storage(format!("CredWriteW falhou: {e}")))
        }

        fn load(&self, key: &str) -> CoreResult<Option<Vec<u8>>> {
            let target = wide(&target_name(key));
            let mut raw: *mut CREDENTIALW = std::ptr::null_mut();

            // SAFETY: `target` e uma string terminada em nulo valida; em caso
            // de sucesso `raw` recebe um ponteiro que liberamos com CredFree.
            let result = unsafe { CredReadW(PWSTR(target.as_ptr() as *mut u16), CRED_TYPE_GENERIC, None, &mut raw) };

            if let Err(e) = result {
                return if e.code() == ERROR_NOT_FOUND.to_hresult() {
                    Ok(None)
                } else {
                    Err(CoreError::Storage(format!("CredReadW falhou: {e}")))
                };
            }

            // SAFETY: `raw` e valido apos sucesso; copiamos antes de liberar.
            let secret = unsafe {
                let cred = &*raw;
                let bytes = std::slice::from_raw_parts(
                    cred.CredentialBlob,
                    cred.CredentialBlobSize as usize,
                )
                .to_vec();
                CredFree(raw as *const c_void);
                bytes
            };

            Ok(Some(secret))
        }

        fn delete(&self, key: &str) -> CoreResult<()> {
            let target = wide(&target_name(key));
            // SAFETY: string valida terminada em nulo.
            let result = unsafe { CredDeleteW(PWSTR(target.as_ptr() as *mut u16), CRED_TYPE_GENERIC, None) };
            match result {
                Ok(()) => Ok(()),
                // Apagar o que nao existe e sucesso: o estado desejado ja vale.
                Err(e) if e.code() == ERROR_NOT_FOUND.to_hresult() => Ok(()),
                Err(e) => Err(CoreError::Storage(format!("CredDeleteW falhou: {e}"))),
            }
        }
    }
}

#[cfg(windows)]
pub use windows_impl::WindowsCredentialStore;

/// Cofre padrao da plataforma.
///
/// Fora do Windows cai para o cofre em memoria: a sessao nao sobrevive ao
/// fechamento do aplicativo, o que e preferivel a gravar um token em texto
/// claro so para manter a conveniencia.
pub fn platform_store() -> Box<dyn CredentialStore> {
    #[cfg(windows)]
    {
        Box::new(WindowsCredentialStore)
    }
    #[cfg(not(windows))]
    {
        tracing::warn!("plataforma sem cofre de credenciais; a sessao nao sera lembrada");
        Box::new(morune_core::auth::MemoryCredentialStore::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_names_are_namespaced() {
        assert_eq!(target_name("spotify.refresh"), "Morune:spotify.refresh");
    }

    #[cfg(windows)]
    mod windows_store {
        use super::*;

        /// Chave exclusiva por processo, para que execucoes em paralelo dos
        /// testes nao disputem a mesma entrada do cofre real.
        fn key() -> String {
            format!("test.{}", std::process::id())
        }

        #[test]
        fn round_trips_a_secret_through_the_system_vault() {
            let store = WindowsCredentialStore;
            let k = key();
            let _ = store.delete(&k);

            store.store(&k, b"token-de-teste").unwrap();
            assert_eq!(store.load(&k).unwrap().as_deref(), Some(&b"token-de-teste"[..]));

            store.delete(&k).unwrap();
            assert!(store.load(&k).unwrap().is_none());
        }

        #[test]
        fn deleting_a_missing_entry_is_not_an_error() {
            let store = WindowsCredentialStore;
            assert!(store.delete(&format!("{}-inexistente", key())).is_ok());
        }

        #[test]
        fn overwriting_replaces_the_previous_secret() {
            let store = WindowsCredentialStore;
            let k = format!("{}-overwrite", key());
            store.store(&k, b"antigo").unwrap();
            store.store(&k, b"novo").unwrap();
            assert_eq!(store.load(&k).unwrap().as_deref(), Some(&b"novo"[..]));
            let _ = store.delete(&k);
        }

        #[test]
        fn oversized_secrets_are_refused_before_touching_the_vault() {
            let store = WindowsCredentialStore;
            let huge = vec![0u8; MAX_SECRET_LEN + 1];
            assert!(store.store(&format!("{}-huge", key()), &huge).is_err());
        }
    }
}
