//! Busca e biblioteca do usuario sobre o Web API do Spotify.
//!
//! O que este modulo faz e montar caminho, pedir e traduzir -- a traducao mora
//! em [`crate::dto`] e o transporte em [`crate::webapi`]. Nao ha cache aqui, de
//! proposito: o contrato de [`Catalog`] diz que cache e da camada de
//! armazenamento, e um cache escondido dentro do provedor seria impossivel de
//! limpar sem sair do lugar.
//!
//! Duas coisas nao sao negociaveis nas paginas:
//!
//! - **teto de itens por requisicao.** O Spotify aceita ate 50 (100 nas faixas
//!   de playlist) e recusa mais que isso com erro. Pedir mais e trocar dado por
//!   erro;
//! - **playlist grande nunca vem inteira.** A tela pede por faixa visivel; e o
//!   maior risco de RAM do aplicativo e o motivo de `playlist_tracks` existir
//!   separado de `playlist`.

use std::sync::Arc;

use morune_core::catalog::{
    BoxFuture, Catalog, Library, Page, SearchKind, SearchResults, TopRange,
};
use morune_core::model::{
    Album, AlbumId, Artist, ArtistId, Playlist, PlaylistId, PlaylistKind, Provider, Track, TrackId,
};
use morune_core::{CoreError, CoreResult};

use crate::auth::SharedSession;
use crate::internal::{ArtistMeta, Conjunto, Internal, PlaylistSummary, TrackMeta};
use crate::pathfinder::Pathfinder;

/// Maximo de itens por pagina aceito pela maioria dos endpoints.
const MAX_PAGE: u32 = 50;

/// Maximo aceito pelas faixas de uma playlist.
const MAX_PLAYLIST_PAGE: u32 = 100;

/// Recusa das prateleiras que perderam o caminho com o Web API.
///
/// A interface trata `Unsupported` escondendo a secao, em vez de mostrar erro:
/// e o unico dos dois que nao mente. Ver `docs/HANDOFF.md` para o que foi
/// medido e nao respondeu.
const SEM_CAMINHO: &str = "o Spotify nao expoe mais isto por nenhum caminho que o Morune alcance";

/// Catalogo e biblioteca do Spotify.
///
/// Implementa os dois contratos porque falam com o mesmo servidor com o mesmo
/// token; separa-los em dois tipos duplicaria a fiacao sem separar nada de
/// verdade.
#[derive(Debug)]
pub struct SpotifyCatalog {
    /// Caminho interno, para o que a librespot ja fala nativamente.
    internal: Internal,
    /// Busca. E o unico caminho que sobrou depois que o Web API fechou --
    /// ver `crate::pathfinder`.
    pathfinder: Pathfinder,
}

impl SpotifyCatalog {
    pub(crate) fn new(session: SharedSession) -> Self {
        Self {
            internal: Internal::new(session.clone()),
            pathfinder: Pathfinder::new(session),
        }
    }

    /// Metadado de varias faixas de uma vez.
    ///
    /// Vai pelo `extended-metadata` do protocolo interno. O `/v1/tracks?ids=`
    /// do Web API fazia o mesmo e morreu -- ver `crate::pathfinder` para a
    /// medicao.
    async fn tracks_by_id(&self, ids: &[String]) -> CoreResult<Vec<Track>> {
        Ok(self
            .internal
            .tracks(ids)
            .await?
            .into_iter()
            .map(meta_to_track)
            .collect())
    }

    /// Playlist pelo caminho interno, quando o Web API recusa.
    ///
    /// Descobertas da Semana e Radar de Novidades respondem 404 em
    /// `/v1/playlists/{id}` desde a mudanca de 2024. Pelo protocolo que o
    /// cliente oficial usa, elas continuam existindo.
    async fn playlist_from_internal(&self, id: &str) -> CoreResult<Playlist> {
        let contents = self.internal.playlist(id).await?;
        let total = contents.track_ids.len() as u32;

        Ok(Playlist {
            id: PlaylistId::spotify(id),
            kind: PlaylistKind::Personal,
            name: Arc::from(contents.name.as_str()),
            owner: None,
            description: None,
            images: Default::default(),
            total_tracks: Some(total),
            // O contrato tem `playlist_tracks` justamente para nao baixar uma
            // playlist inteira ao abrir. Quem precisa das faixas pagina.
            tracks: Vec::new(),
        })
    }

    /// Todas as playlists da conta, ja classificadas.
    ///
    /// Uma requisicao so, e a tela decide o que fazer com cada tipo: as pessoais
    /// vao para a barra lateral, as geradas para o Inicio, e as de vitrine nao
    /// aparecem em prateleira nenhuma. Separar isso em tres chamadas custaria
    /// tres rootlists identicos.
    async fn all_playlists(&self, limit: u32) -> CoreResult<Page<Playlist>> {
        let todas = self.internal.rootlist().await?;
        let total = Some(todas.len() as u32);

        let items: Vec<Playlist> = todas
            .iter()
            .take(limit as usize)
            .map(summary_to_playlist)
            .collect();

        Ok(Page {
            items,
            offset: 0,
            total,
        })
    }

    /// Busca no catalogo.
    ///
    /// Vai pelo pathfinder, e nao pelo Web API: ver `crate::pathfinder` para a
    /// medicao que levou a isso e para a divida que o caminho traz.
    async fn search_all(
        &self,
        query: &str,
        kind: SearchKind,
        limit: u32,
    ) -> CoreResult<SearchResults> {
        self.pathfinder.search(query, kind, limit, 0).await
    }
}

impl Catalog for SpotifyCatalog {
    fn name(&self) -> &'static str {
        "spotify"
    }

    fn search<'a>(
        &'a self,
        query: &'a str,
        kind: SearchKind,
        limit: u32,
    ) -> BoxFuture<'a, CoreResult<SearchResults>> {
        Box::pin(self.search_all(query, kind, limit))
    }

    fn track<'a>(&'a self, id: &'a TrackId) -> BoxFuture<'a, CoreResult<Track>> {
        Box::pin(async move {
            let id = spotify_id(id.provider, &id.id)?;
            self.tracks_by_id(std::slice::from_ref(&id.to_string()))
                .await?
                .pop()
                .ok_or_else(|| CoreError::NotFound(format!("faixa {id}")))
        })
    }

    fn album<'a>(&'a self, id: &'a AlbumId) -> BoxFuture<'a, CoreResult<Album>> {
        Box::pin(async move {
            let id = spotify_id(id.provider, &id.id)?;
            let meta = self.internal.album(id).await?;

            Ok(Album {
                id: AlbumId::spotify(meta.id.as_str()),
                name: Arc::from(meta.name.as_str()),
                artists: artist_refs(meta.artists),
                images: meta.images,
                release_date: meta.year.map(|y| Arc::from(y.to_string().as_str())),
                total_tracks: Some(meta.tracks.len() as u32),
                tracks: meta.tracks.into_iter().map(meta_to_track).collect(),
            })
        })
    }

    /// Artista, sem discografia nem faixas populares.
    ///
    /// As duas vinham de `/v1/artists/{id}/albums` e `/top-tracks`, que morreram
    /// com o resto do Web API. O protobuf do artista traz os grupos de album,
    /// mas so como referencia -- montar a discografia exigiria um pedido de
    /// metadado por album, e isso ainda nao foi medido. Ver docs/HANDOFF.md.
    fn artist<'a>(&'a self, id: &'a ArtistId) -> BoxFuture<'a, CoreResult<Artist>> {
        Box::pin(async move {
            let id = spotify_id(id.provider, &id.id)?;
            Ok(meta_to_artist(self.internal.artist(id).await?))
        })
    }

    fn playlist<'a>(&'a self, id: &'a PlaylistId) -> BoxFuture<'a, CoreResult<Playlist>> {
        Box::pin(async move {
            let id = spotify_id(id.provider, &id.id)?;
            self.playlist_from_internal(id).await
        })
    }

    /// Faixas de uma playlist, paginadas.
    ///
    /// O caminho interno entrega a lista de ids inteira numa requisicao, e o
    /// recorte acontece aqui. So a fatia pedida vira requisicao de metadado --
    /// que e o que mantem de pe a regra de nunca carregar playlist grande
    /// inteira, mesmo com a lista de ids toda em maos.
    fn playlist_tracks<'a>(
        &'a self,
        id: &'a PlaylistId,
        offset: u32,
        limit: u32,
    ) -> BoxFuture<'a, CoreResult<Page<Track>>> {
        Box::pin(async move {
            let id = spotify_id(id.provider, &id.id)?;
            let contents = self.internal.playlist(id).await?;
            let total = Some(contents.track_ids.len() as u32);

            let fatia: Vec<String> = contents
                .track_ids
                .into_iter()
                .skip(offset as usize)
                .take(clamp(limit, MAX_PLAYLIST_PAGE) as usize)
                .collect();

            Ok(Page {
                items: self.tracks_by_id(&fatia).await?,
                offset,
                total,
            })
        })
    }

    fn radio<'a>(
        &'a self,
        seed: &'a TrackId,
        limit: u32,
    ) -> BoxFuture<'a, CoreResult<Page<Track>>> {
        Box::pin(async move {
            let seed = spotify_id(seed.provider, &seed.id)?;
            let playlist_id = self.internal.radio_playlist(seed).await?;
            let contents = self.internal.playlist(&playlist_id).await?;
            let total = Some(contents.track_ids.len() as u32);
            let ids: Vec<String> = contents
                .track_ids
                .into_iter()
                .take(clamp(limit, MAX_PLAYLIST_PAGE) as usize)
                .collect();
            Ok(Page {
                items: self.tracks_by_id(&ids).await?,
                offset: 0,
                total,
            })
        })
    }
}

impl Library for SpotifyCatalog {
    fn name(&self) -> &'static str {
        "spotify"
    }

    /// Playlists da conta, pelo rootlist.
    ///
    /// O `/v1/me/playlists` morreu junto com o resto do Web API. O rootlist ja
    /// era usado pela prateleira "Feito para voce" e traz a lista inteira numa
    /// requisicao so, decorada com nome, dono e tamanho -- entao a Biblioteca
    /// passa pelo mesmo caminho e a pagina e recortada aqui.
    fn saved_playlists<'a>(
        &'a self,
        offset: u32,
        limit: u32,
    ) -> BoxFuture<'a, CoreResult<Page<Playlist>>> {
        Box::pin(async move {
            let todas = self.internal.rootlist().await?;
            let total = Some(todas.len() as u32);

            let items: Vec<Playlist> = todas
                .iter()
                .skip(offset as usize)
                .take(limit as usize)
                .map(summary_to_playlist)
                .collect();

            Ok(Page {
                items,
                offset,
                total,
            })
        })
    }

    /// Albuns salvos -- **sem caminho conhecido**.
    ///
    /// Vinha de `/v1/me/albums`. O Web API fechou, e nem
    /// `hm://collection/album/{usuario}` nem `collection/v2/paging` responderam
    /// na sonda de 19/08/2026. Recusar explicitamente e melhor que devolver
    /// lista vazia: vazio diria ao usuario que ele nao salvou nenhum album.
    fn saved_albums<'a>(
        &'a self,
        _offset: u32,
        _limit: u32,
    ) -> BoxFuture<'a, CoreResult<Page<Album>>> {
        Box::pin(async { Err(CoreError::Unsupported(SEM_CAMINHO)) })
    }

    /// Musicas curtidas, pela colecao do protocolo interno.
    ///
    /// A colecao devolve so ids, e de uma vez -- nao ha paginacao no caminho.
    /// O recorte acontece aqui, e so a fatia pedida vira requisicao de
    /// metadado: numa conta com 723 curtidas, pedir as 723 para desenhar 50
    /// seria quinze vezes mais rede do que a tela usa.
    fn saved_tracks<'a>(
        &'a self,
        offset: u32,
        limit: u32,
    ) -> BoxFuture<'a, CoreResult<Page<Track>>> {
        Box::pin(async move {
            let ids = self.internal.collection(Conjunto::Faixas).await?;
            let total = Some(ids.len() as u32);

            let fatia: Vec<String> = ids
                .into_iter()
                .skip(offset as usize)
                .take(clamp(limit, MAX_PAGE) as usize)
                .collect();

            Ok(Page {
                items: self.tracks_by_id(&fatia).await?,
                offset,
                total,
            })
        })
    }

    /// Artistas seguidos, pela colecao do protocolo interno.
    ///
    /// O `/v1/me/following` era paginado por cursor e obrigava a caminhar ate o
    /// deslocamento pedido. A colecao devolve a lista inteira de uma vez, entao
    /// o cursor -- e o teto de requisicoes que existia para conte-lo -- deixou
    /// de ser necessario.
    fn followed_artists<'a>(
        &'a self,
        offset: u32,
        limit: u32,
    ) -> BoxFuture<'a, CoreResult<Page<Artist>>> {
        Box::pin(async move {
            let ids = self.internal.collection(Conjunto::Artistas).await?;
            let total = Some(ids.len() as u32);

            let items = self
                .internal
                .artists(
                    &ids.into_iter()
                        .skip(offset as usize)
                        .take(clamp(limit, MAX_PAGE) as usize)
                        .collect::<Vec<_>>(),
                )
                .await?
                .into_iter()
                .map(meta_to_artist)
                .collect();

            Ok(Page {
                items,
                offset,
                total,
            })
        })
    }

    /// Mais ouvidos -- **sem caminho conhecido**.
    ///
    /// Vinha de `/v1/me/top/*`. Nao ha equivalente medido no protocolo interno.
    fn top_tracks<'a>(
        &'a self,
        _range: TopRange,
        _offset: u32,
        _limit: u32,
    ) -> BoxFuture<'a, CoreResult<Page<Track>>> {
        Box::pin(async { Err(CoreError::Unsupported(SEM_CAMINHO)) })
    }

    fn top_artists<'a>(
        &'a self,
        _range: TopRange,
        _offset: u32,
        _limit: u32,
    ) -> BoxFuture<'a, CoreResult<Page<Artist>>> {
        Box::pin(async { Err(CoreError::Unsupported(SEM_CAMINHO)) })
    }

    /// Todas as playlists, classificadas. Ver [`SpotifyCatalog::all_playlists`].
    ///
    /// O nome do contrato envelheceu: hoje isto devolve tudo, e a tela e que
    /// escolhe. Trocar o nome mexeria no contrato do core por causa de um
    /// detalhe de quem chama.
    fn made_for_you<'a>(&'a self, limit: u32) -> BoxFuture<'a, CoreResult<Page<Playlist>>> {
        Box::pin(self.all_playlists(limit))
    }

    /// Tocadas recentemente -- **sem caminho conhecido**.
    ///
    /// Vinha de `/v1/me/player/recently-played`. `play-history/v1` e
    /// `recently-played/v3` responderam 404 na sonda.
    fn recently_played<'a>(&'a self, _limit: u32) -> BoxFuture<'a, CoreResult<Page<Track>>> {
        Box::pin(async { Err(CoreError::Unsupported(SEM_CAMINHO)) })
    }
}

/// Mantem o pedido dentro do que o endpoint aceita.
///
/// Pedir acima do teto nao devolve menos itens: devolve erro. Cortar aqui e a
/// diferenca entre uma lista curta e uma tela de falha.
fn clamp(limit: u32, max: u32) -> u32 {
    limit.clamp(1, max)
}

/// Recorta um identificador do Spotify.
///
/// Ids vem de resposta do servidor ou de cache em disco. Um id com `/` ou `?`
/// mudaria o endereco chamado, entao qualquer coisa fora do alfabeto base62 e
/// recusada antes de virar requisicao.
fn checked_id(id: &str) -> CoreResult<&str> {
    if !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric()) {
        Ok(id)
    } else {
        Err(CoreError::NotFound(format!("identificador invalido: {id}")))
    }
}

/// Id do Spotify pronto para entrar num caminho de URL.
fn spotify_id(provider: Provider, id: &str) -> CoreResult<&str> {
    if provider != Provider::Spotify {
        return Err(CoreError::NotFound(format!(
            "{} nao e um recurso do Spotify",
            provider.as_str()
        )));
    }
    checked_id(id)
}

/// Referencias de artista vindas do protocolo interno.
fn artist_refs(pares: Vec<(String, String)>) -> Vec<morune_core::model::ArtistRef> {
    pares
        .into_iter()
        .map(|(id, nome)| morune_core::model::ArtistRef {
            id: ArtistId::spotify(id.as_str()),
            name: Arc::from(nome.as_str()),
        })
        .collect()
}

/// Artista do protocolo interno no modelo do core.
///
/// Sem `top_tracks` nem discografia: a Biblioteca so desenha nome e capa, e
/// buscar as duas para cada linha custaria dezenas de requisicoes numa tela que
/// nao as mostra. A tela do artista as pede quando abre.
fn meta_to_artist(meta: ArtistMeta) -> Artist {
    Artist {
        id: ArtistId::spotify(meta.id.as_str()),
        name: Arc::from(meta.name.as_str()),
        images: meta.images,
        genres: meta.genres.iter().map(|g| Arc::from(g.as_str())).collect(),
        top_tracks: meta.top_tracks.into_iter().map(meta_to_track).collect(),
        albums: meta
            .albums
            .into_iter()
            .map(|(id, name, images)| morune_core::model::AlbumRef {
                id: AlbumId::spotify(id.as_str()),
                name: Arc::from(name.as_str()),
                images,
            })
            .collect(),
    }
}

/// Faixa do protocolo interno no modelo do core.
///
/// `playable` fica `true`: o protocolo interno nao informa disponibilidade por
/// mercado, e marcar tudo como indisponivel esconderia a biblioteca inteira.
/// Faixa que o Spotify recusar na hora de tocar vira erro de reproducao, que a
/// interface ja sabe mostrar.
fn meta_to_track(meta: TrackMeta) -> Track {
    Track {
        id: TrackId::spotify(meta.id.as_str()),
        name: Arc::from(meta.name.as_str()),
        artists: artist_refs(meta.artists),
        album: meta
            .album
            .map(|(id, nome, capas)| morune_core::model::AlbumRef {
                id: AlbumId::spotify(id.as_str()),
                name: Arc::from(nome.as_str()),
                images: capas,
            }),
        duration: std::time::Duration::from_millis(meta.duration_ms),
        track_number: meta.number,
        disc_number: meta.disc,
        explicit: meta.explicit,
        playable: true,
    }
}

/// Converte o resumo do rootlist no modelo do core.
///
/// Sem faixas e sem capa: o rootlist descreve a playlist, nao o conteudo dela.
/// As faixas chegam quando o usuario abre.
fn summary_to_playlist(summary: &PlaylistSummary) -> Playlist {
    Playlist {
        id: summary.id.clone(),
        kind: summary.kind(),
        name: Arc::from(summary.name.as_str()),
        owner: Some(summary.owner.as_str())
            .filter(|o| !o.is_empty())
            .map(Arc::from),
        description: None,
        images: summary.images.clone(),
        total_tracks: Some(summary.length),
        tracks: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_request_never_asks_for_more_than_the_server_accepts() {
        // Acima do teto o Spotify responde erro, e a tela fica vazia em vez de
        // curta -- o pior dos dois resultados.
        assert_eq!(clamp(500, MAX_PAGE), 50);
        assert_eq!(clamp(0, MAX_PAGE), 1);
        assert_eq!(clamp(20, MAX_PAGE), 20);
        assert_eq!(clamp(200, MAX_PLAYLIST_PAGE), 100);
    }

    #[test]
    fn ids_from_another_provider_are_refused_before_reaching_the_network() {
        // Um id local vindo de um `.musicpack` nao pode virar uma requisicao ao
        // Spotify -- responderia 404 depois de uma ida a rede.
        assert!(spotify_id(Provider::Local, "musica.flac").is_err());
        assert_eq!(
            spotify_id(Provider::Spotify, "4cOdK2wGLETK").unwrap(),
            "4cOdK2wGLETK"
        );
    }
}
