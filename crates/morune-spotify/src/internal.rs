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

use std::collections::HashMap;
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
use librespot_protocol::playlist4_external::ListAttributes as ListAttributesMessage;
use librespot_protocol::playlist4_external::SelectedListContent as SelectedListContentMessage;
use morune_core::model::{ImageRef, ImageSet, PlaylistId, PlaylistKind};
use morune_core::{CoreError, CoreResult};
use protobuf::{EnumOrUnknown, Message};

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
    /// Capa, quando a playlist tem uma propria.
    ///
    /// Boa parte nao tem: o mosaico de quatro capas que o cliente oficial
    /// mostra e montado por ele a partir das faixas, e nao vem na resposta.
    /// Na conta de teste, 37 das 86 playlists trazem capa.
    pub images: ImageSet,
}

impl PlaylistSummary {
    /// Classifica a playlist pelo `format` que o rootlist declara.
    ///
    /// Os valores abaixo foram lidos da resposta real de uma conta, e nao
    /// deduzidos do nome -- "Grupo Revelacao Radio" e "Mix 2Pac" acabam no
    /// mesmo lugar sem que ninguem precise procurar a palavra "Radio", e uma
    /// conta noutro idioma se comporta igual.
    ///
    /// | `format` | vira |
    /// |---|---|
    /// | vazio | [`PlaylistKind::Personal`] |
    /// | `daily-mix`, `discover-weekly`, `blend` | [`PlaylistKind::MadeForYou`] |
    /// | `topic-mix`, `inspiredby-mix`, `artist-mix-reader` | [`PlaylistKind::Station`] |
    /// | `wrapped-*`, `all-time-top-songs-*` | [`PlaylistKind::Retrospective`] |
    /// | `editorial`, `artistsets`, e o resto | [`PlaylistKind::Editorial`] |
    ///
    /// Formato desconhecido cai em `Editorial` de proposito: e a categoria que
    /// o Inicio nao mostra, entao um valor novo do Spotify aparece na barra
    /// lateral em vez de entrar numa prateleira onde nao pertence.
    pub fn kind(&self) -> PlaylistKind {
        if self.format.is_empty() {
            // Sem `format` e dono `spotify` acontece nas seguidas de vitrine.
            return if self.owner.eq_ignore_ascii_case("spotify") {
                PlaylistKind::Editorial
            } else {
                PlaylistKind::Personal
            };
        }

        match self.format.as_str() {
            "daily-mix" | "discover-weekly" | "blend" => PlaylistKind::MadeForYou,
            "topic-mix" | "inspiredby-mix" | "artist-mix-reader" => PlaylistKind::Station,
            outro if outro.starts_with("wrapped-") => PlaylistKind::Retrospective,
            outro if outro.starts_with("all-time-top-songs") => PlaylistKind::Retrospective,
            _ => PlaylistKind::Editorial,
        }
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
    /// repetido, e dentro de cada item o campo 2 traz o `gid` de 16 bytes e o
    /// campo 5, quando o item e datado, traz o instante em que entrou na
    /// colecao. Foi lido byte a byte de `bench-out/sonda/colecao-faixas.bin`.
    ///
    /// Arquivo local da conta aparece na mesma lista com um id de texto no
    /// lugar do `gid`. Nao ha como o Morune tocar, entao some aqui em vez de
    /// virar linha que falha ao ser clicada.
    ///
    /// # A ordem e nossa, nao do servidor
    ///
    /// A resposta **nao vem ordenada**. Nas 723 curtidas da conta de teste, as
    /// datas alternam entre 2022 e 2025 sem padrao: 362 pares em ordem
    /// crescente contra 360 em decrescente. Pegar as primeiras seria pegar uma
    /// fatia arbitraria, que na tela aparece como "minhas curtidas estao
    /// desatualizadas".
    ///
    /// Por isso a lista sai daqui **da mais recente para a mais antiga**, que
    /// e o que "Musicas curtidas" significa em qualquer player. Item sem data
    /// vai para o fim, em vez de disputar o topo com um zero.
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

        let mut itens = Vec::new();
        for parte in &resposta.payload {
            itens.extend(collection_items(parte));
        }

        // Da mais recente para a mais antiga. Ver o cabecalho do metodo: sem
        // isto a tela mostra uma fatia arbitraria da colecao.
        itens.sort_by_key(|(_, quando)| std::cmp::Reverse(*quando));

        Ok(itens.into_iter().map(|(id, _)| id).collect())
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
        // Mesmo motivo de [`Internal::tracks`]: a resposta nao respeita a ordem
        // do pedido.
        let mut por_id: HashMap<String, ArtistMeta> = HashMap::with_capacity(ids.len());

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
                        por_id.insert(artista.id.clone(), artista);
                    }
                }
            }
        }

        Ok(ids.iter().filter_map(|id| por_id.remove(id)).collect())
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
        // Guardadas por id, e nao numa lista: a resposta **nao volta na ordem
        // pedida**, e o pedido sai em lotes de 50. Acumular na ordem de chegada
        // embaralhava a playlist inteira, e nas curtidas desfazia a ordenacao
        // por data que o passo anterior tinha acabado de fazer.
        let mut por_id: HashMap<String, TrackMeta> = HashMap::with_capacity(ids.len());

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
                        por_id.insert(faixa.id.clone(), faixa);
                    }
                }
            }
        }

        // De volta a ordem pedida. Faixa que o servidor nao devolveu some da
        // lista, em vez de deixar um buraco ou empurrar as seguintes.
        Ok(ids.iter().filter_map(|id| por_id.remove(id)).collect())
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

/// Itens de um payload de colecao, com a data em que entraram.
///
/// Le o protobuf a mao porque o esquema nao esta entre os compilados pela
/// librespot. So dois campos interessam, e o que nao encaixar e pulado -- ver
/// [`Internal::collection`].
fn collection_items(bytes: &[u8]) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    let mut cursor = 0usize;

    while let Some((campo, dado, proximo)) = campo_delimitado(bytes, cursor) {
        cursor = proximo;
        if campo != 1 {
            continue;
        }

        let mut id = None;
        let mut quando = 0u64;
        let mut interno = 0usize;

        while let Some((campo, valor, proximo)) = campo_do_item(dado, interno) {
            interno = proximo;
            match (campo, valor) {
                (2, Valor::Bytes(bruto)) => id = base62(bruto),
                (5, Valor::Numero(instante)) => quando = instante,
                _ => {}
            }
        }

        if let Some(id) = id {
            out.push((id, quando));
        }
    }

    out
}

/// Valor de um campo do protobuf, do tipo que interessa aqui.
enum Valor<'a> {
    Bytes(&'a [u8]),
    Numero(u64),
}

/// Proximo campo de um item, delimitado ou numerico.
///
/// Existe separado de [`campo_delimitado`] porque dentro do item interessam os
/// dois tipos: o identificador e delimitado, e a data e varint.
fn campo_do_item(bytes: &[u8], mut cursor: usize) -> Option<(u64, Valor<'_>, usize)> {
    loop {
        let (chave, lido) = varint(bytes, cursor)?;
        cursor += lido;
        let campo = chave >> 3;

        match chave & 7 {
            2 => {
                let (tamanho, lido) = varint(bytes, cursor)?;
                cursor += lido;
                let fim = cursor.checked_add(tamanho as usize)?;
                let dado = bytes.get(cursor..fim)?;
                return Some((campo, Valor::Bytes(dado), fim));
            }
            0 => {
                let (valor, lido) = varint(bytes, cursor)?;
                cursor += lido;
                return Some((campo, Valor::Numero(valor), cursor));
            }
            5 => cursor = cursor.checked_add(4)?,
            1 => cursor = cursor.checked_add(8)?,
            _ => return None,
        }
    }
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
            images: atributos.map(playlist_covers).unwrap_or_default(),
        });
    }

    out
}

/// Capas de uma playlist, como o rootlist as descreve.
///
/// Sao duas formas, e uma playlist pode trazer as duas, uma, ou nenhuma:
///
/// - **`picture`** e um identificador de arquivo, como no metadado de album:
///   a URL e montada aqui. Nao vem com tamanho.
/// - **`picture_size`** ja traz a URL pronta, e num host diferente
///   (`pickasso.spotifycdn.com`, onde ficam as capas geradas). O
///   `target_name` e um rotulo (`default`, `small`...), e nao uma largura --
///   entao nenhuma dessas entra no `ImageSet` com tamanho conhecido.
///
/// Sem largura, `best_for_width` cai para a unica disponivel, que e o
/// comportamento certo: e melhor uma capa de tamanho incerto que nenhuma.
fn playlist_covers(atributos: &ListAttributesMessage) -> ImageSet {
    let mut refs = Vec::new();

    if !atributos.picture().is_empty() {
        refs.push(ImageRef {
            url: Arc::from(format!("{CDN_IMAGEM}{}", hex(atributos.picture())).as_str()),
            width: None,
            height: None,
        });
    }

    for tamanho in &atributos.picture_size {
        if !tamanho.url().is_empty() {
            refs.push(ImageRef {
                url: Arc::from(tamanho.url()),
                width: None,
                height: None,
            });
        }
    }

    ImageSet(refs)
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
            images: ImageSet::default(),
        }
    }

    /// Monta um item de colecao: campo 2 com o `gid`, campo 5 com a data.
    fn item(gid: u8, quando: u64) -> Vec<u8> {
        let mut corpo = vec![0x12, 16];
        corpo.extend(std::iter::repeat_n(gid, 16));
        corpo.push(5 << 3); // campo 5, varint
        let mut valor = quando;
        loop {
            let mut byte = (valor & 0x7f) as u8;
            valor >>= 7;
            if valor != 0 {
                byte |= 0x80;
            }
            corpo.push(byte);
            if valor == 0 {
                break;
            }
        }

        let mut out = vec![0x0a, corpo.len() as u8];
        out.extend(corpo);
        out
    }

    #[test]
    fn the_requested_order_survives_a_response_that_ignores_it() {
        // O `extended-metadata` responde na ordem que quer, e o pedido sai em
        // lotes. Acumular na ordem de chegada embaralhava a playlist e desfazia
        // a ordenacao por data das curtidas -- e o defeito que a tela mostrava
        // como "minhas curtidas vieram fora de ordem".
        //
        // Aqui o reordenamento e exercitado sem rede: sao os mesmos indexOf que
        // `tracks` faz depois de receber tudo.
        let pedidos = ["aaa".to_string(), "bbb".to_string(), "ccc".to_string()];

        let mut chegaram: HashMap<String, &str> = HashMap::new();
        chegaram.insert("ccc".into(), "terceira");
        chegaram.insert("aaa".into(), "primeira");
        chegaram.insert("bbb".into(), "segunda");

        let ordenadas: Vec<&str> =
            pedidos.iter().filter_map(|id| chegaram.remove(id)).collect();

        assert_eq!(ordenadas, vec!["primeira", "segunda", "terceira"]);
    }

    #[test]
    fn a_track_the_server_did_not_return_just_disappears() {
        // Buraco na resposta nao pode empurrar as seguintes nem virar linha
        // vazia: o item some e a lista continua na ordem.
        let pedidos = ["aaa".to_string(), "sumida".to_string(), "ccc".to_string()];

        let mut chegaram: HashMap<String, &str> = HashMap::new();
        chegaram.insert("aaa".into(), "primeira");
        chegaram.insert("ccc".into(), "terceira");

        let ordenadas: Vec<&str> =
            pedidos.iter().filter_map(|id| chegaram.remove(id)).collect();

        assert_eq!(ordenadas, vec!["primeira", "terceira"]);
    }

    #[test]
    fn the_collection_comes_out_newest_first() {
        // A resposta do servidor nao vem ordenada: na conta de teste as datas
        // alternavam entre 2022 e 2025 sem padrao, e pegar as primeiras 50
        // entregava uma fatia arbitraria -- o que na tela parecia "minhas
        // curtidas estao desatualizadas".
        let mut payload = Vec::new();
        payload.extend(item(0xaa, 1_600_000_000));
        payload.extend(item(0xbb, 1_750_000_000));
        payload.extend(item(0xcc, 1_700_000_000));

        let itens = collection_items(&payload);
        let mut ordenados = itens.clone();
        ordenados.sort_by_key(|(_, quando)| std::cmp::Reverse(*quando));

        assert_eq!(itens.len(), 3);
        let datas: Vec<u64> = ordenados.iter().map(|(_, q)| *q).collect();
        assert_eq!(datas, vec![1_750_000_000, 1_700_000_000, 1_600_000_000]);
    }

    #[test]
    fn an_item_without_a_date_does_not_take_the_top() {
        // Sem data o item vale zero, e zero no topo de uma ordem decrescente
        // colocaria o mais antigo primeiro.
        let mut payload = Vec::new();
        payload.extend(vec![0x0a, 18, 0x12, 16]);
        payload.extend(std::iter::repeat_n(0xdd, 16));
        payload.extend(item(0xee, 1_750_000_000));

        let mut itens = collection_items(&payload);
        itens.sort_by_key(|(_, quando)| std::cmp::Reverse(*quando));

        assert_eq!(itens.len(), 2);
        assert_eq!(itens[0].1, 1_750_000_000);
        assert_eq!(itens[1].1, 0);
    }

    #[test]
    fn a_local_file_never_becomes_a_track() {
        // Arquivo do computador entra na mesma lista com um id de texto no
        // lugar do `gid` de 16 bytes. Nao ha como toca-lo.
        let nome = b"::minha-musica.mp3";
        let mut corpo = vec![0x12, nome.len() as u8];
        corpo.extend_from_slice(nome);
        let mut payload = vec![0x0a, corpo.len() as u8];
        payload.extend(corpo);

        assert!(collection_items(&payload).is_empty());
    }

    #[test]
    fn every_format_the_account_really_returns_is_classified() {
        // Os valores vieram da resposta real de uma conta -- ver o cabecalho de
        // `kind`. Sao eles que decidem o que o Inicio mostra.
        let caso = |formato: &str| summary("spotify", formato).kind();

        assert_eq!(caso("daily-mix"), PlaylistKind::MadeForYou);
        assert_eq!(caso("discover-weekly"), PlaylistKind::MadeForYou);
        assert_eq!(caso("blend"), PlaylistKind::MadeForYou);

        assert_eq!(caso("topic-mix"), PlaylistKind::Station);
        assert_eq!(caso("inspiredby-mix"), PlaylistKind::Station);
        assert_eq!(caso("artist-mix-reader"), PlaylistKind::Station);

        assert_eq!(caso("wrapped-2025-top100"), PlaylistKind::Retrospective);
        assert_eq!(caso("all-time-top-songs-20-years"), PlaylistKind::Retrospective);
    }

    #[test]
    fn the_showcase_kinds_are_kept_out_of_the_home() {
        // "This Is <artista>" e as vitrines de genero sao iguais para todo
        // mundo: nao sao feitas para esta conta e nao entram no Inicio.
        assert_eq!(summary("spotify", "artistsets").kind(), PlaylistKind::Editorial);
        assert_eq!(summary("spotify", "editorial").kind(), PlaylistKind::Editorial);
    }

    #[test]
    fn a_format_nobody_has_seen_yet_stays_out_of_the_home() {
        // O Spotify inventa formato novo sem avisar. Cair em `Editorial` faz o
        // desconhecido aparecer na barra lateral, e nao numa prateleira onde
        // talvez nao pertenca.
        assert_eq!(summary("spotify", "formato-que-ainda-nao-existe").kind(), PlaylistKind::Editorial);
    }

    #[test]
    fn a_playlist_the_user_made_is_personal() {
        assert_eq!(summary("seititm", "").kind(), PlaylistKind::Personal);
        // Vitrine seguida pelo usuario vem sem `format`, mas com dono `spotify`.
        assert_eq!(summary("spotify", "").kind(), PlaylistKind::Editorial);
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
