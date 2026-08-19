use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};

use crate::error::CoreResult;
use crate::model::{Album, AlbumId, Artist, ArtistId, Playlist, PlaylistId, Track, TrackId};

/// Future retornada pelos metodos de catalogo.
///
/// Escrita a mao em vez de `async fn` no trait porque o trait precisa ser
/// dyn-compativel: a aplicacao guarda `Arc<dyn Catalog>` e troca de provedor em
/// tempo de execucao. `async fn` em trait ainda nao permite isso.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Pagina de resultados. `total` e `None` quando o provedor nao informa.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub offset: u32,
    pub total: Option<u32>,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>) -> Self {
        let total = Some(items.len() as u32);
        Self { items, offset: 0, total }
    }

    pub fn empty() -> Self {
        Self { items: Vec::new(), offset: 0, total: Some(0) }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// `true` quando existe pelo menos mais uma pagina depois desta.
    pub fn has_more(&self) -> bool {
        match self.total {
            Some(total) => self.offset + self.items.len() as u32 > 0
                && self.offset + self.items.len() as u32 <= total
                && (self.offset + self.items.len() as u32) < total,
            None => !self.items.is_empty(),
        }
    }
}

/// Filtro de busca. `All` faz uma requisicao por categoria quando o provedor
/// nao suporta busca combinada.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchKind {
    #[default]
    All,
    Tracks,
    Albums,
    Artists,
    Playlists,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResults {
    #[serde(default)]
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub albums: Vec<Album>,
    #[serde(default)]
    pub artists: Vec<Artist>,
    #[serde(default)]
    pub playlists: Vec<Playlist>,
}

impl SearchResults {
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
            && self.albums.is_empty()
            && self.artists.is_empty()
            && self.playlists.is_empty()
    }

    pub fn total(&self) -> usize {
        self.tracks.len() + self.albums.len() + self.artists.len() + self.playlists.len()
    }
}

/// Acesso somente leitura ao catalogo de um provedor.
///
/// Implementacoes nao devem fazer cache proprio: cache e responsabilidade da
/// camada de storage, que envolve qualquer `Catalog`. Manter isto separado e o
/// que permite testar a UI com um catalogo em memoria.
pub trait Catalog: Send + Sync + 'static {
    fn name(&self) -> &'static str;

    fn search<'a>(
        &'a self,
        query: &'a str,
        kind: SearchKind,
        limit: u32,
    ) -> BoxFuture<'a, CoreResult<SearchResults>>;

    fn track<'a>(&'a self, id: &'a TrackId) -> BoxFuture<'a, CoreResult<Track>>;
    fn album<'a>(&'a self, id: &'a AlbumId) -> BoxFuture<'a, CoreResult<Album>>;
    fn artist<'a>(&'a self, id: &'a ArtistId) -> BoxFuture<'a, CoreResult<Artist>>;
    fn playlist<'a>(&'a self, id: &'a PlaylistId) -> BoxFuture<'a, CoreResult<Playlist>>;

    /// Faixas de uma playlist, paginadas. Playlists grandes nunca devem ser
    /// carregadas de uma vez: e o principal risco de RAM do aplicativo.
    fn playlist_tracks<'a>(
        &'a self,
        id: &'a PlaylistId,
        offset: u32,
        limit: u32,
    ) -> BoxFuture<'a, CoreResult<Page<Track>>>;
}

/// Biblioteca do usuario (o que ele salvou), separada do catalogo publico.
pub trait Library: Send + Sync + 'static {
    fn name(&self) -> &'static str;

    fn saved_playlists<'a>(
        &'a self,
        offset: u32,
        limit: u32,
    ) -> BoxFuture<'a, CoreResult<Page<Playlist>>>;

    fn saved_albums<'a>(
        &'a self,
        offset: u32,
        limit: u32,
    ) -> BoxFuture<'a, CoreResult<Page<Album>>>;

    fn saved_tracks<'a>(
        &'a self,
        offset: u32,
        limit: u32,
    ) -> BoxFuture<'a, CoreResult<Page<Track>>>;

    fn followed_artists<'a>(
        &'a self,
        offset: u32,
        limit: u32,
    ) -> BoxFuture<'a, CoreResult<Page<Artist>>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_reports_more_only_when_items_remain() {
        let full = Page { items: vec![1, 2, 3], offset: 0, total: Some(10) };
        assert!(full.has_more());

        let last = Page { items: vec![1, 2, 3], offset: 7, total: Some(10) };
        assert!(!last.has_more());

        let unknown_total = Page { items: vec![1], offset: 0, total: None };
        assert!(unknown_total.has_more());

        let exhausted: Page<i32> = Page { items: vec![], offset: 10, total: None };
        assert!(!exhausted.has_more());
    }

    #[test]
    fn empty_search_results_are_detected() {
        let r = SearchResults::default();
        assert!(r.is_empty());
        assert_eq!(r.total(), 0);
    }
}
