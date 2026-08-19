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

    /// A conta existe e o login funcionou, mas o plano dela nao permite o que o
    /// aplicativo faz.
    ///
    /// Separado de [`CoreError::NotAuthenticated`] porque a acao do usuario e
    /// outra: nao adianta entrar de novo. A mensagem vem pronta do backend, que
    /// e quem sabe o nome do plano.
    #[error("{0}")]
    AccountPlan(String),

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
            CoreError::AccountPlan(_)
            | CoreError::NotFound(_)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_account_plan_problem_is_never_worth_retrying() {
        // Tentar de novo com a mesma conta da o mesmo resultado. A UI precisa
        // saber disso para nao oferecer "tentar de novo".
        let error = CoreError::AccountPlan("conta gratuita".into());
        assert_eq!(error.kind(), ErrorKind::Permanent);
        assert!(!error.is_retryable());
    }

    #[test]
    fn an_account_plan_problem_shows_the_message_the_backend_wrote() {
        // O core nao sabe dizer "Premium"; quem sabe e o backend, e a mensagem
        // dele tem de chegar inteira na tela.
        let error = CoreError::AccountPlan("O Spotify pede Premium.".into());
        assert_eq!(error.to_string(), "O Spotify pede Premium.");
    }

    #[test]
    fn expired_credentials_ask_for_a_new_login_and_not_a_retry() {
        assert_eq!(CoreError::AuthExpired.kind(), ErrorKind::Auth);
        assert_eq!(CoreError::Network("x".into()).kind(), ErrorKind::Transient);
    }
}
