//! Formato das respostas do Web API e a traducao para o modelo do core.
//!
//! Fica separado do [`crate::catalog`] por um motivo pratico: a traducao e a
//! parte que erra, e e a unica testavel sem rede. Aqui dentro nao ha requisicao
//! nenhuma -- so JSON entrando e [`morune_core::model`] saindo.
//!
//! Duas regras valem para todo campo:
//!
//! **Tudo que o Spotify pode omitir e `Option`.** Um campo faltando numa faixa
//! nao pode derrubar a busca inteira: a resposta e uma pagina com dezenas de
//! itens, e recusar a pagina toda por causa de um item incompleto e desperdicio.
//!
//! **O que nao da para tocar nao vira faixa.** Item nulo, id ausente e arquivo
//! local da conta viram `None` e somem da lista, em vez de virarem uma linha que
//! falha ao ser clicada.

use std::sync::Arc;
use std::time::Duration;

use morune_core::model::{
    Album, AlbumId, AlbumRef, Artist, ArtistId, ArtistRef, ImageRef, ImageSet, Playlist,
    PlaylistId, Track, TrackId,
};
use serde::Deserialize;

/// Pagina do Web API.
///
/// `items` aceita nulo em cada posicao porque o Spotify devolve isso de
/// verdade: playlists apagadas continuam ocupando lugar na lista.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct Paged<T> {
    #[serde(default = "Vec::new")]
    pub items: Vec<Option<T>>,
    /// Quantos itens a conta tem ao todo, quando o Spotify informa.
    ///
    /// O deslocamento tambem volta na resposta e nao e lido: quem pediu a
    /// pagina ja sabe de onde pediu, e confiar no eco significaria reportar
    /// zero se ele faltasse.
    pub total: Option<u32>,
}

impl<T> Paged<T> {
    /// Itens presentes, na ordem, sem os buracos.
    pub fn present(self) -> Vec<T> {
        self.items.into_iter().flatten().collect()
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ImageDto {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ArtistRefDto {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AlbumRefDto {
    pub id: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub images: Vec<ImageDto>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TrackDto {
    pub id: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub artists: Vec<ArtistRefDto>,
    pub album: Option<AlbumRefDto>,
    #[serde(default)]
    pub duration_ms: u64,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    #[serde(default)]
    pub explicit: bool,
    /// So vem quando a requisicao informa o mercado da conta.
    pub is_playable: Option<bool>,
    /// Arquivo do computador do usuario, sincronizado pelo cliente oficial.
    /// Nao tem id no servidor e nao ha como o Morune tocar.
    #[serde(default)]
    pub is_local: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AlbumDto {
    pub id: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub artists: Vec<ArtistRefDto>,
    #[serde(default)]
    pub images: Vec<ImageDto>,
    pub release_date: Option<String>,
    pub total_tracks: Option<u32>,
    pub tracks: Option<Paged<TrackDto>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ArtistDto {
    pub id: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub images: Vec<ImageDto>,
    #[serde(default)]
    pub genres: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PlaylistDto {
    pub id: Option<String>,
    pub name: Option<String>,
    pub owner: Option<OwnerDto>,
    pub description: Option<String>,
    #[serde(default)]
    pub images: Vec<ImageDto>,
    pub tracks: Option<PlaylistTracksDto>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OwnerDto {
    pub display_name: Option<String>,
    pub id: Option<String>,
}

/// Faixas de uma playlist. Vem resumida (so o total) na listagem e completa no
/// detalhe, e por isso `items` e opcional.
#[derive(Debug, Deserialize)]
pub(crate) struct PlaylistTracksDto {
    pub total: Option<u32>,
    pub items: Option<Vec<Option<PlaylistItemDto>>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PlaylistItemDto {
    pub track: Option<TrackDto>,
}

/// Resposta de `/v1/search`.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct SearchDto {
    pub tracks: Option<Paged<TrackDto>>,
    pub albums: Option<Paged<AlbumDto>>,
    pub artists: Option<Paged<ArtistDto>>,
    pub playlists: Option<Paged<PlaylistDto>>,
}

/// Resposta de `/v1/artists/{id}/top-tracks`.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct TopTracksDto {
    #[serde(default = "Vec::new")]
    pub tracks: Vec<Option<TrackDto>>,
}

/// Item de `/v1/me/player/recently-played`.
///
/// O historico e paginado por cursor e cada item carrega o instante em que a
/// faixa tocou. O instante nao entra no modelo: a ordem da resposta ja e a
/// ordem cronologica, e uma data na tela nao ajuda a decidir o que ouvir.
#[derive(Debug, Deserialize)]
pub(crate) struct PlayHistoryDto {
    pub track: Option<TrackDto>,
}

/// Item de `/v1/me/albums`.
#[derive(Debug, Deserialize)]
pub(crate) struct SavedAlbumDto {
    pub album: Option<AlbumDto>,
}

/// Item de `/v1/me/tracks`.
#[derive(Debug, Deserialize)]
pub(crate) struct SavedTrackDto {
    pub track: Option<TrackDto>,
}

/// Resposta de `/v1/me/following?type=artist`, que e paginada por cursor.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct FollowedArtistsDto {
    pub artists: Option<CursorPagedDto<ArtistDto>>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct CursorPagedDto<T> {
    #[serde(default = "Vec::new")]
    pub items: Vec<Option<T>>,
    pub total: Option<u32>,
    pub cursors: Option<CursorsDto>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct CursorsDto {
    pub after: Option<String>,
}

// ---- traducao para o modelo do core ----

/// Converte as imagens preservando o contrato do [`ImageSet`]: menor primeiro.
///
/// O Spotify entrega da maior para a menor, e `best_for_width` depende da ordem
/// crescente para escolher a menor imagem suficiente -- que e o que mantem o uso
/// de RAM baixo numa grade com dezenas de capas.
fn images(raw: Vec<ImageDto>) -> ImageSet {
    let mut refs: Vec<ImageRef> = raw
        .into_iter()
        .map(|i| ImageRef { url: Arc::from(i.url.as_str()), width: i.width, height: i.height })
        .collect();
    refs.sort_by_key(|i| i.width.unwrap_or(u32::MAX));
    ImageSet(refs)
}

fn artist_refs(raw: Vec<ArtistRefDto>) -> Vec<ArtistRef> {
    raw.into_iter()
        .filter_map(|a| {
            Some(ArtistRef {
                id: ArtistId::spotify(a.id?.as_str()),
                name: Arc::from(a.name.unwrap_or_default().as_str()),
            })
        })
        .collect()
}

impl AlbumRefDto {
    fn into_ref(self) -> Option<AlbumRef> {
        Some(AlbumRef {
            id: AlbumId::spotify(self.id?.as_str()),
            name: Arc::from(self.name.unwrap_or_default().as_str()),
            images: images(self.images),
        })
    }
}

impl TrackDto {
    /// Faixa do modelo, ou `None` quando nao ha o que tocar.
    pub fn into_track(self) -> Option<Track> {
        self.into_track_of(None)
    }

    /// Idem, herdando o album de quem a listou.
    ///
    /// As faixas dentro de um album nao repetem o album em cada item, e uma
    /// faixa sem album fica sem capa na tela da fila.
    pub fn into_track_of(self, album: Option<AlbumRef>) -> Option<Track> {
        if self.is_local {
            return None;
        }
        let id = self.id?;
        Some(Track {
            id: TrackId::spotify(id.as_str()),
            name: Arc::from(self.name.unwrap_or_default().as_str()),
            artists: artist_refs(self.artists),
            album: self.album.and_then(AlbumRefDto::into_ref).or(album),
            duration: Duration::from_millis(self.duration_ms),
            track_number: self.track_number,
            disc_number: self.disc_number,
            explicit: self.explicit,
            // Ausente significa "o servidor nao avaliou", nao "nao toca": so o
            // `false` explicito marca a faixa como indisponivel.
            playable: self.is_playable.unwrap_or(true),
        })
    }
}

impl AlbumDto {
    pub fn into_album(self) -> Option<Album> {
        let id = AlbumId::spotify(self.id?.as_str());
        let name: Arc<str> = Arc::from(self.name.unwrap_or_default().as_str());
        let art = images(self.images);

        let own_ref =
            AlbumRef { id: id.clone(), name: name.clone(), images: art.clone() };
        let tracks = self
            .tracks
            .map(|p| {
                p.present()
                    .into_iter()
                    .filter_map(|t| t.into_track_of(Some(own_ref.clone())))
                    .collect()
            })
            .unwrap_or_default();

        Some(Album {
            id,
            name,
            artists: artist_refs(self.artists),
            images: art,
            release_date: self.release_date.map(|d| Arc::from(d.as_str())),
            total_tracks: self.total_tracks,
            tracks,
        })
    }

    /// Referencia curta, para quando so o nome e a capa interessam.
    pub fn into_ref(self) -> Option<AlbumRef> {
        Some(AlbumRef {
            id: AlbumId::spotify(self.id?.as_str()),
            name: Arc::from(self.name.unwrap_or_default().as_str()),
            images: images(self.images),
        })
    }
}

impl ArtistDto {
    pub fn into_artist(self) -> Option<Artist> {
        Some(Artist {
            id: ArtistId::spotify(self.id?.as_str()),
            name: Arc::from(self.name.unwrap_or_default().as_str()),
            images: images(self.images),
            genres: self.genres.into_iter().map(|g| Arc::from(g.as_str())).collect(),
            top_tracks: Vec::new(),
            albums: Vec::new(),
        })
    }
}

impl PlaylistDto {
    pub fn into_playlist(self) -> Option<Playlist> {
        let tracks = self.tracks;
        Some(Playlist {
            id: PlaylistId::spotify(self.id?.as_str()),
            name: Arc::from(self.name.unwrap_or_default().as_str()),
            owner: self
                .owner
                .and_then(|o| o.display_name.or(o.id))
                .map(|o| Arc::from(o.as_str())),
            description: self
                .description
                .filter(|d| !d.is_empty())
                .map(|d| Arc::from(d.as_str())),
            images: images(self.images),
            total_tracks: tracks.as_ref().and_then(|t| t.total),
            tracks: tracks.map(PlaylistTracksDto::into_tracks).unwrap_or_default(),
        })
    }
}

impl PlaylistTracksDto {
    pub fn into_tracks(self) -> Vec<Track> {
        self.items
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .filter_map(|i| i.track?.into_track())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse<T: serde::de::DeserializeOwned>(json: &str) -> T {
        serde_json::from_str(json).expect("json de exemplo do Web API")
    }

    const TRACK: &str = r#"{
        "id": "4cOdK2wGLETKBW3PvgPWqT",
        "name": "Never Gonna Give You Up",
        "artists": [{"id": "0gxyHStUsqpMadRV0Di1Qt", "name": "Rick Astley"}],
        "album": {
            "id": "6eUW0wxWtzkFdaEFsTJto6",
            "name": "Whenever You Need Somebody",
            "images": [
                {"url": "grande", "width": 640, "height": 640},
                {"url": "media", "width": 300, "height": 300},
                {"url": "pequena", "width": 64, "height": 64}
            ]
        },
        "duration_ms": 213573,
        "track_number": 1,
        "disc_number": 1,
        "explicit": false,
        "is_playable": true,
        "is_local": false
    }"#;

    #[test]
    fn a_track_keeps_everything_the_player_needs() {
        let track = parse::<TrackDto>(TRACK).into_track().expect("faixa tocavel");

        assert_eq!(track.id, TrackId::spotify("4cOdK2wGLETKBW3PvgPWqT"));
        assert_eq!(track.name.as_ref(), "Never Gonna Give You Up");
        assert_eq!(track.artists_line(), "Rick Astley");
        assert_eq!(track.duration, Duration::from_millis(213573));
        assert!(track.playable);
        assert_eq!(track.album.unwrap().name.as_ref(), "Whenever You Need Somebody");
    }

    #[test]
    fn cover_sizes_come_out_from_smallest_to_largest() {
        // `ImageSet::best_for_width` depende desta ordem para escolher a menor
        // capa suficiente, que e o que segura a RAM numa grade cheia.
        let track = parse::<TrackDto>(TRACK).into_track().unwrap();
        let art = track.album.unwrap().images;

        let widths: Vec<_> = art.0.iter().map(|i| i.width.unwrap()).collect();
        assert_eq!(widths, vec![64, 300, 640]);
        assert_eq!(art.best_for_width(100).unwrap().url.as_ref(), "media");
    }

    #[test]
    fn a_local_file_never_becomes_a_playable_row() {
        let dto: TrackDto = parse(r#"{"id": null, "name": "musica.mp3", "is_local": true}"#);
        assert!(dto.into_track().is_none());
    }

    #[test]
    fn a_track_without_id_is_dropped_instead_of_breaking_the_page() {
        let dto: TrackDto = parse(r#"{"name": "faixa sem id"}"#);
        assert!(dto.into_track().is_none());
    }

    #[test]
    fn a_track_the_server_did_not_rate_is_assumed_playable() {
        // `is_playable` so vem quando a requisicao informa o mercado. Tratar a
        // ausencia como "nao toca" apagaria a biblioteca inteira.
        let dto: TrackDto = parse(r#"{"id": "abc", "name": "sem veredito"}"#);
        assert!(dto.into_track().unwrap().playable);

        let refused: TrackDto = parse(r#"{"id": "abc", "name": "x", "is_playable": false}"#);
        assert!(!refused.into_track().unwrap().playable);
    }

    #[test]
    fn null_items_in_a_page_are_skipped() {
        // O Spotify devolve nulo no lugar de playlists apagadas. Recusar a
        // pagina inteira por causa disso deixaria a biblioteca vazia.
        let page: Paged<TrackDto> = parse(
            r#"{"items": [null, {"id": "abc", "name": "existe"}, null],
                "offset": 0, "total": 3}"#,
        );
        let tracks: Vec<_> = page.present().into_iter().filter_map(TrackDto::into_track).collect();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].name.as_ref(), "existe");
    }

    #[test]
    fn album_tracks_inherit_the_album_they_came_from() {
        // A resposta do album nao repete o album em cada faixa. Sem herdar, a
        // fila mostraria as faixas sem album e sem capa.
        let album: AlbumDto = parse(
            r#"{
                "id": "6eUW0wxWtzkFdaEFsTJto6",
                "name": "Whenever You Need Somebody",
                "artists": [{"id": "0gx", "name": "Rick Astley"}],
                "images": [{"url": "capa", "width": 300, "height": 300}],
                "release_date": "1987-11-16",
                "total_tracks": 10,
                "tracks": {"items": [{"id": "t1", "name": "faixa 1", "duration_ms": 1000}],
                           "offset": 0, "total": 1}
            }"#,
        );
        let album = album.into_album().expect("album valido");

        assert_eq!(album.release_date.as_deref(), Some("1987-11-16"));
        assert_eq!(album.tracks.len(), 1);
        let inherited = album.tracks[0].album.as_ref().expect("album herdado");
        assert_eq!(inherited.id, album.id);
        assert_eq!(inherited.images.best_for_width(1).unwrap().url.as_ref(), "capa");
    }

    #[test]
    fn a_playlist_summary_reports_the_total_without_the_tracks() {
        // E assim que `/v1/me/playlists` responde: total sim, faixas nao. A tela
        // precisa do numero antes de baixar mil faixas.
        let dto: PlaylistDto = parse(
            r#"{"id": "37i9", "name": "Descobertas da Semana",
                "owner": {"display_name": "Spotify", "id": "spotify"},
                "description": "", "images": [], "tracks": {"total": 30}}"#,
        );
        let playlist = dto.into_playlist().unwrap();

        assert_eq!(playlist.owner.as_deref(), Some("Spotify"));
        assert_eq!(playlist.total_tracks, Some(30));
        assert!(playlist.tracks.is_empty());
        // Descricao vazia e ausencia de descricao sao a mesma coisa na tela.
        assert!(playlist.description.is_none());
    }

    #[test]
    fn playlist_items_without_a_track_are_skipped() {
        // Episodios de podcast e faixas removidas chegam assim.
        let dto: PlaylistDto = parse(
            r#"{"id": "37i9", "name": "mista", "images": [],
                "tracks": {"total": 3, "items": [
                    {"track": null},
                    null,
                    {"track": {"id": "t1", "name": "vale", "duration_ms": 1}}
                ]}}"#,
        );
        let playlist = dto.into_playlist().unwrap();
        assert_eq!(playlist.tracks.len(), 1);
        assert_eq!(playlist.tracks[0].name.as_ref(), "vale");
    }

    #[test]
    fn the_history_keeps_the_order_it_arrived_in() {
        // A resposta ja vem da mais recente para a mais antiga, e reordenar
        // seria inventar um criterio que o usuario nao pediu.
        let page: Paged<PlayHistoryDto> = parse(
            r#"{"items": [
                {"track": {"id": "t1", "name": "ultima", "duration_ms": 1}},
                {"track": null},
                {"track": {"id": "t2", "name": "anterior", "duration_ms": 1}}
            ]}"#,
        );
        let nomes: Vec<String> = page
            .present()
            .into_iter()
            .filter_map(|i| i.track?.into_track())
            .map(|t| t.name.to_string())
            .collect();
        assert_eq!(nomes, vec!["ultima", "anterior"]);
    }

    #[test]
    fn a_search_response_maps_every_category() {
        let dto: SearchDto = parse(
            r#"{
                "tracks": {"items": [{"id": "t1", "name": "faixa", "duration_ms": 1}], "total": 1},
                "albums": {"items": [{"id": "a1", "name": "album"}], "total": 1},
                "artists": {"items": [{"id": "r1", "name": "artista", "genres": ["rock"]}], "total": 1},
                "playlists": {"items": [null, {"id": "p1", "name": "playlist"}], "total": 2}
            }"#,
        );

        let tracks: Vec<_> = dto.tracks.unwrap().present();
        assert_eq!(tracks.len(), 1);
        let artist = dto.artists.unwrap().present().pop().unwrap().into_artist().unwrap();
        assert_eq!(artist.genres.len(), 1);
        let playlists = dto.playlists.unwrap().present();
        assert_eq!(playlists.len(), 1);
        assert_eq!(dto.albums.unwrap().present()[0].name.as_deref(), Some("album"));
    }

    #[test]
    fn an_empty_response_is_not_an_error() {
        let dto: SearchDto = parse("{}");
        assert!(dto.tracks.is_none());

        let page: Paged<TrackDto> = parse(r#"{"items": []}"#);
        assert!(page.total.is_none());
        assert!(page.present().is_empty());
    }
}
