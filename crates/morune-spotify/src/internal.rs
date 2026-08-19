//! O que so existe pelo protocolo interno do Spotify.
//!
//! Em novembro de 2024 o Spotify fechou boa parte do Web API para aplicativos
//! novos. Entre o que caiu esta exatamente o que um player precisa para nao ser
//! so uma caixa de busca: recomendacao por semente, artistas parecidos,
//! playlists editoriais -- e o acesso as playlists que o proprio Spotify monta
//! para a conta, como Descobertas da Semana e Radar de Novidades. Pedi-las por
//! `/v1/playlists/{id}` responde 404.
//!
//! O cliente oficial nao usa o Web API para isso. Ele usa o mesmo protocolo que
//! a librespot ja fala para tocar audio, e ali essas coisas continuam de pe.
//! Este modulo e a parte do backend que fala por esse caminho.
//!
//! **Por que isto nao e giria de risco.** O que entra aqui e o que a librespot
//! ja expoe com protobuf tipado: `get_rootlist` e `get_playlist` devolvem
//! `SelectedListContent`, com nome, dono e tamanho decorados na mesma resposta.
//! Nao ha JSON adivinhado nem endpoint inventado.
//!
//! **O que este modulo nao faz.** Metadado de faixa. O protocolo interno
//! entrega URIs; nome, artista, album e duracao vem do Web API em lote, que
//! nao foi afetado pela mudanca. Um pedido por faixa custaria cem requisicoes
//! numa playlist de cem.

use std::sync::Arc;

use librespot_core::{SpotifyId, SpotifyUri};
use librespot_metadata::Metadata;
use librespot_protocol::extended_metadata::{
    BatchedEntityRequest, EntityRequest, ExtensionQuery,
};
use librespot_protocol::extension_kind::ExtensionKind;
use librespot_protocol::metadata::Album as AlbumMessage;
use librespot_protocol::metadata::Artist as ArtistMessage;
use librespot_protocol::metadata::ImageGroup as ImageGroupMessage;
use librespot_protocol::metadata::Track as TrackMessage;
use librespot_protocol::metadata::image::Size as ImageSize;
use librespot_protocol::playlist4_external::SelectedListContent as SelectedListContentMessage;
use protobuf::EnumOrUnknown;
use morune_core::model::{ImageRef, ImageSet, PlaylistId};
use morune_core::{CoreError, CoreResult};
use protobuf::Message;

use crate::auth::SharedSession;
use crate::error::from_librespot;

/// Quantas playlists o rootlist traz de uma vez.
///
/// E o tamanho que o cliente oficial pede. Uma conta com mais que isso perde a
/// cauda da lista, e isso e melhor que uma tela que demora para abrir.
const ROOTLIST_LENGTH: usize = 200;

/// Quantas faixas cabem num pedido de metadado.
///
/// E o mesmo teto que o Web API aceitava, e mantem o corpo da requisicao curto
/// o bastante para nao atrasar a primeira tela visivel de uma playlist grande.
const METADATA_BATCH: usize = 50;

/// Largura da capa que o Spotify chama de `SMALL`.
///
/// O protobuf entrega tamanho por enum, e nao em pixels. Os valores abaixo sao
/// os que o cliente oficial usa, e existem porque o `ImageSet` do core escolhe
/// capa por largura -- sem numero nenhum ele nao teria como escolher.
const LARGURA_SMALL: u32 = 64;
const LARGURA_DEFAULT: u32 = 300;
const LARGURA_LARGE: u32 = 640;

/// De onde as capas do protocolo interno sao servidas.
///
/// O protobuf entrega o identificador do arquivo, e nao a URL. O endereco e
/// fixo e e o mesmo que o cliente oficial usa.
const CDN_IMAGEM: &str = "https://i.scdn.co/image/";

/// Playlist como o protocolo interno a descreve.
///
/// Vem inteira numa requisicao so: o `decorate` do rootlist preenche nome, dono
/// e tamanho ao lado das URIs.
#[derive(Debug, Clone)]
pub(crate) struct PlaylistSummary {
    pub id: PlaylistId,
    pub name: String,
    pub owner: String,
    pub length: u32,
    /// Vazio nas playlists que o usuario criou ou seguiu; preenchido nas que o
    /// Spotify monta para ele. E o unico jeito honesto de separar as duas.
    pub format: String,
}

impl PlaylistSummary {
    /// `true` quando a playlist foi montada pelo Spotify para esta conta.
    ///
    /// Duas evidencias, porque nenhuma sozinha cobre tudo: o `format`, que as
    /// playlists algoritmicas carregam, e o dono `spotify`, que as editoriais
    /// carregam.
    pub fn made_by_spotify(&self) -> bool {
        !self.format.is_empty() || self.owner.eq_ignore_ascii_case("spotify")
    }
}

/// Conteudo de uma playlist pelo caminho interno.
#[derive(Debug, Clone)]
pub(crate) struct PlaylistContents {
    pub name: String,
    /// Ids de faixa em base62, na ordem da playlist.
    pub track_ids: Vec<String>,
}

/// Acesso ao protocolo interno sobre a sessao ativa.
#[derive(Debug)]
pub(crate) struct Internal {
    session: SharedSession,
}

impl Internal {
    pub(crate) fn new(session: SharedSession) -> Self {
        Self { session }
    }

    /// Todas as playlists da conta, como o cliente oficial as ve.
    ///
    /// # Por que o protobuf e lido campo a campo
    ///
    /// A librespot expoe `SelectedListContent::try_from`, que seria o caminho
    /// obvio -- mas ela converte a lista inteira de uma vez e **falha inteira**
    /// no primeiro item que nao entender. Na conta de teste isso acontecia de
    /// verdade: a tela mostrava
    /// `estado invalido: ID cannot be parsed` e nenhuma playlist, por causa de
    /// itens que o rootlist mistura na mesma lista.
    ///
    /// O rootlist nao traz so playlist. Traz marcador de pasta
    /// (`spotify:start-group:` e `spotify:end-group:`), e traz o que a conta
    /// tiver de fora do padrao. Nada disso e defeito -- e o formato.
    ///
    /// Ler o protobuf direto custa algumas linhas e devolve a regra que vale no
    /// resto do projeto: **um item que nao da para usar some da lista, e nao
    /// leva a lista junto.**
    pub(crate) async fn rootlist(&self) -> CoreResult<Vec<PlaylistSummary>> {
        let session = self.session.get().ok_or(CoreError::NotAuthenticated)?;
        let bytes = session
            .spclient()
            .get_rootlist(0, Some(ROOTLIST_LENGTH))
            .await
            .map_err(from_librespot)?;

        let message = SelectedListContentMessage::parse_from_bytes(&bytes)
            .map_err(|e| CoreError::Decode(format!("rootlist ilegivel: {e}")))?;

        Ok(summaries_from(&message))
    }

    /// Nome e faixas de uma playlist pelo caminho interno.
    ///
    /// E o que faz Descobertas da Semana tocar: pelo Web API ela responde 404.
    pub(crate) async fn playlist(&self, id: &str) -> CoreResult<PlaylistContents> {
        let session = self.session.get().ok_or(CoreError::NotAuthenticated)?;
        let uri = SpotifyUri::from_uri(&format!("spotify:playlist:{id}"))
            .map_err(|e| CoreError::NotFound(format!("playlist {id}: {e}")))?;

        let playlist = librespot_metadata::Playlist::get(&session, &uri)
            .await
            .map_err(from_librespot)?;

        let track_ids = playlist
            .tracks()
            .filter_map(|uri| match uri {
                SpotifyUri::Track { id } => id.to_base62().ok(),
                // Episodio de podcast e arquivo local entram na mesma lista e
                // nao sao faixa: somem aqui em vez de virar linha que falha.
                _ => None,
            })
            .collect();

        Ok(PlaylistContents { name: playlist.name().to_string(), track_ids })
    }
    /// Ids da colecao da conta: curtidas, ou artistas seguidos.
    ///
    /// # Por que mercury, e nao `collection/v2/paging`
    ///
    /// O `collection2v2.proto` da librespot descreve um `PageRequest` limpo,
    /// mas o endpoint correspondente respondeu 404 na sonda. O que responde e
    /// o `hm://collection/{conjunto}/{usuario}`, medido em 19/08/2026: 723
    /// itens nas curtidas desta conta.
    ///
    /// # O formato
    ///
    /// Protobuf simples, sem esquema publicado, mas sem ambiguidade: campo 1
    /// repetido, e dentro de cada item o campo 2 traz o `gid` de 16 bytes. Foi
    /// lido byte a byte de `bench-out/sonda/colecao-faixas.bin`.
    ///
    /// Arquivo local da conta aparece na mesma lista com um id de texto no
    /// lugar do `gid`. Nao ha como o Morune tocar, entao some aqui em vez de
    /// virar linha que falha ao ser clicada.
    pub(crate) async fn collection(&self, conjunto: Conjunto) -> CoreResult<Vec<String>> {
        let session = self.session.get().ok_or(CoreError::NotAuthenticated)?;
        let usuario = session.username();

        let uri = format!("hm://collection/{}/{usuario}/?allowonlytracks=false", conjunto.caminho());
        let resposta = session
            .mercury()
            .get(uri)
            .map_err(from_librespot)?
            .await
            .map_err(from_librespot)?;

        let mut out = Vec::new();
        for parte in &resposta.payload {
            out.extend(collection_ids(parte));
        }
        Ok(out)
    }
    /// Album completo pelo protocolo interno: capa, artistas, data e faixas.
    ///
    /// Uma requisicao so. O protobuf do album ja traz as faixas de cada disco
    /// com nome e duracao, entao a tela de album nao precisa de um segundo
    /// pedido de metadado.
    pub(crate) async fn album(&self, id: &str) -> CoreResult<AlbumMeta> {
        let session = self.session.get().ok_or(CoreError::NotAuthenticated)?;
        let uri = SpotifyUri::from_uri(&format!("spotify:album:{id}"))
            .map_err(|e| CoreError::NotFound(format!("album {id}: {e}")))?;

        let bytes = session
            .spclient()
            .get_album_metadata(&uri)
            .await
            .map_err(from_librespot)?;

        let message = AlbumMessage::parse_from_bytes(&bytes)
            .map_err(|e| CoreError::Decode(format!("album ilegivel: {e}")))?;

        AlbumMeta::from_message(&message)
            .ok_or_else(|| CoreError::NotFound(format!("album {id}")))
    }

    /// Artista pelo protocolo interno.
    pub(crate) async fn artist(&self, id: &str) -> CoreResult<ArtistMeta> {
        let session = self.session.get().ok_or(CoreError::NotAuthenticated)?;
        let uri = SpotifyUri::from_uri(&format!("spotify:artist:{id}"))
            .map_err(|e| CoreError::NotFound(format!("artista {id}: {e}")))?;

        let bytes = session
            .spclient()
            .get_artist_metadata(&uri)
            .await
            .map_err(from_librespot)?;

        let message = ArtistMessage::parse_from_bytes(&bytes)
            .map_err(|e| CoreError::Decode(format!("artista ilegivel: {e}")))?;

        ArtistMeta::from_message(&message)
            .ok_or_else(|| CoreError::NotFound(format!("artista {id}")))
    }
    /// Metadado de varios artistas numa requisicao so.
    ///
    /// Mesmo caminho e mesmo motivo do [`Internal::tracks`]: a colecao entrega
    /// ids, e um pedido por artista custaria um por linha da tela.
    pub(crate) async fn artists(&self, ids: &[String]) -> CoreResult<Vec<ArtistMeta>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let session = self.session.get().ok_or(CoreError::NotAuthenticated)?;
        let mut out = Vec::with_capacity(ids.len());

        for lote in ids.chunks(METADATA_BATCH) {
            let mut pedido = BatchedEntityRequest::new();
            for id in lote {
                let mut consulta = ExtensionQuery::new();
                consulta.extension_kind = EnumOrUnknown::new(ExtensionKind::ARTIST_V4);

                let mut entidade = EntityRequest::new();
                entidade.entity_uri = format!("spotify:artist:{id}");
                entidade.query.push(consulta);
                pedido.entity_request.push(entidade);
            }

            let resposta = session
                .spclient()
                .get_extended_metadata(pedido)
                .await
                .map_err(from_librespot)?;

            for entidade in &resposta.extended_metadata {
                for extensao in &entidade.extension_data {
                    let Some(dado) = extensao.extension_data.as_ref() else { continue };
                    let Ok(message) = ArtistMessage::parse_from_bytes(&dado.value) else {
                        continue;
                    };
                    if let Some(artista) = ArtistMeta::from_message(&message) {
                        out.push(artista);
                    }
                }
            }
        }

        Ok(out)
    }

    /// Metadado de varias faixas numa requisicao so.
    ///
    /// O `/v1/tracks?ids=...` do Web API morreu. O substituto e o
    /// `extended-metadata` do protocolo interno, que aceita lote pelo mesmo
    /// motivo: uma requisicao por faixa custaria cem numa playlist de cem.
    ///
    /// A resposta e protobuf tipado pela propria librespot -- nao ha JSON
    /// adivinhado. Faixa que o servidor nao devolver some da lista, e nao leva
    /// a playlist junto.
    pub(crate) async fn tracks(&self, ids: &[String]) -> CoreResult<Vec<TrackMeta>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let session = self.session.get().ok_or(CoreError::NotAuthenticated)?;
        let mut out = Vec::with_capacity(ids.len());

        for lote in ids.chunks(METADATA_BATCH) {
            let mut pedido = BatchedEntityRequest::new();
            for id in lote {
                let mut consulta = ExtensionQuery::new();
                consulta.extension_kind = EnumOrUnknown::new(ExtensionKind::TRACK_V4);

                let mut entidade = EntityRequest::new();
                entidade.entity_uri = format!("spotify:track:{id}");
                entidade.query.push(consulta);
                pedido.entity_request.push(entidade);
            }

            let resposta = session
                .spclient()
                .get_extended_metadata(pedido)
                .await
                .map_err(from_librespot)?;

            for entidade in &resposta.extended_metadata {
                for extensao in &entidade.extension_data {
                    let Some(dado) = extensao.extension_data.as_ref() else { continue };
                    let Ok(message) = TrackMessage::parse_from_bytes(&dado.value) else {
                        continue;
                    };
                    if let Some(faixa) = TrackMeta::from_message(&message) {
                        out.push(faixa);
                    }
                }
            }
        }

        Ok(out)
    }
}


/// Faixa como o protocolo interno a descreve.
///
/// Existe em vez de devolver `morune_core::model::Track` direto porque o
/// protobuf entrega `gid` de 16 bytes, e a conversao para base62 pode falhar --
/// e falha tem de virar item descartado, nao `Track` invalido.
#[derive(Debug, Clone)]
pub(crate) struct TrackMeta {
    pub id: String,
    pub name: String,
    pub artists: Vec<(String, String)>,
    pub album: Option<(String, String, ImageSet)>,
    pub duration_ms: u64,
    pub number: Option<u32>,
    pub disc: Option<u32>,
    pub explicit: bool,
}

impl TrackMeta {
    fn from_message(msg: &TrackMessage) -> Option<Self> {
        Some(Self {
            id: base62(msg.gid())?,
            name: msg.name().to_string(),
            artists: msg
                .artist
                .iter()
                .filter_map(|a| Some((base62(a.gid())?, a.name().to_string())))
                .collect(),
            album: msg.album.as_ref().and_then(|a| {
                Some((
                    base62(a.gid())?,
                    a.name().to_string(),
                    covers(a.cover_group.as_ref()),
                ))
            }),
            duration_ms: msg.duration().max(0) as u64,
            // O protobuf usa 0 para ausente, e faixa 0 nao existe.
            number: Some(msg.number()).filter(|n| *n > 0).map(|n| n as u32),
            disc: Some(msg.disc_number()).filter(|n| *n > 0).map(|n| n as u32),
            explicit: msg.explicit(),
        })
    }
}

/// Converte um `gid` de 16 bytes no id base62 que o resto do Morune usa.
fn base62(gid: &[u8]) -> Option<String> {
    SpotifyId::from_raw(gid).ok()?.to_base62().ok()
}

/// Capas de um album, no contrato do [`ImageSet`]: menor primeiro.
///
/// O protobuf da um `file_id` e um tamanho por enum; a URL e montada aqui. Uma
/// imagem sem largura conhecida entra com a largura do enum, que e o que
/// permite ao `best_for_width` nao baixar 640 px para desenhar 64.
fn covers(group: Option<&ImageGroupMessage>) -> ImageSet {
    let Some(group) = group else { return ImageSet::default() };

    let mut refs: Vec<ImageRef> = group
        .image
        .iter()
        .map(|img| {
            let largura = match img.width() {
                w if w > 0 => w as u32,
                _ => match img.size() {
                    ImageSize::SMALL => LARGURA_SMALL,
                    ImageSize::LARGE | ImageSize::XLARGE => LARGURA_LARGE,
                    ImageSize::DEFAULT => LARGURA_DEFAULT,
                },
            };
            ImageRef {
                url: Arc::from(format!("{CDN_IMAGEM}{}", hex(img.file_id())).as_str()),
                width: Some(largura),
                height: Some(largura),
            }
        })
        .collect();

    refs.sort_by_key(|i| i.width.unwrap_or(u32::MAX));
    ImageSet(refs)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut acc, b| {
        use std::fmt::Write;
        let _ = write!(acc, "{b:02x}");
        acc
    })
}




/// Album como o protocolo interno o descreve.
#[derive(Debug, Clone)]
pub(crate) struct AlbumMeta {
    pub id: String,
    pub name: String,
    pub artists: Vec<(String, String)>,
    pub images: ImageSet,
    pub year: Option<i32>,
    pub tracks: Vec<TrackMeta>,
}

impl AlbumMeta {
    fn from_message(msg: &AlbumMessage) -> Option<Self> {
        let capas = covers(msg.cover_group.as_ref());

        // As faixas vem agrupadas por disco e nao repetem o album em cada
        // item. Sem preencher o album aqui, a fila ficaria sem capa.
        let referencia = base62(msg.gid()).map(|id| (id, msg.name().to_string(), capas.clone()));

        let tracks = msg
            .disc
            .iter()
            .flat_map(|disco| disco.track.iter())
            .filter_map(|faixa| {
                let mut meta = TrackMeta::from_message(faixa)?;
                if meta.album.is_none() {
                    meta.album = referencia.clone();
                }
                Some(meta)
            })
            .collect();

        Some(Self {
            id: base62(msg.gid())?,
            name: msg.name().to_string(),
            artists: msg
                .artist
                .iter()
                .filter_map(|a| Some((base62(a.gid())?, a.name().to_string())))
                .collect(),
            images: capas,
            year: msg.date.as_ref().map(|d| d.year()).filter(|y| *y > 0),
            tracks,
        })
    }
}

/// Artista como o protocolo interno o descreve.
#[derive(Debug, Clone)]
pub(crate) struct ArtistMeta {
    pub id: String,
    pub name: String,
    pub images: ImageSet,
    pub genres: Vec<String>,
}

impl ArtistMeta {
    fn from_message(msg: &ArtistMessage) -> Option<Self> {
        Some(Self {
            id: base62(msg.gid())?,
            name: msg.name().to_string(),
            images: covers(msg.portrait_group.as_ref()),
            // O protobuf do artista nao traz genero neste caminho. A tela do
            // artista, que e onde genero aparece, vem por outra requisicao.
            genres: Vec::new(),
        })
    }
}

/// Conjuntos da colecao que o Morune le.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Conjunto {
    /// Musicas curtidas.
    Faixas,
    /// Artistas seguidos.
    Artistas,
}

impl Conjunto {
    fn caminho(self) -> &'static str {
        match self {
            // O conjunto das curtidas se chama `collection` mesmo, o que rende
            // o `hm://collection/collection/...` que parece erro de digitacao.
            Self::Faixas => "collection",
            Self::Artistas => "artist",
        }
    }
}

/// Ids base62 de um payload de colecao.
///
/// Le o protobuf a mao porque o esquema nao esta entre os compilados pela
/// librespot. So dois campos interessam, e o que nao encaixar e pulado -- ver
/// [`Internal::collection`].
fn collection_ids(bytes: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = 0usize;

    while let Some((campo, dado, proximo)) = campo_delimitado(bytes, cursor) {
        cursor = proximo;
        if campo != 1 {
            continue;
        }

        // Dentro do item, o campo 2 e o identificador.
        let mut interno = 0usize;
        while let Some((campo, id, proximo)) = campo_delimitado(dado, interno) {
            interno = proximo;
            if campo == 2 {
                if let Some(base62) = base62(id) {
                    out.push(base62);
                }
            }
        }
    }

    out
}

/// Proximo campo delimitado por tamanho a partir de `cursor`.
///
/// Devolve `(numero do campo, conteudo, proximo cursor)`. Campos de outro tipo
/// sao pulados; qualquer coisa malformada encerra a leitura em vez de entrar em
/// laco -- um payload truncado nao pode travar a interface.
fn campo_delimitado(bytes: &[u8], mut cursor: usize) -> Option<(u64, &[u8], usize)> {
    loop {
        let (chave, lido) = varint(bytes, cursor)?;
        cursor += lido;
        let campo = chave >> 3;

        match chave & 7 {
            // delimitado por tamanho
            2 => {
                let (tamanho, lido) = varint(bytes, cursor)?;
                cursor += lido;
                let fim = cursor.checked_add(tamanho as usize)?;
                let dado = bytes.get(cursor..fim)?;
                return Some((campo, dado, fim));
            }
            0 => {
                let (_, lido) = varint(bytes, cursor)?;
                cursor += lido;
            }
            5 => cursor = cursor.checked_add(4)?,
            1 => cursor = cursor.checked_add(8)?,
            _ => return None,
        }
    }
}

/// Varint do protobuf. Devolve o valor e quantos bytes ocupou.
fn varint(bytes: &[u8], cursor: usize) -> Option<(u64, usize)> {
    let mut valor = 0u64;
    let mut deslocamento = 0u32;

    for (lido, byte) in bytes.get(cursor..)?.iter().enumerate() {
        // Mais que dez bytes nao e varint de 64 bits: e lixo.
        if lido >= 10 {
            return None;
        }
        valor |= u64::from(byte & 0x7f) << deslocamento;
        if byte & 0x80 == 0 {
            return Some((valor, lido + 1));
        }
        deslocamento += 7;
    }

    None
}

/// Le o rootlist item a item, descartando o que nao for playlist utilizavel.
///
/// Separado do metodo assincrono para poder ser testado com um protobuf montado
/// a mao -- que e o unico jeito de cobrir os casos que quebravam a tela sem
/// precisar de uma conta real.
///
/// `items` e `meta_items` sao listas paralelas: a decoracao do item na posicao
/// N esta na posicao N da outra. Sem decoracao, ou fora de sincronia, a playlist
/// ainda entra -- so que sem nome, que e melhor que sumir.
fn summaries_from(message: &SelectedListContentMessage) -> Vec<PlaylistSummary> {
    let contents = message.contents.as_ref();
    let items = contents.map(|c| c.items.as_slice()).unwrap_or_default();
    let meta = contents.map(|c| c.meta_items.as_slice()).unwrap_or_default();

    let mut out = Vec::with_capacity(items.len());

    for (index, item) in items.iter().enumerate() {
        let Some(id) = playlist_id(item.uri()) else {
            // Marcador de pasta e URI fora do padrao caem aqui. Nao e erro: e o
            // formato do rootlist.
            tracing::trace!(uri = item.uri(), "item do rootlist que nao e playlist");
            continue;
        };

        let decorated = meta.get(index);
        let atributos = decorated.and_then(|m| m.attributes.as_ref());

        out.push(PlaylistSummary {
            id: PlaylistId::spotify(id.as_str()),
            name: atributos.map(|a| a.name().to_string()).unwrap_or_default(),
            owner: decorated
                .map(|m| m.owner_username().to_string())
                .filter(|o| !o.is_empty())
                .or_else(|| owner_from_uri(item.uri()))
                .unwrap_or_default(),
            length: decorated.map(|m| m.length().max(0) as u32).unwrap_or_default(),
            format: atributos.map(|a| a.format().to_string()).unwrap_or_default(),
        });
    }

    out
}

/// Id base62 de uma URI de playlist, ou `None` quando nao e uma.
///
/// Aceita as duas formas que o rootlist mistura: `spotify:playlist:{id}` e a
/// antiga `spotify:user:{dono}:playlist:{id}`.
fn playlist_id(uri: &str) -> Option<String> {
    let resto = uri.strip_prefix("spotify:")?;

    let id = match resto.strip_prefix("playlist:") {
        Some(id) => id,
        // `user:{dono}:playlist:{id}`
        None => resto.strip_prefix("user:")?.split_once(":playlist:")?.1,
    };

    // Base62 e o unico alfabeto valido. Recusar aqui e o que impede um id
    // estranho de virar requisicao para outro endereco.
    (id.len() == 22 && id.bytes().all(|b| b.is_ascii_alphanumeric())).then(|| id.to_string())
}

/// Dono embutido na forma antiga da URI, quando a decoracao nao trouxe um.
fn owner_from_uri(uri: &str) -> Option<String> {
    let dono = uri.strip_prefix("spotify:user:")?.split_once(':')?.0;
    (!dono.is_empty()).then(|| dono.to_string())
}


/// Ponte para o exemplo `rootlist`. Ver `crate::debug_rootlist`.
pub(crate) fn summaries_for_debug(
    bytes: &[u8],
) -> Result<Vec<(String, String, u32, String)>, String> {
    let message = SelectedListContentMessage::parse_from_bytes(bytes)
        .map_err(|e| format!("rootlist ilegivel: {e}"))?;

    Ok(summaries_from(&message)
        .into_iter()
        .map(|s| (s.name, s.owner, s.length, s.format))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use librespot_protocol::playlist4_external::{Item, ListItems};

    fn summary(owner: &str, format: &str) -> PlaylistSummary {
        PlaylistSummary {
            id: PlaylistId::spotify("37i9dQZEVXcJZyENOWUFo7"),
            name: "Descobertas da Semana".into(),
            owner: owner.into(),
            length: 30,
            format: format.into(),
        }
    }

    #[test]
    fn an_algorithmic_playlist_is_recognised_by_its_format() {
        assert!(summary("felipe", "discover-weekly").made_by_spotify());
    }

    #[test]
    fn an_editorial_playlist_is_recognised_by_its_owner() {
        // As editoriais nao trazem `format`, mas pertencem ao proprio Spotify.
        assert!(summary("spotify", "").made_by_spotify());
        assert!(summary("Spotify", "").made_by_spotify());
    }

    #[test]
    fn a_playlist_the_user_made_is_not_confused_with_one_the_spotify_made() {
        assert!(!summary("felipe", "").made_by_spotify());
    }
    #[test]
    fn a_folder_marker_no_longer_takes_the_whole_rootlist_down() {
        // Este e o defeito que a tela mostrava como "ID cannot be parsed":
        // um item que nao e playlist fazia a conversao da librespot falhar
        // inteira, e a conta ficava sem nenhuma playlist.
        let mut lista = ListItems::new();
        for uri in [
            "spotify:start-group:8e3f1a2b:Minhas",
            "spotify:playlist:37i9dQZEVXcJZyENOWUFo7",
            "spotify:end-group:8e3f1a2b",
        ] {
            let mut item = Item::new();
            item.set_uri(uri.to_string());
            lista.items.push(item);
        }

        let mut message = SelectedListContentMessage::new();
        message.contents = protobuf::MessageField::some(lista);

        let out = summaries_from(&message);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id.id.as_ref(), "37i9dQZEVXcJZyENOWUFo7");
    }

    #[test]
    fn both_uri_shapes_the_rootlist_mixes_are_accepted() {
        assert_eq!(
            playlist_id("spotify:playlist:37i9dQZEVXcJZyENOWUFo7").as_deref(),
            Some("37i9dQZEVXcJZyENOWUFo7")
        );
        assert_eq!(
            playlist_id("spotify:user:felipe:playlist:37i9dQZEVXcJZyENOWUFo7").as_deref(),
            Some("37i9dQZEVXcJZyENOWUFo7")
        );
        assert_eq!(owner_from_uri("spotify:user:felipe:playlist:37i9dQZEVXcJZyENOWUFo7").as_deref(), Some("felipe"));
    }

    #[test]
    fn anything_that_is_not_a_playlist_id_is_refused() {
        // Um id fora do base62 de 22 caracteres viraria requisicao para outro
        // endereco.
        assert!(playlist_id("spotify:start-group:8e3f1a2b:Minhas").is_none());
        assert!(playlist_id("spotify:album:37i9dQZEVXcJZyENOWUFo7").is_none());
        assert!(playlist_id("spotify:playlist:curto").is_none());
        assert!(playlist_id("spotify:playlist:../../me/player").is_none());
        assert!(playlist_id("nao e uri").is_none());
    }

    #[test]
    fn a_playlist_without_decoration_still_shows_up() {
        // Sem `meta_items` a playlist entra sem nome, que e melhor que sumir.
        let mut lista = ListItems::new();
        let mut item = Item::new();
        item.set_uri("spotify:playlist:37i9dQZEVXcJZyENOWUFo7".to_string());
        lista.items.push(item);

        let mut message = SelectedListContentMessage::new();
        message.contents = protobuf::MessageField::some(lista);

        let out = summaries_from(&message);
        assert_eq!(out.len(), 1);
        assert!(out[0].name.is_empty());
        assert_eq!(out[0].length, 0);
    }
}
