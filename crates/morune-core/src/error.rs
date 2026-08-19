use std::fmt;

/// Erro de dominio do core.
///
/// O core nao conhece Spotify, HTTP ou sistema de arquivos: backends concretos
/// mapeiam seus erros para estas variantes, e a UI so precisa lidar com estas.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("nao autenticado")]
    NotAuthenticated,

    #[error("credenciais invalidas ou expiradas")]
    AuthExpired,

    #[error("recurso nao encontrado: {0}")]
    NotFound(String),

    #[error("operacao nao suportada por este backend: {0}")]
    Unsupported(&'static str),

    #[error("falha de rede: {0}")]
    Network(String),

    #[error("falha no dispositivo de audio: {0}")]
    AudioDevice(String),

    #[error("falha ao decodificar audio: {0}")]
    Decode(String),

    #[error("armazenamento: {0}")]
    Storage(String),

    #[error("estado invalido: {0}")]
    InvalidState(String),

    #[error("cancelado")]
    Cancelled,
}

pub type CoreResult<T> = Result<T, CoreError>;

/// Classificacao usada pela UI para decidir se mostra retry, login ou erro fatal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Exige acao de login do usuario.
    Auth,
    /// Provavelmente transitorio; retry faz sentido.
    Transient,
    /// Permanente para esta requisicao; retry nao ajuda.
    Permanent,
}

impl CoreError {
    pub fn kind(&self) -> ErrorKind {
        match self {
            CoreError::NotAuthenticated | CoreError::AuthExpired => ErrorKind::Auth,
            CoreError::Network(_) | CoreError::AudioDevice(_) | CoreError::Cancelled => {
                ErrorKind::Transient
            }
            CoreError::NotFound(_)
            | CoreError::Unsupported(_)
            | CoreError::Decode(_)
            | CoreError::Storage(_)
            | CoreError::InvalidState(_) => ErrorKind::Permanent,
        }
    }

    pub fn is_retryable(&self) -> bool {
        self.kind() == ErrorKind::Transient
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Auth => f.write_str("auth"),
            ErrorKind::Transient => f.write_str("transient"),
            ErrorKind::Permanent => f.write_str("permanent"),
        }
    }
}
