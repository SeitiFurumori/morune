//! Traducao dos erros da librespot para o vocabulario do core.
//!
//! A interface nunca ve um erro do Spotify: ela decide o que fazer a partir de
//! [`morune_core::CoreError::kind`] -- se pede login de novo, se oferece tentar
//! outra vez, ou se e definitivo. Manter essa traducao num lugar so e o que
//! garante que um erro novo nao vire "erro desconhecido" na tela.

use librespot_core::Error as LibrespotError;
use librespot_core::error::ErrorKind;
use morune_core::CoreError;

/// Converte um erro da librespot preservando a decisao que a UI precisa tomar.
pub fn from_librespot(error: LibrespotError) -> CoreError {
    let message = error.to_string();
    match error.kind {
        // O servidor recusou a credencial: nao adianta repetir, so logar de novo.
        ErrorKind::Unauthenticated | ErrorKind::PermissionDenied => CoreError::AuthExpired,
        ErrorKind::NotFound => CoreError::NotFound(message),
        ErrorKind::Unavailable | ErrorKind::DeadlineExceeded | ErrorKind::ResourceExhausted => {
            CoreError::Network(message)
        }
        ErrorKind::Cancelled | ErrorKind::Aborted => CoreError::Cancelled,
        ErrorKind::Unimplemented => CoreError::Unsupported("recurso nao suportado pelo Spotify"),
        _ => CoreError::InvalidState(message),
    }
}

/// Converte um erro do fluxo OAuth.
///
/// Tudo aqui e falha de login, mas a distincao entre "o usuario desistiu" e "a
/// rede caiu" muda o que a tela oferece a seguir.
pub fn from_oauth(error: librespot_oauth::OAuthError) -> CoreError {
    use librespot_oauth::OAuthError;
    match error {
        OAuthError::Recv | OAuthError::AuthCodeListenerTerminated => CoreError::Cancelled,
        // Porta ocupada nao e falha de rede, e mandar o usuario "verificar a
        // internet" o deixaria procurando no lugar errado. O endereco de
        // retorno e fixo -- tem de bater com o registrado no client ID -- entao
        // a unica saida e liberar a porta.
        OAuthError::AuthCodeListenerBind { addr, .. } => CoreError::InvalidState(format!(
            "a porta {addr} precisa estar livre para o login terminar, porque e \
             nela que o Spotify devolve a autorizacao. Feche o programa que \
             estiver usando essa porta e tente de novo"
        )),
        OAuthError::AuthCodeListenerRead
        | OAuthError::AuthCodeListenerWrite
        | OAuthError::AuthCodeListenerParse => CoreError::Network(error.to_string()),
        other => CoreError::InvalidState(other.to_string()),
    }
}
