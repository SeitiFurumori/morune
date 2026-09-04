//! Token de acesso do Spotify: obtencao, guarda e renovacao.
//!
//! Existe separado do login porque duas partes do backend precisam do mesmo
//! token e por motivos diferentes:
//!
//! - o **login** troca o codigo do navegador por um token e abre a sessao;
//! - o **catalogo** assina cada requisicao ao Web API com ele.
//!
//! Se cada uma guardasse o seu, a segunda renovaria por conta propria e a conta
//! acumularia tokens vivos sem necessidade. Aqui ha um so, renovado sob trava,
//! e quem pede um token vencido espera a mesma renovacao em vez de disparar
//! outra.
//!
//! O refresh token nunca toca o disco em texto: vai para o [`CredentialStore`],
//! que no Windows e o Gerenciador de Credenciais.

use std::sync::Arc;

use librespot_oauth::{OAuthClient, OAuthClientBuilder, OAuthToken};
use morune_core::auth::CredentialStore;
use morune_core::{CoreError, CoreResult};
use tokio::sync::Mutex;

use crate::error::from_oauth;

/// Client ID publico usado pelos clientes de desktop do Spotify.
///
/// Nao e segredo e nao pode ser: um `.exe` distribuido nao guarda segredo
/// nenhum. E por isso que o fluxo e PKCE, que dispensa client secret.
pub(crate) const CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";

/// Endereco de retorno do navegador.
///
/// Precisa ser fixo porque tem de bater com o que esta registrado no client ID
/// -- nao da para sortear porta a cada login.
pub(crate) const REDIRECT_URI: &str = "http://127.0.0.1:5588/login";

/// Permissoes pedidas ao usuario.
///
/// A lista e curta de proposito: cada escopo aqui e uma coisa que o Morune
/// consegue fazer com a conta dele, e a tela de consentimento mostra todas.
/// Nada de escrita enquanto o aplicativo nao souber escrever.
pub(crate) const SCOPES: &[&str] = &[
    "streaming",
    "user-read-email",
    "user-read-private",
    "user-library-read",
    "playlist-read-private",
    "playlist-read-collaborative",
    "user-top-read",
    "user-read-recently-played",
    "user-follow-read",
];

/// Chave do refresh token no cofre do sistema.
const REFRESH_KEY: &str = "spotify.refresh_token";

/// Pagina mostrada no navegador quando o login termina.
const BROWSER_MESSAGE: &str = concat!(
    "<!doctype html><meta charset=\"utf-8\"><title>Morune</title>",
    "<body style=\"font-family:system-ui;display:grid;place-items:center;",
    "height:100vh;margin:0;background:#0b0d12;color:#eceff4\">",
    "<div style=\"text-align:center\"><h1 style=\"font-weight:600\">Pronto</h1>",
    "<p>Pode fechar esta aba e voltar para o Morune.</p></div></body>"
);

/// Tempo maximo de espera pelo retorno do navegador.
///
/// Cinco minutos cobrem quem precisa digitar senha e segundo fator sem pressa.
/// O que ele evita e o caso real: a pessoa fecha o navegador sem concluir, e a
/// espera fica de pe para sempre segurando a porta.
const AUTH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

/// Autorizacao interativa em andamento.
///
/// **Por que este tipo existe.** O caminho interativo da librespot faz tudo de
/// uma vez, dentro de uma funcao que bloqueia: monta a URL, abre o navegador,
/// prende a porta e espera. Duas consequencias, as duas sentidas em uso real:
///
/// 1. A URL so vai para `println!`. Num build de release, que nao tem console,
///    ela e perdida no instante em que e escrita -- e era a unica forma de
///    reabrir a autorizacao noutro navegador. Quem abriu no perfil errado do
///    Chrome nao tinha para onde ir.
/// 2. A espera nao tem tempo limite nem cancelamento. Fechar o navegador antes
///    de concluir deixava a porta 5588 tomada ate o processo morrer, e toda
///    tentativa seguinte batia em "ja ha um login em andamento".
///
/// Aqui os dois passos ficam separados: montar a URL devolve **a URL**, e a
/// espera e um `await` cancelavel com prazo. Soltar este valor fecha a porta.
pub(crate) struct PendingAuth {
    /// O endereco que o usuario precisa abrir. E o que o contrato de
    /// `Authenticator::begin_login` sempre prometeu devolver.
    pub(crate) url: String,
    verifier: oauth2::PkceCodeVerifier,
    /// Valor anti-CSRF que o Spotify devolve intacto.
    ///
    /// Conferido na volta. O fluxo da librespot **nao** confere: ele aceita
    /// qualquer `code` que chegue na porta.
    csrf: oauth2::CsrfToken,
    listener: tokio::net::TcpListener,
}

impl std::fmt::Debug for PendingAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingAuth").finish_non_exhaustive()
    }
}

/// Fonte unica de token de acesso.
pub(crate) struct TokenSource {
    credentials: Arc<dyn CredentialStore>,
    /// Trava assincrona, e nao `std::sync::Mutex`: a renovacao faz rede
    /// enquanto a segura, e segurar uma trava sincrona atravessando `await`
    /// prenderia a thread do executor.
    current: Mutex<Option<OAuthToken>>,
}

impl std::fmt::Debug for TokenSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenSource").finish_non_exhaustive()
    }
}

impl TokenSource {
    pub(crate) fn new(credentials: Arc<dyn CredentialStore>) -> Self {
        Self {
            credentials,
            current: Mutex::new(None),
        }
    }

    /// Monta a autorizacao e comeca a escutar o retorno.
    ///
    /// Devolve assim que a URL existe -- **antes** de o usuario fazer qualquer
    /// coisa. E o que permite a tela mostrar o endereco e oferecer copia-lo: o
    /// navegador que abre sozinho pode ser o perfil errado, e sem o endereco
    /// nao haveria segunda chance.
    ///
    /// A porta e aberta aqui, e nao na espera, de proposito: se ela ja estiver
    /// tomada, o erro aparece agora, com o botao ainda na mao do usuario.
    pub(crate) async fn begin_interactive() -> CoreResult<PendingAuth> {
        use oauth2::{ClientId, CsrfToken, PkceCodeChallenge, RedirectUrl, Scope};

        let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
        // Construido aqui e de novo na troca, em vez de sair de uma funcao
        // compartilhada: no oauth2 5.0 o tipo do cliente carrega nos genericos
        // quais endpoints ja foram definidos, e escrever esse tipo num retorno
        // custa mais linha do que as tres chamadas que ele economiza.
        let client = oauth2::basic::BasicClient::new(ClientId::new(CLIENT_ID.to_string()))
            .set_auth_uri(auth_url()?)
            .set_token_uri(token_url()?)
            .set_redirect_uri(RedirectUrl::new(REDIRECT_URI.to_string()).map_err(url_error)?);

        let (url, csrf) = client
            .authorize_url(CsrfToken::new_random)
            .add_scopes(SCOPES.iter().map(|s| Scope::new((*s).to_string())))
            .set_pkce_challenge(challenge)
            .url();

        let endereco = redirect_socket_addr()?;
        let listener = tokio::net::TcpListener::bind(endereco).await.map_err(|e| {
            CoreError::InvalidState(format!(
                "a porta {endereco} ja esta em uso; feche o Morune pela bandeja e abra de novo ({e})"
            ))
        })?;

        Ok(PendingAuth {
            url: url.to_string(),
            verifier,
            csrf,
            listener,
        })
    }

    /// Troca o codigo do navegador por um token, encerrando a autorizacao.
    pub(crate) async fn finish_interactive(pending: PendingAuth) -> CoreResult<OAuthToken> {
        use oauth2::{AuthorizationCode, ClientId, RedirectUrl, TokenResponse};

        let codigo = pending.wait_for_code().await?;

        let client = oauth2::basic::BasicClient::new(ClientId::new(CLIENT_ID.to_string()))
            .set_auth_uri(auth_url()?)
            .set_token_uri(token_url()?)
            .set_redirect_uri(RedirectUrl::new(REDIRECT_URI.to_string()).map_err(url_error)?);

        // O mesmo `reqwest` que o resto do binario ja usa, com o TLS do
        // sistema. Um cliente proprio aqui traria outra pilha de TLS.
        let http = reqwest::Client::new();
        let resposta = client
            .exchange_code(AuthorizationCode::new(codigo))
            .set_pkce_verifier(pending.verifier)
            .request_async(&http)
            .await
            .map_err(|e| {
                CoreError::InvalidState(format!("o Spotify recusou a autorizacao: {e}"))
            })?;

        Ok(OAuthToken {
            access_token: resposta.access_token().secret().to_string(),
            refresh_token: resposta
                .refresh_token()
                .map(|t| t.secret().to_string())
                .unwrap_or_default(),
            expires_at: std::time::Instant::now()
                + resposta
                    .expires_in()
                    .unwrap_or_else(|| std::time::Duration::from_secs(3600)),
            token_type: format!("{:?}", resposta.token_type()),
            scopes: SCOPES.iter().map(|s| (*s).to_string()).collect(),
        })
    }

    /// Cliente OAuth silencioso, para renovar sem interromper o usuario.
    fn silent_client() -> CoreResult<OAuthClient> {
        Self::builder().build().map_err(from_oauth)
    }

    fn builder() -> OAuthClientBuilder {
        OAuthClientBuilder::new(CLIENT_ID, REDIRECT_URI, SCOPES.to_vec())
            .with_custom_message(BROWSER_MESSAGE)
    }

    /// Adota um token recem-obtido e guarda o refresh no cofre.
    pub(crate) async fn adopt(&self, token: OAuthToken) {
        self.persist(&token);
        *self.current.lock().await = Some(token);
    }

    /// Esquece o token em memoria e apaga o segredo guardado.
    pub(crate) async fn forget(&self) -> CoreResult<()> {
        *self.current.lock().await = None;
        self.credentials.delete(REFRESH_KEY)?;
        Ok(())
    }

    /// Apaga so o segredo guardado, sem tocar no token em memoria.
    ///
    /// Usado quando o refresh token e recusado: mante-lo faria toda abertura
    /// seguinte tentar o mesmo segredo morto.
    pub(crate) fn discard_stored(&self) {
        let _ = self.credentials.delete(REFRESH_KEY);
    }

    /// Refresh token guardado da ultima sessao, se houver.
    pub(crate) fn stored_refresh(&self) -> CoreResult<Option<String>> {
        let Some(bytes) = self.credentials.load(REFRESH_KEY)? else {
            return Ok(None);
        };
        String::from_utf8(bytes)
            .map(Some)
            .map_err(|_| CoreError::Storage("refresh token corrompido".into()))
    }

    /// Troca um refresh token por um token de acesso novo, sem adota-lo.
    ///
    /// A restauracao de sessao usa isto: se a troca falhar, quem chamou decide
    /// se apaga o segredo, e nao este modulo.
    pub(crate) async fn exchange(&self, refresh: &str) -> CoreResult<OAuthToken> {
        let token = Self::silent_client()?
            .refresh_token_async(refresh)
            .await
            .map_err(from_oauth)?;
        Ok(Self::keeping_refresh(token, refresh))
    }

    fn persist(&self, token: &OAuthToken) {
        if token.refresh_token.is_empty() {
            return;
        }
        if let Err(e) = self
            .credentials
            .store(REFRESH_KEY, token.refresh_token.as_bytes())
        {
            // Falhar aqui custa um login a mais na proxima abertura, e nada
            // mais: nao vale derrubar uma sessao que ja esta funcionando.
            tracing::warn!(error = %e, "nao foi possivel guardar o refresh token");
        }
    }

    /// Mantem o refresh token anterior quando a resposta nao traz um novo.
    ///
    /// O Spotify so devolve refresh token quando ele muda. Adotar a resposta
    /// crua apagaria o unico segredo que permite voltar sem login.
    fn keeping_refresh(mut token: OAuthToken, previous: &str) -> OAuthToken {
        if token.refresh_token.is_empty() {
            token.refresh_token = previous.to_string();
        }
        token
    }
}

/// Endereco de autorizacao do Spotify.
fn auth_url() -> CoreResult<oauth2::AuthUrl> {
    oauth2::AuthUrl::new("https://accounts.spotify.com/authorize".to_string()).map_err(url_error)
}

/// Endereco de troca de codigo por token.
fn token_url() -> CoreResult<oauth2::TokenUrl> {
    oauth2::TokenUrl::new("https://accounts.spotify.com/api/token".to_string()).map_err(url_error)
}

fn url_error(e: oauth2::url::ParseError) -> CoreError {
    CoreError::InvalidState(format!("endereco de OAuth invalido: {e}"))
}

/// O endereco de escuta, extraido do `REDIRECT_URI`.
///
/// Derivado, e nao escrito duas vezes: o endereco de retorno esta registrado no
/// client ID, e a porta tem de ser exatamente a dele.
fn redirect_socket_addr() -> CoreResult<std::net::SocketAddr> {
    let url = oauth2::url::Url::parse(REDIRECT_URI).map_err(url_error)?;
    let host = url.host_str().unwrap_or("127.0.0.1");
    let porta = url.port().unwrap_or(80);
    format!("{host}:{porta}")
        .parse()
        .map_err(|e| CoreError::InvalidState(format!("endereco de retorno invalido: {e}")))
}

impl PendingAuth {
    /// Abre o navegador padrao na URL da autorizacao.
    ///
    /// Falhar aqui **nao** e erro: a URL continua na tela para ser aberta a
    /// mao, que e justamente o caminho de quem tem mais de um navegador.
    pub(crate) fn open_browser(&self) {
        if let Err(e) = open::that_detached(&self.url) {
            tracing::warn!(error = %e, "nao foi possivel abrir o navegador");
        }
    }

    /// Espera o navegador voltar com o codigo.
    ///
    /// Com prazo: passado ele, a funcao devolve erro e **solta a porta**, que e
    /// a diferenca em relacao ao fluxo anterior. Cancelar a tarefa que aguarda
    /// tem o mesmo efeito, porque o `listener` morre junto.
    async fn wait_for_code(&self) -> CoreResult<String> {
        match tokio::time::timeout(AUTH_TIMEOUT, self.accept_code()).await {
            Ok(resultado) => resultado,
            // Estourou o prazo: para quem esta na frente da tela isso e um
            // login cancelado, que e a frase que a interface ja sabe mostrar.
            Err(_) => Err(CoreError::Cancelled),
        }
    }

    /// Aceita conexoes ate uma delas trazer o retorno da autorizacao.
    ///
    /// Um laco, e nao uma conexao so: o navegador costuma pedir o favicon na
    /// mesma porta, e atender apenas a primeira conexao gastaria a espera com
    /// um pedido que nao e o retorno.
    async fn accept_code(&self) -> CoreResult<String> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        loop {
            let (mut stream, _) = self.listener.accept().await.map_err(|e| {
                CoreError::Network(format!("o navegador nao conseguiu responder: {e}"))
            })?;

            let mut linha = String::new();
            {
                let mut leitor = BufReader::new(&mut stream);
                if leitor.read_line(&mut linha).await.is_err() {
                    continue;
                }
            }

            let Some(alvo) = linha.split_whitespace().nth(1) else {
                continue;
            };
            let Ok(url) = oauth2::url::Url::parse(&format!("http://localhost{alvo}")) else {
                continue;
            };

            let mut codigo = None;
            let mut estado = None;
            let mut erro = None;
            for (chave, valor) in url.query_pairs() {
                match chave.as_ref() {
                    "code" => codigo = Some(valor.into_owned()),
                    "state" => estado = Some(valor.into_owned()),
                    "error" => erro = Some(valor.into_owned()),
                    _ => {}
                }
            }

            // O usuario clicou em "cancelar" na tela do Spotify.
            if let Some(erro) = erro {
                let _ = responder(&mut stream, RECUSADO).await;
                tracing::info!(motivo = %erro, "autorizacao recusada no navegador");
                return Err(CoreError::Cancelled);
            }

            let (Some(codigo), Some(estado)) = (codigo, estado) else {
                // Favicon e afins: responde e continua esperando o retorno.
                let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\n\r\n").await;
                continue;
            };

            // Conferido, e o fluxo da librespot nao conferia: sem isto, um
            // codigo vindo de outra pagina aberta na mesma maquina seria aceito
            // como se fosse o nosso.
            if estado != *self.csrf.secret() {
                let _ = responder(&mut stream, RECUSADO).await;
                return Err(CoreError::InvalidState(
                    "a resposta do navegador nao corresponde a este login".into(),
                ));
            }

            let _ = responder(&mut stream, BROWSER_MESSAGE).await;
            return Ok(codigo);
        }
    }
}

/// Pagina mostrada quando a autorizacao e recusada ou nao confere.
const RECUSADO: &str = concat!(
    "<!doctype html><meta charset=\"utf-8\"><title>Morune</title>",
    "<body style=\"font-family:system-ui;display:grid;place-items:center;",
    "height:100vh;margin:0;background:#0b0d12;color:#eceff4\">",
    "<div style=\"text-align:center\"><h1 style=\"font-weight:600\">Login nao concluido</h1>",
    "<p>Volte ao Morune e tente de novo.</p></div></body>"
);

async fn responder(stream: &mut tokio::net::TcpStream, corpo: &str) -> std::io::Result<()> {
    use tokio::io::AsyncWriteExt;

    let resposta = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{corpo}",
        corpo.len()
    );
    stream.write_all(resposta.as_bytes()).await
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
    use morune_core::auth::MemoryCredentialStore;

    fn token(refresh: &str, valid_for: Duration) -> OAuthToken {
        OAuthToken {
            access_token: "acesso".into(),
            refresh_token: refresh.into(),
            expires_at: Instant::now() + valid_for,
            token_type: "Bearer".into(),
            scopes: SCOPES.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn source() -> (Arc<MemoryCredentialStore>, TokenSource) {
        let store = Arc::new(MemoryCredentialStore::default());
        (store.clone(), TokenSource::new(store))
    }

    #[tokio::test]
    async fn adopting_a_token_stores_the_refresh_secret() {
        let (store, tokens) = source();
        tokens
            .adopt(token("segredo", Duration::from_secs(3600)))
            .await;
        assert_eq!(
            store.load(REFRESH_KEY).unwrap().as_deref(),
            Some(&b"segredo"[..])
        );
    }

    #[tokio::test]
    async fn forgetting_clears_memory_and_the_vault() {
        let (store, tokens) = source();
        tokens
            .adopt(token("segredo", Duration::from_secs(3600)))
            .await;
        tokens.forget().await.unwrap();

        assert!(store.load(REFRESH_KEY).unwrap().is_none());
        assert!(tokens.stored_refresh().unwrap().is_none());
    }

    #[test]
    fn renewal_keeps_the_previous_refresh_token_when_none_comes_back() {
        let renewed = TokenSource::keeping_refresh(token("", Duration::from_secs(60)), "antigo");
        assert_eq!(renewed.refresh_token, "antigo");

        let rotated =
            TokenSource::keeping_refresh(token("novo", Duration::from_secs(60)), "antigo");
        assert_eq!(rotated.refresh_token, "novo");
    }

    #[test]
    fn the_oauth_client_builds_with_the_registered_redirect() {
        // Um erro de digitacao no endereco de retorno so apareceria na hora do
        // login, depois de abrir o navegador. Aqui aparece no teste.
        assert!(TokenSource::silent_client().is_ok());
        assert!(auth_url().is_ok());
        assert!(token_url().is_ok());
    }

    /// A porta que o ouvinte abre tem de ser a do endereco registrado no client
    /// ID. Derivar em vez de escrever duas vezes e o que garante isso, e este
    /// teste e o que garante a derivacao.
    #[test]
    fn a_porta_de_escuta_vem_do_endereco_de_retorno() {
        let addr = redirect_socket_addr().expect("endereco valido");
        assert_eq!(addr.port(), 5588);
        assert!(addr.ip().is_loopback(), "o retorno tem de ser local");
    }

    /// **O defeito que motivou o fluxo proprio.**
    ///
    /// No caminho da librespot, uma tentativa abandonada segurava a porta 5588
    /// ate o processo morrer: quem fechasse o navegador sem concluir ficava sem
    /// como tentar de novo. Aqui a autorizacao e um valor -- solta-la fecha o
    /// ouvinte --, e este teste prova isso pedindo a porta duas vezes seguidas.
    #[tokio::test]
    async fn desistir_libera_a_porta_para_a_proxima_tentativa() {
        let Ok(primeira) = TokenSource::begin_interactive().await else {
            // Porta ocupada por um Morune aberto; nao e falha deste teste.
            return;
        };
        drop(primeira);

        let segunda = TokenSource::begin_interactive().await;
        assert!(
            segunda.is_ok(),
            "a porta continuou tomada depois de desistir: {:?}",
            segunda.err()
        );
    }

    /// O caminho novo devolve a URL de autorizacao de verdade -- e nao o
    /// endereco de retorno, que era o que a versao anterior entregava.
    ///
    /// Abre a porta 5588, entao nao pode rodar junto com um login de verdade.
    #[tokio::test]
    async fn a_autorizacao_devolve_a_url_do_spotify() {
        let Ok(pendente) = TokenSource::begin_interactive().await else {
            // A porta pode estar ocupada por um Morune aberto na mesma maquina.
            // Isso nao e falha deste teste.
            return;
        };

        assert!(
            pendente
                .url
                .starts_with("https://accounts.spotify.com/authorize"),
            "url inesperada: {}",
            pendente.url
        );
        assert!(pendente.url.contains("code_challenge="), "sem PKCE");
        assert!(pendente.url.contains("state="), "sem protecao anti-CSRF");
        assert!(
            pendente.url.contains("client_id=") && pendente.url.contains(CLIENT_ID),
            "sem o client ID do Morune"
        );
    }

    #[test]
    fn scopes_never_ask_for_write_access() {
        // A tela de consentimento mostra cada escopo. Pedir escrita sem usar
        // custa confianca do usuario e nao entrega nada.
        for scope in SCOPES {
            assert!(
                !scope.contains("modify"),
                "escopo de escrita pedido sem uso: {scope}"
            );
        }
    }
}
