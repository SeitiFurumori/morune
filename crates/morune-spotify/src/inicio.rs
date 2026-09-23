//! O Inicio como o Spotify o monta: a consulta persistida `home`.
//!
//! **Por que existe.** O Inicio do Morune separava por tipo as playlists que a
//! conta segue no `rootlist` -- e "Feito para voce" parava nas cinco que a
//! pessoa tinha seguido. O cliente oficial nao funciona assim: ele pede ao
//! servidor as secoes prontas ("Feito para voce", "Tocados recentemente",
//! "Para fas de ...", estacoes, mixes), ja ordenadas e traduzidas. Medido em
//! 23/09/2026 com a conta real: 20 secoes, contra 4 prateleiras do Morune.
//!
//! **O que fica de fora.** Podcasts e episodios (fora do produto), a grade de
//! atalhos do topo (`HomeShortsSectionData`, que repete "Tocados
//! recentemente") e item de tipo que o Morune nao abre -- a colecao de
//! curtidas aparece como `UnknownType`, e ja tem prateleira propria.
//!
//! **A divida** e a mesma de toda consulta do pathfinder: o hash acompanha o
//! player web. Hash vencido vira erro comum, e o Inicio cai de volta nas
//! prateleiras do `rootlist`.

use std::sync::Arc;

use morune_core::model::{
    Album, AlbumId, Artist, ArtistId, ArtistRef, FeedItem, FeedSection, ImageRef, ImageSet,
    Playlist, PlaylistId, PlaylistKind, Provider,
};
use serde_json::Value;

/// Hash de `home` no player web (`web-player.86442c64.js`, 23/09/2026).
pub(crate) const HASH_HOME: &str =
    "76243c78b0e20ecdbe41b794dec8cbe73f75e585b0a7201b8d2e84578412847a";

/// Itens por secao. E o padrao do player web; cada cartao e uma capa
/// decodificada na memoria, e a prateleira rola para o lado de qualquer jeito.
const ITENS_POR_SECAO: u32 = 10;

/// Variaveis da consulta, iguais as do player web.
pub(crate) fn variaveis() -> Value {
    serde_json::json!({
        "homeEndUserIntegration": "INTEGRATION_WEB_PLAYER",
        "timeZone": fuso(),
        "sp_t": "",
        "facet": "",
        "sectionItemsLimit": ITENS_POR_SECAO,
        "includeEpisodeContentRatingsV2": false,
    })
}

/// O fuso decide a secao do momento ("Trilha sonora para o fim de tarde").
/// O Windows nao entrega nome IANA sem ICU, e o publico do Morune e o Brasil;
/// errar o fuso so troca o nome dessa secao.
fn fuso() -> &'static str {
    "America/Sao_Paulo"
}

/// Le a resposta de `home` como secoes do Inicio.
///
/// Leitura tolerante de proposito: um campo que faltar descarta o item, nao a
/// tela inteira. O servidor acrescenta tipos novos sem aviso.
pub(crate) fn secoes(resposta: &Value) -> Vec<FeedSection> {
    let Some(lista) = resposta
        .pointer("/data/home/sectionContainer/sections/items")
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    lista
        .iter()
        .filter(|s| texto(s, "/data/__typename") == Some("HomeGenericSectionData"))
        .filter_map(|s| {
            let titulo = texto(s, "/data/title/transformedLabel")?.trim();
            if titulo.is_empty() {
                return None;
            }
            let itens: Vec<FeedItem> = s
                .pointer("/sectionItems/items")
                .and_then(Value::as_array)?
                .iter()
                .filter_map(|i| item(i.get("content")?))
                .collect();
            (!itens.is_empty()).then(|| FeedSection {
                title: titulo.into(),
                items: itens,
            })
        })
        .collect()
}

fn item(conteudo: &Value) -> Option<FeedItem> {
    let dados = conteudo.get("data")?;
    match texto(conteudo, "/__typename")? {
        "PlaylistResponseWrapper" => {
            let id = id_de(texto(dados, "/uri")?, "playlist")?;
            Some(FeedItem::Playlist(Playlist {
                id: PlaylistId::new(Provider::Spotify, id),
                kind: PlaylistKind::default(),
                name: texto(dados, "/name")?.into(),
                owner: texto(dados, "/ownerV2/data/name").map(Arc::from),
                description: None,
                images: imagens(dados.pointer("/images/items/0/sources")),
                total_tracks: None,
                tracks: Vec::new(),
            }))
        }
        "AlbumResponseWrapper" => {
            let id = id_de(texto(dados, "/uri")?, "album")?;
            let artistas = dados
                .pointer("/artists/items")
                .and_then(Value::as_array)
                .map(|lista| {
                    lista
                        .iter()
                        .filter_map(|a| {
                            Some(ArtistRef {
                                id: ArtistId::new(
                                    Provider::Spotify,
                                    id_de(texto(a, "/uri")?, "artist")?,
                                ),
                                name: texto(a, "/profile/name")?.into(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(FeedItem::Album(Album {
                id: AlbumId::new(Provider::Spotify, id),
                name: texto(dados, "/name")?.into(),
                artists: artistas,
                images: imagens(dados.pointer("/coverArt/sources")),
                release_date: None,
                total_tracks: None,
                tracks: Vec::new(),
            }))
        }
        "ArtistResponseWrapper" => {
            let id = id_de(texto(dados, "/uri")?, "artist")?;
            Some(FeedItem::Artist(Artist {
                id: ArtistId::new(Provider::Spotify, id),
                name: texto(dados, "/profile/name")?.into(),
                images: imagens(dados.pointer("/visuals/avatarImage/sources")),
                genres: Vec::new(),
                top_tracks: Vec::new(),
                albums: Vec::new(),
            }))
        }
        // Episodio, colecao de curtidas e o que vier de novo.
        _ => None,
    }
}

/// `spotify:<tipo>:<id>` -> `<id>`, recusando id fora do base62 (ele vira
/// caminho de URL depois).
fn id_de<'a>(uri: &'a str, tipo: &str) -> Option<&'a str> {
    let resto = uri.strip_prefix("spotify:")?.strip_prefix(tipo)?;
    let id = resto.strip_prefix(':')?;
    (!id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric())).then_some(id)
}

fn imagens(fontes: Option<&Value>) -> ImageSet {
    let Some(fontes) = fontes.and_then(Value::as_array) else {
        return ImageSet::default();
    };
    let mut lista: Vec<ImageRef> = fontes
        .iter()
        .filter_map(|f| {
            let url = f.get("url")?.as_str()?;
            let largura = f.get("width").and_then(Value::as_u64);
            let altura = f.get("height").and_then(Value::as_u64);
            Some(match (largura, altura) {
                (Some(l), Some(a)) => ImageRef::sized(url, l as u32, a as u32),
                _ => ImageRef::new(url),
            })
        })
        .collect();
    lista.sort_by_key(|i| i.width.unwrap_or(u32::MAX));
    ImageSet(lista)
}

fn texto<'a>(valor: &'a Value, caminho: &str) -> Option<&'a str> {
    valor.pointer(caminho)?.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resposta() -> Value {
        serde_json::json!({ "data": { "home": { "sectionContainer": { "sections": { "items": [
            { "data": { "__typename": "HomeShortsSectionData" },
              "sectionItems": { "items": [ { "content": { "__typename": "PlaylistResponseWrapper",
                  "data": { "uri": "spotify:playlist:abc", "name": "Atalho" } } } ] } },
            { "data": { "__typename": "HomeGenericSectionData",
                        "title": { "transformedLabel": "Feito para furumori" } },
              "sectionItems": { "items": [
                { "content": { "__typename": "PlaylistResponseWrapper", "data": {
                    "uri": "spotify:playlist:37i9dQZEVXcGgR69b0yNZS", "name": "Discover Weekly",
                    "ownerV2": { "data": { "name": "Spotify" } },
                    "images": { "items": [ { "sources": [ { "url": "https://x/cover", "width": null, "height": null } ] } ] } } } },
                { "content": { "__typename": "UnknownType", "uri": "spotify:user:x:collection" } },
                { "content": { "__typename": "AlbumResponseWrapper", "data": {
                    "uri": "spotify:album:5l7lj9", "name": "Prince Of Egypt",
                    "artists": { "items": [ { "uri": "spotify:artist:0Lyf", "profile": { "name": "Vários" } } ] },
                    "coverArt": { "sources": [
                        { "url": "https://x/640", "width": 640, "height": 640 },
                        { "url": "https://x/300", "width": 300, "height": 300 } ] } } } },
                { "content": { "__typename": "ArtistResponseWrapper", "data": {
                    "uri": "spotify:artist:23Ht", "profile": { "name": "Jazz Fruits" },
                    "visuals": { "avatarImage": { "sources": [ { "url": "https://x/a", "width": 160, "height": 160 } ] } } } } }
              ] } },
            { "data": { "__typename": "HomeGenericSectionData",
                        "title": { "transformedLabel": "Episódios sugeridos" } },
              "sectionItems": { "items": [ { "content": { "__typename": "EpisodeOrChapterResponseWrapper", "data": {} } } ] } }
        ] } } } } })
    }

    #[test]
    fn keeps_music_sections_in_server_order_and_drops_shorts_and_podcasts() {
        let secoes = secoes(&resposta());
        assert_eq!(secoes.len(), 1);
        assert_eq!(&*secoes[0].title, "Feito para furumori");
        assert_eq!(secoes[0].items.len(), 3);
    }

    #[test]
    fn reads_each_item_kind() {
        let secoes = secoes(&resposta());
        let FeedItem::Playlist(p) = &secoes[0].items[0] else {
            panic!("playlist")
        };
        assert_eq!(p.id.id.as_ref(), "37i9dQZEVXcGgR69b0yNZS");
        assert_eq!(p.images.0.len(), 1);
        let FeedItem::Album(a) = &secoes[0].items[1] else {
            panic!("album")
        };
        assert_eq!(&*a.artists[0].name, "Vários");
        assert_eq!(a.images.best_for_width(300).unwrap().width, Some(300));
        assert!(matches!(&secoes[0].items[2], FeedItem::Artist(x) if &*x.name == "Jazz Fruits"));
    }

    #[test]
    fn an_unexpected_shape_is_an_empty_home_not_a_crash() {
        assert!(secoes(&serde_json::json!({ "data": null })).is_empty());
        assert_eq!(id_de("spotify:playlist:a/b", "playlist"), None);
        assert_eq!(id_de("spotify:album:x", "playlist"), None);
    }
}
