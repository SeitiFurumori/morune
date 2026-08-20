//! Formato das respostas do `pathfinder` e a traducao para o modelo do core.
//!
//! Fica separado do [`crate::pathfinder`] porque a traducao e a parte que erra,
//! e e a unica testavel sem rede. Aqui dentro nao ha requisicao nenhuma.
//!
//! **Nada aqui foi adivinhado.** Os tipos foram escritos contra a resposta real
//! gravada pela sonda em `bench-out/sonda/busca.json` -- ver
//! [`docs/HANDOFF.md`](../../../docs/HANDOFF.md).
//!
//! Tres particularidades do GraphQL do Spotify moldam este modulo:
//!
//! **Cada item vem embrulhado.** `items[].item.data` para faixa,
//! `items[].data` para album, artista e playlist. O embrulho carrega um
//! `__typename` que nao interessa ao Morune.
//!
//! **So faixa tem `id`.** Album, artista e playlist trazem apenas `uri`, e o id
//! sai dela. Uma URI de formato inesperado descarta o item, e nao a pagina.
//!
//! **Imagem de playlist vem sem tamanho.** `width` e `height` sao nulos ali,
//! entao o `ImageSet` recebe `None` e o `best_for_width` cai para a unica
//! disponivel.

use std::sync::Arc;
use std::time::Duration;

use morune_core::catalog::{SearchKind, SearchResults};
use morune_core::model::{
    Album, AlbumId, AlbumRef, Artist, ArtistId, ArtistRef, ImageRef, ImageSet, Playlist,
    PlaylistId, Track, TrackId,
};
use serde::Deserialize;

/// Resposta de uma consulta de busca.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct SearchDataDto {
    pub data: Option<SearchRootDto>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct SearchRootDto {
    #[serde(rename = "searchV2")]
    pub search: Option<SearchV2Dto>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct SearchV2Dto {
    #[serde(rename = "tracksV2")]
    pub tracks: Option<ItemsDto<WrappedItemDto<TrackDto>>>,
    #[serde(rename = "albumsV2")]
    pub albums: Option<ItemsDto<WrapperDto<AlbumDto>>>,
    pub artists: Option<ItemsDto<WrapperDto<ArtistDto>>>,
    pub playlists: Option<ItemsDto<WrapperDto<PlaylistDto>>>,
}

/// Lista paginada do GraphQL.
///
/// `items` aceita nulo em cada posicao pelo mesmo motivo do Web API: o servidor
/// devolve buracos, e um buraco nao pode custar a pagina inteira.
#[derive(Debug, Deserialize)]
pub(crate) struct ItemsDto<T> {
    #[serde(default = "Vec::new")]
    pub items: Vec<Option<T>>,
}

// `derive(Default)` exigiria `T: Default`, e os itens do GraphQL nao tem padrao
// sensato -- um album sem URI nao e um album vazio, e sim um item que deve sumir
// da lista. A lista vazia, essa sim, tem padrao.
impl<T> Default for ItemsDto<T> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<T> ItemsDto<T> {
    fn present(self) -> Vec<T> {
        self.items.into_iter().flatten().collect()
    }
}

/// Embrulho de faixa: `{ "item": { "data": ... } }`.
#[derive(Debug, Deserialize)]
pub(crate) struct WrappedItemDto<T> {
    pub item: Option<WrapperDto<T>>,
}

/// Embrulho de album, artista e playlist: `{ "data": ... }`.
#[derive(Debug, Deserialize)]
pub(crate) struct WrapperDto<T> {
    pub data: Option<T>,
}

/// Conjunto de imagens do GraphQL.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct CoverDto {
    #[serde(default = "Vec::new")]
    pub sources: Vec<SourceDto>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SourceDto {
    pub url: String,
    /// Nulo nas imagens de playlist. Ver o cabecalho.
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TrackDto {
    pub id: Option<String>,
    pub name: Option<String>,
    pub uri: Option<String>,
    pub duration: Option<DurationDto>,
    #[serde(rename = "albumOfTrack")]
    pub album: Option<AlbumRefDto>,
    pub artists: Option<ItemsDto<ArtistRefDto>>,
    pub playability: Option<PlayabilityDto>,
    #[serde(rename = "contentRating")]
    pub content_rating: Option<ContentRatingDto>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DurationDto {
    #[serde(rename = "totalMilliseconds", default)]
    pub total_ms: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PlayabilityDto {
    #[serde(default = "crate::graphql::yes")]
    pub playable: bool,
}

/// `label` vale `EXPLICIT` quando a faixa e explicita, e `NONE` quando nao.
#[derive(Debug, Deserialize)]
pub(crate) struct ContentRatingDto {
    pub label: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AlbumRefDto {
    pub id: Option<String>,
    pub uri: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "coverArt")]
    pub cover: Option<CoverDto>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ArtistRefDto {
    pub uri: Option<String>,
    pub profile: Option<ProfileDto>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProfileDto {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AlbumDto {
    pub uri: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "coverArt")]
    pub cover: Option<CoverDto>,
    pub artists: Option<ItemsDto<ArtistRefDto>>,
    pub date: Option<DateDto>,
}

/// O GraphQL entrega so o ano, e nao a data completa do Web API.
#[derive(Debug, Deserialize)]
pub(crate) struct DateDto {
    pub year: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ArtistDto {
    pub uri: Option<String>,
    pub profile: Option<ProfileDto>,
    pub visuals: Option<VisualsDto>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct VisualsDto {
    #[serde(rename = "avatarImage")]
    pub avatar: Option<CoverDto>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PlaylistDto {
    pub uri: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub images: Option<ItemsDto<CoverDto>>,
    #[serde(rename = "ownerV2")]
    pub owner: Option<WrapperDto<OwnerDto>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OwnerDto {
    pub name: Option<String>,
}

fn yes() -> bool {
    true
}

// ---- traducao para o modelo do core ----

impl SearchDataDto {
    /// Resultados no formato que a tela consome.
    ///
    /// `kind` filtra aqui, e nao na requisicao: a consulta persistida devolve
    /// todos os tipos de uma vez, e descartar o que a tela nao mostra e mais
    /// barato do que uma segunda viagem a rede.
    pub(crate) fn into_results(self, kind: SearchKind) -> SearchResults {
        let Some(search) = self.data.and_then(|d| d.search) else {
            return SearchResults::default();
        };

        let todos = matches!(kind, SearchKind::All);
        let quer = |alvo: SearchKind| todos || kind == alvo;

        SearchResults {
            tracks: if quer(SearchKind::Tracks) {
                unwrap_tracks(search.tracks)
            } else {
                Vec::new()
            },
            albums: if quer(SearchKind::Albums) {
                unwrap(search.albums, AlbumDto::into_album)
            } else {
                Vec::new()
            },
            artists: if quer(SearchKind::Artists) {
                unwrap(search.artists, ArtistDto::into_artist)
            } else {
                Vec::new()
            },
            playlists: if quer(SearchKind::Playlists) {
                unwrap(search.playlists, PlaylistDto::into_playlist)
            } else {
                Vec::new()
            },
        }
    }
}

fn unwrap<T, U>(
    list: Option<ItemsDto<WrapperDto<T>>>,
    map: impl Fn(T) -> Option<U>,
) -> Vec<U> {
    list.unwrap_or_default()
        .present()
        .into_iter()
        .filter_map(|w| w.data)
        .filter_map(map)
        .collect()
}

fn unwrap_tracks(list: Option<ItemsDto<WrappedItemDto<TrackDto>>>) -> Vec<Track> {
    list.unwrap_or_default()
        .present()
        .into_iter()
        .filter_map(|w| w.item?.data)
        .filter_map(TrackDto::into_track)
        .collect()
}

/// Ultimo segmento de uma URI do Spotify.
///
/// `spotify:album:4aawy...` vira `4aawy...`. Recusa o que nao tiver essa forma:
/// um id errado viraria requisicao para outro endereco, e a lista prefere um
/// item a menos a uma linha que falha ao ser clicada.
fn id_from_uri(uri: Option<String>, tipo: &str) -> Option<String> {
    let uri = uri?;
    let esperado = format!("spotify:{tipo}:");
    let id = uri.strip_prefix(&esperado)?;
    (!id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric())).then(|| id.to_string())
}

fn image_set(cover: Option<CoverDto>) -> ImageSet {
    let mut refs: Vec<ImageRef> = cover
        .unwrap_or_default()
        .sources
        .into_iter()
        .map(|s| ImageRef { url: Arc::from(s.url.as_str()), width: s.width, height: s.height })
        .collect();
    // O contrato do `ImageSet` e menor primeiro, e o GraphQL entrega em ordem
    // arbitraria -- na amostra veio 300, 64, 640.
    refs.sort_by_key(|i| i.width.unwrap_or(u32::MAX));
    ImageSet(refs)
}

fn artist_refs(list: Option<ItemsDto<ArtistRefDto>>) -> Vec<ArtistRef> {
    list.unwrap_or_default()
        .present()
        .into_iter()
        .filter_map(|a| {
            Some(ArtistRef {
                id: ArtistId::spotify(id_from_uri(a.uri, "artist")?.as_str()),
                name: Arc::from(
                    a.profile.and_then(|p| p.name).unwrap_or_default().as_str(),
                ),
            })
        })
        .collect()
}

impl TrackDto {
    pub(crate) fn into_track(self) -> Option<Track> {
        // Faixa e o unico tipo que traz `id` proprio; a URI cobre o caso de ele
        // faltar.
        let id = self.id.filter(|i| !i.is_empty()).or_else(|| id_from_uri(self.uri, "track"))?;

        Some(Track {
            id: TrackId::spotify(id.as_str()),
            name: Arc::from(self.name.unwrap_or_default().as_str()),
            artists: artist_refs(self.artists),
            album: self.album.and_then(AlbumRefDto::into_ref),
            duration: Duration::from_millis(self.duration.map(|d| d.total_ms).unwrap_or(0)),
            // O GraphQL da busca nao entrega numero de faixa nem de disco. Sao
            // usados so na tela de album, que vem por outro caminho.
            track_number: None,
            disc_number: None,
            explicit: self
                .content_rating
                .and_then(|c| c.label)
                .is_some_and(|l| l.eq_ignore_ascii_case("EXPLICIT")),
            playable: self.playability.map(|p| p.playable).unwrap_or(true),
        })
    }
}

impl AlbumRefDto {
    fn into_ref(self) -> Option<AlbumRef> {
        let id = self.id.filter(|i| !i.is_empty()).or_else(|| id_from_uri(self.uri, "album"))?;
        Some(AlbumRef {
            id: AlbumId::spotify(id.as_str()),
            name: Arc::from(self.name.unwrap_or_default().as_str()),
            images: image_set(self.cover),
        })
    }
}

impl AlbumDto {
    pub(crate) fn into_album(self) -> Option<Album> {
        Some(Album {
            id: AlbumId::spotify(id_from_uri(self.uri, "album")?.as_str()),
            name: Arc::from(self.name.unwrap_or_default().as_str()),
            artists: artist_refs(self.artists),
            images: image_set(self.cover),
            release_date: self.date.and_then(|d| d.year).map(|y| Arc::from(y.to_string().as_str())),
            // A busca lista albuns; quantas faixas cada um tem e assunto da
            // tela de album, que pede o metadado pelo protocolo interno.
            total_tracks: None,
            tracks: Vec::new(),
        })
    }
}

impl ArtistDto {
    pub(crate) fn into_artist(self) -> Option<Artist> {
        Some(Artist {
            id: ArtistId::spotify(id_from_uri(self.uri, "artist")?.as_str()),
            name: Arc::from(
                self.profile.and_then(|p| p.name).unwrap_or_default().as_str(),
            ),
            images: image_set(self.visuals.and_then(|v| v.avatar)),
            genres: Vec::new(),
            top_tracks: Vec::new(),
            albums: Vec::new(),
        })
    }
}

impl PlaylistDto {
    pub(crate) fn into_playlist(self) -> Option<Playlist> {
        // Playlist tem varias imagens em `images.items`, cada uma com suas
        // fontes. A primeira e a capa.
        let cover = self.images.and_then(|i| i.present().into_iter().next());

        Some(Playlist {
            id: PlaylistId::spotify(id_from_uri(self.uri, "playlist")?.as_str()),
            // A busca traz o `format`, mas classificar resultado de busca nao
            // muda nada: eles nunca alimentam prateleira do Inicio nem a barra
            // lateral, que sao as duas telas que olham o tipo.
            kind: morune_core::model::PlaylistKind::default(),
            name: Arc::from(self.name.unwrap_or_default().as_str()),
            owner: self
                .owner
                .and_then(|o| o.data)
                .and_then(|o| o.name)
                .filter(|n| !n.is_empty())
                .map(|n| Arc::from(n.as_str())),
            description: self.description.filter(|d| !d.is_empty()).map(|d| Arc::from(d.as_str())),
            images: image_set(cover),
            total_tracks: None,
            tracks: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse<T: serde::de::DeserializeOwned>(json: &str) -> T {
        serde_json::from_str(json).expect("json de teste")
    }

    #[test]
    fn a_track_survives_the_double_wrapper() {
        // O formato veio de `bench-out/sonda/busca.json`: faixa mora em
        // `items[].item.data`, e nao em `items[].data` como os outros tipos.
        let dto: SearchDataDto = parse(
            r#"{"data":{"searchV2":{"tracksV2":{"items":[{"item":{"data":{
                "id":"2JiDi0qAXsPwhPqA2qaKGt",
                "name":"Bohemian Rhapsody",
                "uri":"spotify:track:2JiDi0qAXsPwhPqA2qaKGt",
                "duration":{"totalMilliseconds":355154},
                "playability":{"playable":true},
                "contentRating":{"label":"NONE"},
                "artists":{"items":[{"uri":"spotify:artist:1dfeR4HaWDbWqFHLkxsg1d","profile":{"name":"Queen"}}]},
                "albumOfTrack":{"id":"1TkbyIkf6GSrO5e7gWS4AM","name":"A Night At The Opera","uri":"spotify:album:1TkbyIkf6GSrO5e7gWS4AM"}
            }}}]}}}}"#,
        );

        let r = dto.into_results(SearchKind::All);
        assert_eq!(r.tracks.len(), 1);
        assert_eq!(r.tracks[0].name.as_ref(), "Bohemian Rhapsody");
        assert_eq!(r.tracks[0].duration, Duration::from_millis(355154));
        assert_eq!(r.tracks[0].artists[0].name.as_ref(), "Queen");
        assert_eq!(r.tracks[0].album.as_ref().unwrap().name.as_ref(), "A Night At The Opera");
        assert!(!r.tracks[0].explicit);
        assert!(r.tracks[0].playable);
    }

    #[test]
    fn an_item_without_a_usable_uri_is_dropped_not_the_page() {
        // Uma playlist apagada, ou uma URI de outro tipo no meio da lista, nao
        // pode custar o resto dos resultados.
        let dto: SearchDataDto = parse(
            r#"{"data":{"searchV2":{"albumsV2":{"items":[
                {"data":{"uri":"spotify:show:algo","name":"podcast"}},
                null,
                {"data":{"uri":"spotify:album:4aawyAB79vO75wGe2Vx","name":"vale"}}
            ]}}}}"#,
        );

        let r = dto.into_results(SearchKind::All);
        assert_eq!(r.albums.len(), 1);
        assert_eq!(r.albums[0].name.as_ref(), "vale");
    }

    #[test]
    fn covers_come_out_smallest_first() {
        // O `best_for_width` do core depende dessa ordem para nao baixar 640 px
        // onde a tela desenha 64, e o GraphQL entrega fora de ordem.
        let dto: SearchDataDto = parse(
            r#"{"data":{"searchV2":{"albumsV2":{"items":[{"data":{
                "uri":"spotify:album:4aawyAB79vO75wGe2Vx","name":"a",
                "coverArt":{"sources":[
                    {"url":"media","width":300,"height":300},
                    {"url":"pequena","width":64,"height":64},
                    {"url":"grande","width":640,"height":640}
                ]}
            }}]}}}}"#,
        );

        let capas = &dto.into_results(SearchKind::All).albums[0].images;
        assert_eq!(capas.best_for_width(64).unwrap().url.as_ref(), "pequena");
        assert_eq!(capas.best_for_width(320).unwrap().url.as_ref(), "grande");
    }

    #[test]
    fn a_playlist_image_without_dimensions_still_works() {
        // O GraphQL entrega `width` e `height` nulos em capa de playlist.
        let dto: SearchDataDto = parse(
            r#"{"data":{"searchV2":{"playlists":{"items":[{"data":{
                "uri":"spotify:playlist:37i9dQZF1DX","name":"minha",
                "ownerV2":{"data":{"name":"furumori"}},
                "images":{"items":[{"sources":[{"url":"capa","width":null,"height":null}]}]}
            }}]}}}}"#,
        );

        let p = &dto.into_results(SearchKind::All).playlists[0];
        assert_eq!(p.owner.as_deref(), Some("furumori"));
        assert_eq!(p.images.best_for_width(64).unwrap().url.as_ref(), "capa");
    }

    #[test]
    fn asking_for_one_kind_does_not_bring_the_others() {
        let dto: SearchDataDto = parse(
            r#"{"data":{"searchV2":{
                "tracksV2":{"items":[{"item":{"data":{"id":"abc","name":"f","uri":"spotify:track:abc"}}}]},
                "artists":{"items":[{"data":{"uri":"spotify:artist:1dfeR4","profile":{"name":"Queen"}}}]}
            }}}"#,
        );

        let r = dto.into_results(SearchKind::Tracks);
        assert_eq!(r.tracks.len(), 1);
        assert!(r.artists.is_empty());
    }

    #[test]
    fn an_empty_response_is_not_an_error() {
        // Hash vencido responde 200 com `errors` e sem `data`. A tela mostra
        // "nada encontrado", que e melhor que uma mensagem tecnica.
        let dto: SearchDataDto = parse(r#"{"errors":[{"message":"PersistedQueryNotFound"}]}"#);
        assert_eq!(dto.into_results(SearchKind::All), SearchResults::default());
    }

    #[test]
    fn an_explicit_track_is_marked() {
        let dto: SearchDataDto = parse(
            r#"{"data":{"searchV2":{"tracksV2":{"items":[{"item":{"data":{
                "id":"abc","name":"f","uri":"spotify:track:abc",
                "contentRating":{"label":"EXPLICIT"},
                "playability":{"playable":false}
            }}}]}}}}"#,
        );

        let t = &dto.into_results(SearchKind::All).tracks[0];
        assert!(t.explicit);
        assert!(!t.playable);
    }
}
