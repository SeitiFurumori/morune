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

use morune_core::catalog::{BoxFuture, Catalog, Library, Page, SearchKind, SearchResults};
use morune_core::model::{
    Album, AlbumId, Artist, ArtistId, Playlist, PlaylistId, Provider, Track, TrackId,
};
use morune_core::{CoreError, CoreResult};

use crate::auth::SharedSession;
use crate::dto::{
    AlbumDto, ArtistDto, FollowedArtistsDto, Paged, PlaylistDto, PlaylistItemDto, SavedAlbumDto,
    SavedTrackDto, SearchDto, TopTracksDto, TrackDto,
};
use crate::token::TokenSource;
use crate::webapi::{WebApi, checked_id, escape};

/// Maximo de itens por pagina aceito pela maioria dos endpoints.
const MAX_PAGE: u32 = 50;

/// Maximo aceito pelas faixas de uma playlist.
const MAX_PLAYLIST_PAGE: u32 = 100;

/// Teto de requisicoes ao caminhar por cursor.
///
/// `/v1/me/following` nao aceita deslocamento: para chegar ao item 200 e
/// preciso passar pelos 199 anteriores. O teto existe para que um pedido de
/// deslocamento absurdo termine em resposta vazia em vez de em centenas de
/// requisicoes.
const MAX_CURSOR_PAGES: u32 = 40;

/// Catalogo e biblioteca do Spotify.
///
/// Implementa os dois contratos porque falam com o mesmo servidor com o mesmo
/// token; separa-los em dois tipos duplicaria a fiacao sem separar nada de
/// verdade.
#[derive(Debug)]
pub struct SpotifyCatalog {
    api: WebApi,
}

impl SpotifyCatalog {
    pub(crate) fn new(session: SharedSession, tokens: Arc<TokenSource>) -> Self {
        Self { api: WebApi::new(session, tokens) }
    }

    /// Mercado da conta como par de query, ou vazio quando desconhecido.
    fn market(&self) -> Option<(&'static str, String)> {
        self.api.market().map(|m| ("market", m))
    }

    async fn search_all(
        &self,
        query: &str,
        kind: SearchKind,
        limit: u32,
    ) -> CoreResult<SearchResults> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(SearchResults::default());
        }

        let mut params = vec![
            ("q", escape(query)),
            ("type", kinds(kind).to_string()),
            ("limit", clamp(limit, MAX_PAGE).to_string()),
        ];
        params.extend(self.market());

        let dto: SearchDto = self.api.get(&path("/v1/search", &params)).await?;
        Ok(SearchResults {
            tracks: map_page(dto.tracks, TrackDto::into_track),
            albums: map_page(dto.albums, AlbumDto::into_album),
            artists: map_page(dto.artists, ArtistDto::into_artist),
            playlists: map_page(dto.playlists, PlaylistDto::into_playlist),
        })
    }

    /// Artista com o que a tela dele mostra: capa, generos, top e discografia.
    ///
    /// Sao tres requisicoes porque o Web API nao junta as tres, e nao ha como
    /// desenhar a tela sem as tres. Falha nas complementares nao derruba o
    /// artista: um artista sem discografia ainda e melhor que uma tela de erro.
    async fn artist_detail(&self, id: &str) -> CoreResult<Artist> {
        let dto: ArtistDto = self.api.get(&format!("/v1/artists/{id}")).await?;
        let mut artist = dto
            .into_artist()
            .ok_or_else(|| CoreError::NotFound(format!("artista {id}")))?;

        let mut params = vec![];
        params.extend(self.market());
        let top: CoreResult<TopTracksDto> =
            self.api.get(&path(&format!("/v1/artists/{id}/top-tracks"), &params)).await;
        match top {
            Ok(top) => {
                artist.top_tracks =
                    top.tracks.into_iter().flatten().filter_map(TrackDto::into_track).collect()
            }
            Err(e) => tracing::debug!(artist = id, error = %e, "sem faixas populares"),
        }

        params.push(("limit", MAX_PAGE.to_string()));
        params.push(("include_groups", "album,single".into()));
        let albums: CoreResult<Paged<AlbumDto>> =
            self.api.get(&path(&format!("/v1/artists/{id}/albums"), &params)).await;
        match albums {
            Ok(page) => {
                artist.albums =
                    page.present().into_iter().filter_map(AlbumDto::into_ref).collect()
            }
            Err(e) => tracing::debug!(artist = id, error = %e, "sem discografia"),
        }

        Ok(artist)
    }

    /// Caminha a paginacao por cursor de `/v1/me/following`.
    async fn followed(&self, offset: u32, limit: u32) -> CoreResult<Page<Artist>> {
        let wanted = clamp(limit, MAX_PAGE) as usize;
        let mut after: Option<String> = None;
        let mut skipped = 0u32;
        let mut items = Vec::new();
        let mut total = None;

        for _ in 0..MAX_CURSOR_PAGES {
            let mut params =
                vec![("type", "artist".to_string()), ("limit", MAX_PAGE.to_string())];
            if let Some(cursor) = &after {
                params.push(("after", escape(cursor)));
            }

            let dto: FollowedArtistsDto = self.api.get(&path("/v1/me/following", &params)).await?;
            let Some(page) = dto.artists else { break };
            total = total.or(page.total);

            let batch: Vec<Artist> =
                page.items.into_iter().flatten().filter_map(ArtistDto::into_artist).collect();
            let empty = batch.is_empty();

            for artist in batch {
                if skipped < offset {
                    skipped += 1;
                } else if items.len() < wanted {
                    items.push(artist);
                }
            }

            after = page.cursors.and_then(|c| c.after);
            if empty || after.is_none() || items.len() >= wanted {
                break;
            }
        }

        Ok(Page { items, offset, total })
    }

    async fn saved<T, U, F>(&self, endpoint: &str, offset: u32, limit: u32, map: F) -> CoreResult<Page<U>>
    where
        T: serde::de::DeserializeOwned,
        F: Fn(T) -> Option<U>,
    {
        let mut params =
            vec![("offset", offset.to_string()), ("limit", clamp(limit, MAX_PAGE).to_string())];
        params.extend(self.market());

        let page: Paged<T> = self.api.get(&path(endpoint, &params)).await?;
        let total = page.total;
        Ok(Page { items: page.present().into_iter().filter_map(map).collect(), offset, total })
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
            let mut params = vec![];
            params.extend(self.market());

            let dto: TrackDto = self.api.get(&path(&format!("/v1/tracks/{id}"), &params)).await?;
            dto.into_track().ok_or_else(|| CoreError::NotFound(format!("faixa {id}")))
        })
    }

    fn album<'a>(&'a self, id: &'a AlbumId) -> BoxFuture<'a, CoreResult<Album>> {
        Box::pin(async move {
            let id = spotify_id(id.provider, &id.id)?;
            let mut params = vec![];
            params.extend(self.market());

            let dto: AlbumDto = self.api.get(&path(&format!("/v1/albums/{id}"), &params)).await?;
            dto.into_album().ok_or_else(|| CoreError::NotFound(format!("album {id}")))
        })
    }

    fn artist<'a>(&'a self, id: &'a ArtistId) -> BoxFuture<'a, CoreResult<Artist>> {
        Box::pin(async move {
            let id = spotify_id(id.provider, &id.id)?;
            self.artist_detail(id).await
        })
    }

    fn playlist<'a>(&'a self, id: &'a PlaylistId) -> BoxFuture<'a, CoreResult<Playlist>> {
        Box::pin(async move {
            let id = spotify_id(id.provider, &id.id)?;
            let mut params = vec![];
            params.extend(self.market());

            let dto: PlaylistDto =
                self.api.get(&path(&format!("/v1/playlists/{id}"), &params)).await?;
            dto.into_playlist().ok_or_else(|| CoreError::NotFound(format!("playlist {id}")))
        })
    }

    fn playlist_tracks<'a>(
        &'a self,
        id: &'a PlaylistId,
        offset: u32,
        limit: u32,
    ) -> BoxFuture<'a, CoreResult<Page<Track>>> {
        Box::pin(async move {
            let id = spotify_id(id.provider, &id.id)?;
            let mut params = vec![
                ("offset", offset.to_string()),
                ("limit", clamp(limit, MAX_PLAYLIST_PAGE).to_string()),
            ];
            params.extend(self.market());

            let page: Paged<PlaylistItemDto> =
                self.api.get(&path(&format!("/v1/playlists/{id}/tracks"), &params)).await?;
            let total = page.total;
            let items = page
                .present()
                .into_iter()
                .filter_map(|i| i.track?.into_track())
                .collect();

            Ok(Page { items, offset, total })
        })
    }
}

impl Library for SpotifyCatalog {
    fn name(&self) -> &'static str {
        "spotify"
    }

    fn saved_playlists<'a>(
        &'a self,
        offset: u32,
        limit: u32,
    ) -> BoxFuture<'a, CoreResult<Page<Playlist>>> {
        Box::pin(self.saved("/v1/me/playlists", offset, limit, PlaylistDto::into_playlist))
    }

    fn saved_albums<'a>(&'a self, offset: u32, limit: u32) -> BoxFuture<'a, CoreResult<Page<Album>>> {
        Box::pin(self.saved("/v1/me/albums", offset, limit, |saved: SavedAlbumDto| {
            saved.album?.into_album()
        }))
    }

    fn saved_tracks<'a>(&'a self, offset: u32, limit: u32) -> BoxFuture<'a, CoreResult<Page<Track>>> {
        Box::pin(self.saved("/v1/me/tracks", offset, limit, |saved: SavedTrackDto| {
            saved.track?.into_track()
        }))
    }

    fn followed_artists<'a>(
        &'a self,
        offset: u32,
        limit: u32,
    ) -> BoxFuture<'a, CoreResult<Page<Artist>>> {
        Box::pin(self.followed(offset, limit))
    }
}

/// Lista de tipos aceita pelo parametro `type` da busca.
fn kinds(kind: SearchKind) -> &'static str {
    match kind {
        SearchKind::All => "track,album,artist,playlist",
        SearchKind::Tracks => "track",
        SearchKind::Albums => "album",
        SearchKind::Artists => "artist",
        SearchKind::Playlists => "playlist",
    }
}

/// Mantem o pedido dentro do que o endpoint aceita.
///
/// Pedir acima do teto nao devolve menos itens: devolve erro. Cortar aqui e a
/// diferenca entre uma lista curta e uma tela de falha.
fn clamp(limit: u32, max: u32) -> u32 {
    limit.clamp(1, max)
}

/// Monta caminho e query. Valores ja escapados; pares vazios sao omitidos.
fn path(endpoint: &str, params: &[(&str, String)]) -> String {
    let mut out = String::from(endpoint);
    for (i, (key, value)) in params.iter().filter(|(_, v)| !v.is_empty()).enumerate() {
        out.push(if i == 0 { '?' } else { '&' });
        out.push_str(key);
        out.push('=');
        out.push_str(value);
    }
    out
}

/// Id do Spotify pronto para entrar num caminho de URL.
fn spotify_id(provider: Provider, id: &str) -> CoreResult<&str> {
    if provider != Provider::Spotify {
        return Err(CoreError::NotFound(format!("{} nao e um recurso do Spotify", provider.as_str())));
    }
    checked_id(id)
}

fn map_page<T, U>(page: Option<Paged<T>>, map: impl Fn(T) -> Option<U>) -> Vec<U> {
    page.map(|p| p.present().into_iter().filter_map(map).collect()).unwrap_or_default()
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
    fn paths_are_built_with_the_separators_in_the_right_places() {
        assert_eq!(path("/v1/tracks/abc", &[]), "/v1/tracks/abc");
        assert_eq!(
            path("/v1/search", &[("q", "daft%20punk".into()), ("limit", "20".into())]),
            "/v1/search?q=daft%20punk&limit=20"
        );
        // Sem mercado conhecido o par some, e a query nao fica com `&` solto.
        assert_eq!(
            path("/v1/tracks/abc", &[("market", String::new()), ("limit", "1".into())]),
            "/v1/tracks/abc?limit=1"
        );
    }

    #[test]
    fn every_search_kind_maps_to_a_type_the_api_knows() {
        assert_eq!(kinds(SearchKind::All), "track,album,artist,playlist");
        assert_eq!(kinds(SearchKind::Tracks), "track");
        assert_eq!(kinds(SearchKind::Playlists), "playlist");
    }

    #[test]
    fn ids_from_another_provider_are_refused_before_reaching_the_network() {
        // Um id local vindo de um `.musicpack` nao pode virar uma requisicao ao
        // Spotify -- responderia 404 depois de uma ida a rede.
        assert!(spotify_id(Provider::Local, "musica.flac").is_err());
        assert_eq!(spotify_id(Provider::Spotify, "4cOdK2wGLETK").unwrap(), "4cOdK2wGLETK");
    }
}
