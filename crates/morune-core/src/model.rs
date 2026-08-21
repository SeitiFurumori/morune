use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Provedor de origem de um recurso.
///
/// Existe para que o modelo do core nao seja "o modelo do Spotify": um mesmo
/// `TrackId` carrega de onde ele veio, e backends diferentes podem coexistir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Spotify,
    Local,
}

impl Provider {
    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Spotify => "spotify",
            Provider::Local => "local",
        }
    }
}

macro_rules! resource_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub struct $name {
            pub provider: Provider,
            /// Identificador opaco dentro do provedor. Nunca interpretado pelo core.
            pub id: Arc<str>,
        }

        impl $name {
            pub fn new(provider: Provider, id: impl Into<Arc<str>>) -> Self {
                Self {
                    provider,
                    id: id.into(),
                }
            }

            pub fn spotify(id: impl Into<Arc<str>>) -> Self {
                Self::new(Provider::Spotify, id)
            }

            pub fn local(id: impl Into<Arc<str>>) -> Self {
                Self::new(Provider::Local, id)
            }

            /// Forma canonica estavel usada em cache, banco e URIs internas.
            pub fn canonical(&self) -> String {
                format!("{}:{}", self.provider.as_str(), self.id)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}:{}", self.provider.as_str(), self.id)
            }
        }
    };
}

resource_id!(TrackId, "Identificador de faixa.");
resource_id!(AlbumId, "Identificador de album.");
resource_id!(ArtistId, "Identificador de artista.");
resource_id!(PlaylistId, "Identificador de playlist.");

/// Referencia a uma imagem remota ou local, com tamanho conhecido quando disponivel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageRef {
    pub url: Arc<str>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl ImageRef {
    pub fn new(url: impl Into<Arc<str>>) -> Self {
        Self {
            url: url.into(),
            width: None,
            height: None,
        }
    }

    pub fn sized(url: impl Into<Arc<str>>, width: u32, height: u32) -> Self {
        Self {
            url: url.into(),
            width: Some(width),
            height: Some(height),
        }
    }
}

/// Conjunto de imagens de um recurso, ordenado do menor para o maior.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageSet(pub Vec<ImageRef>);

impl ImageSet {
    /// Menor imagem com largura >= `min_width`, caindo para a maior disponivel.
    ///
    /// Escolher a menor imagem suficiente e o que mantem o uso de RAM baixo em
    /// grids com dezenas de capas visiveis.
    pub fn best_for_width(&self, min_width: u32) -> Option<&ImageRef> {
        self.0
            .iter()
            .filter(|i| i.width.is_none_or(|w| w >= min_width))
            .min_by_key(|i| i.width.unwrap_or(u32::MAX))
            .or_else(|| self.0.iter().max_by_key(|i| i.width.unwrap_or(0)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtistRef {
    pub id: ArtistId,
    pub name: Arc<str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlbumRef {
    pub id: AlbumId,
    pub name: Arc<str>,
    #[serde(default)]
    pub images: ImageSet,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Track {
    pub id: TrackId,
    pub name: Arc<str>,
    pub artists: Vec<ArtistRef>,
    pub album: Option<AlbumRef>,
    #[serde(with = "duration_ms")]
    pub duration: Duration,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    #[serde(default)]
    pub explicit: bool,
    /// `false` quando o provedor sinaliza indisponibilidade no mercado do usuario.
    #[serde(default = "default_true")]
    pub playable: bool,
}

impl Track {
    /// Nome dos artistas para exibicao, ja concatenado.
    pub fn artists_line(&self) -> String {
        self.artists
            .iter()
            .map(|a| a.name.as_ref())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Album {
    pub id: AlbumId,
    pub name: Arc<str>,
    pub artists: Vec<ArtistRef>,
    #[serde(default)]
    pub images: ImageSet,
    pub release_date: Option<Arc<str>>,
    pub total_tracks: Option<u32>,
    #[serde(default)]
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artist {
    pub id: ArtistId,
    pub name: Arc<str>,
    #[serde(default)]
    pub images: ImageSet,
    #[serde(default)]
    pub genres: Vec<Arc<str>>,
    #[serde(default)]
    pub top_tracks: Vec<Track>,
    #[serde(default)]
    pub albums: Vec<AlbumRef>,
}

/// Que tipo de playlist e esta.
///
/// Existe porque "playlist" cobre coisas que o usuario trata de forma
/// diferente: a que ele montou fica na navegacao, a que o provedor gerou para
/// ele fica na tela de descoberta, e a vitrine editorial nao e nem uma nem
/// outra.
///
/// Nao e detalhe do Spotify: qualquer provedor que gere lista para o usuario
/// cai nas mesmas categorias. Quem nao souber classificar responde
/// [`PlaylistKind::Personal`], que e o comportamento certo para uma pasta de
/// arquivos locais.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaylistKind {
    /// Criada ou seguida pelo usuario.
    #[default]
    Personal,
    /// Gerada para esta conta a partir do que ela ouve.
    MadeForYou,
    /// Fluxo continuo em torno de uma semente -- artista, faixa ou tema.
    Station,
    /// Retrospectiva: o que a conta mais ouviu num periodo.
    Retrospective,
    /// Vitrine do provedor, igual para todo mundo.
    Editorial,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Playlist {
    pub id: PlaylistId,
    #[serde(default)]
    pub kind: PlaylistKind,
    pub name: Arc<str>,
    pub owner: Option<Arc<str>>,
    pub description: Option<Arc<str>>,
    #[serde(default)]
    pub images: ImageSet,
    pub total_tracks: Option<u32>,
    #[serde(default)]
    pub tracks: Vec<Track>,
}

fn default_true() -> bool {
    true
}

/// Serializa `Duration` como milissegundos inteiros.
///
/// Provedores expoem duracao em ms; guardar em ms evita erro de arredondamento
/// no round-trip cache -> UI -> seek.
mod duration_ms {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(d.as_millis() as u64)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        Ok(Duration::from_millis(u64::deserialize(d)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_id_is_provider_prefixed() {
        assert_eq!(TrackId::spotify("abc").canonical(), "spotify:abc");
        assert_eq!(TrackId::local("x/y.flac").canonical(), "local:x/y.flac");
    }

    #[test]
    fn ids_of_different_providers_never_collide() {
        assert_ne!(TrackId::spotify("abc"), TrackId::local("abc"));
    }

    #[test]
    fn best_for_width_picks_smallest_sufficient() {
        let set = ImageSet(vec![
            ImageRef::sized("a", 64, 64),
            ImageRef::sized("b", 300, 300),
            ImageRef::sized("c", 640, 640),
        ]);
        assert_eq!(set.best_for_width(100).unwrap().url.as_ref(), "b");
        assert_eq!(set.best_for_width(64).unwrap().url.as_ref(), "a");
    }

    #[test]
    fn best_for_width_falls_back_to_largest() {
        let set = ImageSet(vec![ImageRef::sized("a", 64, 64)]);
        assert_eq!(set.best_for_width(4000).unwrap().url.as_ref(), "a");
        assert!(ImageSet::default().best_for_width(10).is_none());
    }
}
