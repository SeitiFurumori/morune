use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::catalog::BoxFuture;
use crate::error::CoreResult;

/// Identidade da sessao ativa. Nunca contem segredo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    /// Codigo de pais usado pelo provedor para decidir disponibilidade.
    pub country: Option<String>,
    /// `true` quando a conta permite reproducao (Premium, no caso do Spotify).
    pub can_stream: bool,
}

/// Estado de autenticacao observavel pela UI.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum AuthState {
    #[default]
    LoggedOut,
    /// Fluxo em andamento; `url` e a pagina que o usuario precisa abrir.
    Pending {
        url: String,
    },
    LoggedIn(UserProfile),
    Failed(String),
}

/// Token de acesso emitido pelo provedor.
///
/// Deliberadamente sem `Debug` derivado e sem `Display`: um token impresso em
/// log e um vazamento de credencial. Use [`AccessToken::redacted`] quando
/// precisar registrar alguma coisa.
#[derive(Clone, Serialize, Deserialize)]
pub struct AccessToken {
    secret: String,
    pub expires_at: SystemTime,
    #[serde(default)]
    pub scopes: Vec<String>,
}

impl AccessToken {
    pub fn new(secret: impl Into<String>, expires_in: Duration, scopes: Vec<String>) -> Self {
        Self {
            secret: secret.into(),
            expires_at: SystemTime::now() + expires_in,
            scopes,
        }
    }

    /// Acesso explicito ao segredo. O nome longo e proposital: chamadas a este
    /// metodo devem saltar aos olhos em revisao de codigo.
    pub fn expose_secret(&self) -> &str {
        &self.secret
    }

    pub fn redacted(&self) -> String {
        format!(
            "<token {} chars, expira em {:?}>",
            self.secret.len(),
            self.remaining()
        )
    }

    pub fn remaining(&self) -> Duration {
        self.expires_at
            .duration_since(SystemTime::now())
            .unwrap_or(Duration::ZERO)
    }

    /// Considera expirado com margem, para nao emitir requisicao com token que
    /// morre no meio do caminho.
    pub fn is_expired(&self) -> bool {
        self.remaining() < Duration::from_secs(60)
    }
}

impl std::fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.redacted())
    }
}

/// Armazenamento de segredos do sistema operacional.
///
/// A implementacao real usa o Gerenciador de Credenciais do Windows. O contrato
/// fica no core para que nada acima dele precise saber disso, e para que testes
/// possam usar um cofre em memoria.
pub trait CredentialStore: Send + Sync + 'static {
    /// Grava um segredo sob uma chave logica ("spotify.refresh_token").
    fn store(&self, key: &str, secret: &[u8]) -> CoreResult<()>;
    fn load(&self, key: &str) -> CoreResult<Option<Vec<u8>>>;
    fn delete(&self, key: &str) -> CoreResult<()>;
}

/// Fluxo de autenticacao de um provedor.
///
/// O contrato nao tem espaco para senha de proposito: nenhuma implementacao
/// pode pedir ou receber a senha do usuario por aqui.
pub trait Authenticator: Send + Sync + 'static {
    fn name(&self) -> &'static str;

    /// Tenta restaurar a sessao a partir do segredo guardado. Retorna
    /// `Ok(None)` quando nao ha nada guardado -- isso nao e erro.
    fn restore(&self) -> BoxFuture<'_, CoreResult<Option<UserProfile>>>;

    /// Inicia o fluxo interativo e devolve a URL que o usuario deve abrir.
    fn begin_login(&self) -> BoxFuture<'_, CoreResult<String>>;

    /// Aguarda a conclusao do fluxo iniciado por [`Authenticator::begin_login`].
    fn complete_login(&self) -> BoxFuture<'_, CoreResult<UserProfile>>;

    /// Encerra a sessao e apaga qualquer segredo persistido.
    fn logout(&self) -> BoxFuture<'_, CoreResult<()>>;
}

/// Cofre em memoria, para teste e para o modo sem persistencia.
#[derive(Debug, Default)]
pub struct MemoryCredentialStore {
    entries: std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>,
}

impl CredentialStore for MemoryCredentialStore {
    fn store(&self, key: &str, secret: &[u8]) -> CoreResult<()> {
        self.entries
            .lock()
            .unwrap()
            .insert(key.to_string(), secret.to_vec());
        Ok(())
    }

    fn load(&self, key: &str) -> CoreResult<Option<Vec<u8>>> {
        Ok(self.entries.lock().unwrap().get(key).cloned())
    }

    fn delete(&self, key: &str) -> CoreResult<()> {
        self.entries.lock().unwrap().remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_debug_never_leaks_the_secret() {
        let t = AccessToken::new("super-secreto-123", Duration::from_secs(3600), vec![]);
        let printed = format!("{t:?}");
        assert!(!printed.contains("super-secreto-123"), "vazou: {printed}");
        assert!(printed.contains("17 chars"));
    }

    #[test]
    fn token_expiring_within_the_margin_counts_as_expired() {
        let fresh = AccessToken::new("x", Duration::from_secs(3600), vec![]);
        assert!(!fresh.is_expired());

        let almost = AccessToken::new("x", Duration::from_secs(30), vec![]);
        assert!(almost.is_expired());
    }

    #[test]
    fn memory_store_round_trips_and_deletes() {
        let s = MemoryCredentialStore::default();
        s.store("k", b"v").unwrap();
        assert_eq!(s.load("k").unwrap().as_deref(), Some(&b"v"[..]));
        s.delete("k").unwrap();
        assert!(s.load("k").unwrap().is_none());
    }

    #[test]
    fn missing_credential_is_not_an_error() {
        let s = MemoryCredentialStore::default();
        assert!(s.load("nao-existe").unwrap().is_none());
    }
}
