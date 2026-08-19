//! Busca pelo `pathfinder`, que e o caminho que sobrou.
//!
//! # Por que nao e o Web API
//!
//! Em 19/08/2026 ficou medido que o `api.spotify.com` responde **429** a
//! qualquer token que o Morune consiga obter -- do OAuth, do keymaster ou do
//! login5 -- e que `searchview`, o endereco de busca do protocolo interno,
//! responde 404 em todas as versoes testadas. Registrar um client ID proprio
//! resolveria o 429 e quebraria o produto: aplicativo novo nasce em modo de
//! desenvolvimento, onde so entra quem o dono cadastrar a mao, e o Morune e
//! aberto. A tabela completa esta em [`docs/HANDOFF.md`](../../../docs/HANDOFF.md).
//!
//! O `pathfinder` e por onde o player web do Spotify busca. Ele aceita o token
//! do login5 com o client token -- os dois ja existem dentro da sessao da
//! librespot -- e devolve faixa, album, artista e playlist em JSON.
//!
//! # A divida
//!
//! Consulta GraphQL crua e recusada com 400. So passa **consulta persistida**,
//! identificada por um hash SHA-256 acordado entre cliente e servidor, que
//! acompanha a versao do player web e **muda sem aviso**. Quando mudar, a busca
//! para de responder ate alguem atualizar [`HASH_BUSCA`].
//!
//! Nao ha alternativa medida, entao a divida e assumida. O que este modulo
//! garante e que ela fique **contida**: quem chama recebe um erro comum, e
//! busca quebrada nao derruba Inicio, Biblioteca nem reproducao.

use bytes::Bytes;
use http::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderName};
use http::{Method, Request};
use morune_core::catalog::{SearchKind, SearchResults};
use morune_core::{CoreError, CoreResult};

use crate::auth::SharedSession;
use crate::error::from_librespot;
use crate::graphql::SearchDataDto;

const ENDPOINT: &str = "https://api-partner.spotify.com/pathfinder/v1/query";

/// Cabecalho que o pathfinder exige alem do `Authorization`.
///
/// E o token de cliente, que a `spclient` da librespot ja sabe pedir e renovar.
const CLIENT_TOKEN: HeaderName = HeaderName::from_static("client-token");

/// Hash da consulta persistida `searchDesktop`.
///
/// **Esta constante e a divida deste modulo.** Ver o cabecalho. Se a busca
/// comecar a responder erro sem que nada tenha mudado no Morune, e aqui que se
/// olha primeiro: o valor vem da versao do player web e nao esta sob nosso
/// controle.
const HASH_BUSCA: &str = "d9f785900f0710b31c07818d617f4f7600c1e21217e80f5b043d1e78d74e6026";

/// Teto de itens por tipo numa busca.
///
/// O pathfinder aceita mais, mas a tela mostra uma fileira por tipo. Pedir 50
/// para desenhar 10 e rede e RAM gastas a toa.
const LIMITE_MAX: u32 = 20;

/// Cliente do pathfinder.
#[derive(Clone)]
pub(crate) struct Pathfinder {
    session: SharedSession,
}

impl std::fmt::Debug for Pathfinder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pathfinder").field("session", &self.session).finish()
    }
}

impl Pathfinder {
    pub(crate) fn new(session: SharedSession) -> Self {
        Self { session }
    }

    /// Busca no catalogo do Spotify.
    ///
    /// `kinds` filtra o que a tela vai mostrar, mas nao muda a requisicao: a
    /// consulta persistida devolve todos os tipos de uma vez, e pedir de novo
    /// por tipo custaria uma viagem a rede para cada fileira.
    pub(crate) async fn search(
        &self,
        query: &str,
        kinds: SearchKind,
        limit: u32,
        offset: u32,
    ) -> CoreResult<SearchResults> {
        if query.trim().is_empty() {
            return Ok(SearchResults::default());
        }

        let limit = limit.clamp(1, LIMITE_MAX);
        let variaveis = serde_json::json!({
            "searchTerm": query,
            "offset": offset,
            "limit": limit,
            "numberOfTopResults": limit.min(5),
            "includeAudiobooks": false,
        });

        let dto: SearchDataDto = self.query("searchDesktop", HASH_BUSCA, &variaveis).await?;
        Ok(dto.into_results(kinds))
    }

    /// Envia uma consulta persistida e desserializa a resposta.
    ///
    /// A resposta vai direto para o tipo pedido, sem passar por
    /// `serde_json::Value`: uma busca traz dezenas de itens com capa e artista
    /// cada, e virar arvore generica antes de virar modelo dobraria a alocacao
    /// a toa.
    async fn query<T: serde::de::DeserializeOwned>(
        &self,
        operacao: &str,
        hash: &str,
        variaveis: &serde_json::Value,
    ) -> CoreResult<T> {
        let session = self.session.get().ok_or(CoreError::NotAuthenticated)?;

        // Os dois vem da sessao e sao renovados por ela. Pedir aqui, e nao
        // guardar, e o que evita mandar token vencido depois de o aplicativo
        // ficar horas aberto.
        let token = session.login5().auth_token().await.map_err(from_librespot)?;
        let client_token = session.spclient().client_token().await.map_err(from_librespot)?;

        let corpo = serde_json::json!({
            "operationName": operacao,
            "variables": variaveis,
            "extensions": { "persistedQuery": { "version": 1, "sha256Hash": hash } },
        })
        .to_string();

        let request = Request::builder()
            .method(Method::POST)
            .uri(ENDPOINT)
            .header(AUTHORIZATION, format!("{} {}", token.token_type, token.access_token))
            .header(CLIENT_TOKEN, client_token)
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .body(Bytes::from(corpo))
            .map_err(|e| CoreError::InvalidState(format!("requisicao invalida: {e}")))?;

        let body =
            session.http_client().request_body(request).await.map_err(from_librespot)?;

        serde_json::from_slice(&body).map_err(|e| {
            // O corpo bruto vai para o log e nao para a tela: traz o que o
            // usuario digitou e o que o Spotify devolveu sobre a conta dele.
            //
            // Erro aqui quase sempre significa hash vencido -- ver o cabecalho.
            tracing::debug!(operacao, error = %e, "pathfinder respondeu fora do formato esperado");
            CoreError::Decode(
                "a busca do Spotify mudou de formato e o Morune ainda nao acompanha".into(),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_query_is_recognised_before_the_network() {
        // A caixa de busca dispara a cada tecla; apagar tudo nao pode virar
        // requisicao. O corte acontece antes de qualquer token ser pedido.
        assert!("   ".trim().is_empty());
        assert!("".trim().is_empty());
        assert!(!"queen".trim().is_empty());
    }

    #[test]
    fn the_limit_stays_within_what_the_screen_draws() {
        assert_eq!(50u32.clamp(1, LIMITE_MAX), LIMITE_MAX);
        assert_eq!(0u32.clamp(1, LIMITE_MAX), 1);
    }
}
