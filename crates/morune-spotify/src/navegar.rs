//! As categorias da busca como o Spotify as monta: `browseAll` e `browsePage`.
//!
//! **Por que existe.** A busca do Morune ficava vazia antes de digitar. O
//! player web preenche a dele com "Navegar por tudo": 65 categorias (Rock,
//! Foco, Anime, Para dormir...), cada uma com cor e capa, e cada categoria abre
//! prateleiras de playlists. Medido em 02/11/2026 com a conta real
//! (`examples/sonda-navegar.rs`).
//!
//! **O que fica de fora.** Os `BrowseXlinkResponseWrapper` (atalhos para fora
//! da pagina, sem titulo nem cor) e, dentro de uma categoria, item de tipo que
//! o Morune nao abre -- o mesmo filtro do Inicio.
//!
//! **A divida** e a de toda consulta do pathfinder: o hash acompanha o player
//! web. Hash vencido vira erro comum e a busca so perde a grade de categorias.

use std::sync::Arc;

use morune_core::model::{Categoria, FeedItem, FeedSection};
use serde_json::Value;

use crate::inicio::{imagens, texto};

/// Hashes de `browseAll`, o atual primeiro. Medido no
/// `xpui-routes-search.2aa27db9.js` em 02/11/2026.
pub(crate) const HASHES_CATEGORIAS: [&str; 1] =
    ["dbd8b55e09a58afc52eab438bc228ba28fd72ac2f2148c6c26354980e4579001"];

/// Hashes de `browsePage`. Medido no `browse-v2.e9ddc72e.js` em 02/11/2026.
pub(crate) const HASHES_CATEGORIA: [&str; 1] =
    ["f5c4e6d668f5716464a231c1cc8b22c1cbf6ad68b09929fd7de813a30581298b"];

/// Variaveis de `browseAll`, iguais as do player web.
pub(crate) fn variaveis_categorias() -> Value {
    serde_json::json!({
        "pagePagination": { "offset": 0, "limit": 10 },
        "sectionPagination": { "offset": 0, "limit": 99 },
        "browseEndUserIntegration": "INTEGRATION_WEB_PLAYER",
    })
}

/// Variaveis de `browsePage`. Dez prateleiras de dez: o mesmo que o player web
/// pede na primeira tela.
pub(crate) fn variaveis_categoria(uri: &str) -> Value {
    serde_json::json!({
        "pagePagination": { "offset": 0, "limit": 10 },
        "sectionPagination": { "offset": 0, "limit": 10 },
        "uri": uri,
        "browseEndUserIntegration": "INTEGRATION_WEB_PLAYER",
        "includeEpisodeContentRatingsV2": false,
    })
}

/// Le a resposta de `browseAll` como categorias.
///
/// Leitura tolerante: cartao sem titulo, cor ou uri some sozinho.
pub(crate) fn categorias(resposta: &Value) -> Vec<Categoria> {
    let Some(secoes) = resposta
        .pointer("/data/browseStart/sections/items")
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    secoes
        .iter()
        .filter_map(|s| s.pointer("/sectionItems/items").and_then(Value::as_array))
        .flatten()
        .filter(|i| texto(i, "/content/__typename") == Some("BrowseSectionContainerWrapper"))
        .filter_map(|i| {
            let cartao = i.pointer("/content/data/data/cardRepresentation")?;
            let uri = texto(i, "/uri")?;
            if !uri.starts_with("spotify:page:") {
                return None;
            }
            Some(Categoria {
                titulo: texto(cartao, "/title/transformedLabel")?.trim().into(),
                cor: texto(cartao, "/backgroundColor/hex").unwrap_or("#535353").into(),
                imagens: imagens(cartao.pointer("/artwork/sources")),
                uri: uri.into(),
            })
        })
        .filter(|c| !c.titulo.is_empty())
        .collect()
}

/// Le a resposta de `browsePage`: o titulo da categoria e as prateleiras.
pub(crate) fn categoria(resposta: &Value) -> (String, Vec<FeedSection>) {
    let pagina = resposta.pointer("/data/browse");
    let titulo = pagina
        .and_then(|p| texto(p, "/header/title/transformedLabel"))
        .unwrap_or_default()
        .to_string();
    let Some(lista) = pagina
        .and_then(|p| p.pointer("/sections/items"))
        .and_then(Value::as_array)
    else {
        return (titulo, Vec::new());
    };

    let secoes = lista
        .iter()
        .filter_map(|s| {
            let nome = texto(s, "/data/title/transformedLabel")?.trim();
            if nome.is_empty() {
                return None;
            }
            let itens: Vec<FeedItem> = s
                .pointer("/sectionItems/items")
                .and_then(Value::as_array)?
                .iter()
                .filter_map(|i| crate::inicio::item(i.get("content")?))
                .collect();
            (!itens.is_empty()).then(|| FeedSection {
                title: Arc::from(nome),
                items: itens,
            })
        })
        .collect();
    (titulo, secoes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_as_categorias_e_ignora_atalhos() {
        let resposta = serde_json::json!({ "data": { "browseStart": { "sections": { "items": [
            { "sectionItems": { "items": [
                { "content": { "__typename": "BrowseSectionContainerWrapper", "data": { "data": {
                    "cardRepresentation": {
                        "title": { "transformedLabel": "Rock" },
                        "backgroundColor": { "hex": "#e13300" },
                        "artwork": { "sources": [ { "url": "https://i.scdn.co/image/x", "width": 300, "height": 300 } ] }
                    } } } },
                  "uri": "spotify:page:0JQ5DAqbMKFDXXwE9BDJAr" },
                { "content": { "__typename": "BrowseXlinkResponseWrapper" }, "uri": "spotify:xlink:abc" }
            ] } }
        ] } } } });
        let lidas = categorias(&resposta);
        assert_eq!(lidas.len(), 1);
        assert_eq!(&*lidas[0].titulo, "Rock");
        assert_eq!(&*lidas[0].cor, "#e13300");
        assert_eq!(&*lidas[0].uri, "spotify:page:0JQ5DAqbMKFDXXwE9BDJAr");
    }

    #[test]
    fn le_a_pagina_da_categoria() {
        let resposta = serde_json::json!({ "data": { "browse": {
            "header": { "title": { "transformedLabel": "Rock" } },
            "sections": { "items": [ {
                "data": { "__typename": "BrowseGenericSectionData", "title": { "transformedLabel": "Clássicos do Rock" } },
                "sectionItems": { "items": [ { "content": { "__typename": "PlaylistResponseWrapper", "data": {
                    "uri": "spotify:playlist:37i9dQZF1DXcF6B6QPhFDv", "name": "MARROW",
                    "images": { "items": [ { "sources": [ { "url": "https://i.scdn.co/image/y" } ] } ] }
                } } } ] }
            } ] }
        } } });
        let (titulo, secoes) = categoria(&resposta);
        assert_eq!(titulo, "Rock");
        assert_eq!(secoes.len(), 1);
        assert_eq!(&*secoes[0].title, "Clássicos do Rock");
        assert_eq!(secoes[0].items.len(), 1);
    }

    #[test]
    fn resposta_estranha_vira_lista_vazia() {
        assert!(categorias(&serde_json::json!({})).is_empty());
        assert_eq!(categoria(&serde_json::json!({})).1.len(), 0);
    }
}
