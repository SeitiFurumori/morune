//! Cache de capas em disco.
//!
//! # Por que existe um teto
//!
//! O criterio de desempenho do Morune nao e economizar MB: e nao atrapalhar
//! quem esta jogando. Um cache sem limite acaba disputando disco e memoria com
//! o resto da maquina, e capa e exatamente o tipo de dado que cresce sem parar
//! -- cada album visto uma vez fica guardado para sempre.
//!
//! Por isso o teto e explicito e o descarte e por **menos usado recentemente**.
//! O valor do teto e livre, e esta em [`TETO_BYTES`].
//!
//! # Como a interface consome
//!
//! A thread da interface nunca espera por rede. O pedido vira tarefa no runtime
//! do backend, e o resultado e recolhido no mesmo temporizador de 100 ms que ja
//! atende bandeja, reproducao e navegacao. Enquanto a capa nao chega, o cartao
//! aparece sem ela -- nunca com um espaco reservado que pula depois.
//!
//! # O arquivo
//!
//! O nome vem do hash da URL, e nao da URL: uma URL do Spotify tem barra e
//! caracteres que nao cabem em nome de arquivo, e o hash tambem impede que uma
//! resposta adulterada escreva fora da pasta do cache.
//!
//! **A extensao importa.** O decodificador do Slint escolhe o formato pelo
//! nome do arquivo, e nao pelo conteudo: gravar um JPEG como `.img` faz
//! `load_from_path` falhar antes de olhar um byte. Por isso o formato e
//! reconhecido pela assinatura no momento da gravacao, e a extensao sai dela.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use morune_core::catalog::Artwork;
use tokio::sync::mpsc::UnboundedSender;

/// Teto do cache em disco.
///
/// Cabem cerca de 1.500 capas de 300 px. E o suficiente para uma biblioteca
/// grande ser navegada inteira sem baixar duas vezes, e pequeno o bastante para
/// nao ser notado num disco moderno.
const TETO_BYTES: u64 = 48 * 1024 * 1024;

/// Quanto do teto sobra depois de uma limpeza.
///
/// Limpar ate exatamente o teto faria a proxima capa disparar outra limpeza.
/// Descer a 80% deixa folga para as proximas dezenas.
const ALVO_APOS_LIMPEZA: u64 = TETO_BYTES * 4 / 5;

/// Extensoes que o cache usa, e as unicas que ele procura em disco.
///
/// Sao as duas que o Spotify serve. A ordem importa so na busca: JPEG e o
/// caso comum, entao vem primeiro.
const EXTENSOES: [&str; 2] = ["jpg", "png"];

/// Esperas antes de refazer um download que falhou de forma transitoria.
///
/// A capa nao e critica para a navegacao, entao a primeira falha nao deve
/// congestionar a rede nem decidir permanentemente o resultado. Duas novas
/// tentativas cobrem quedas curtas sem transformar uma URL indisponivel em
/// trafego continuo.
const ESPERAS_DE_NOVA_TENTATIVA: [Duration; 2] = [Duration::from_secs(1), Duration::from_secs(3)];

/// Capa pronta para a interface desenhar.
#[derive(Debug, Clone)]
pub struct Ready {
    /// A URL pedida, que e como a tela reencontra quem pediu.
    pub url: String,
    pub path: PathBuf,
}

/// Cache de capas.
#[derive(Debug)]
pub struct ArtworkCache {
    dir: PathBuf,
    /// URLs ja pedidas nesta sessao, para nao baixar a mesma capa duas vezes
    /// quando ela aparece em varios cartoes da mesma tela.
    pedidas: HashMap<String, Estado>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Estado {
    /// Download em andamento, incluindo suas tentativas limitadas para uma
    /// falha transitoria. Enquanto isso, a mesma URL nao abre uma segunda
    /// tarefa nem duplica trafego.
    Pedida,
    Pronta(PathBuf),
}

impl ArtworkCache {
    pub fn new(dir: PathBuf) -> Self {
        if let Err(e) = std::fs::create_dir_all(&dir) {
            // Sem pasta o aplicativo continua: as capas simplesmente nao
            // aparecem, o que e melhor que nao abrir.
            tracing::warn!(dir = %dir.display(), error = %e, "cache de capas indisponivel");
        }
        Self {
            dir,
            pedidas: HashMap::new(),
        }
    }

    /// Caminho da capa, se ela ja estiver em disco.
    ///
    /// Sincrono e barato de proposito: e chamado enquanto a tela e montada, e
    /// uma capa ja baixada tem de aparecer no primeiro quadro.
    pub fn cached(&mut self, url: &str) -> Option<PathBuf> {
        if let Some(Estado::Pronta(path)) = self.pedidas.get(url) {
            return Some(path.clone());
        }

        // A extensao depende do que o servidor mandou, entao nao da para
        // deduzi-la da URL: procura-se pelas conhecidas.
        let achado = EXTENSOES
            .iter()
            .map(|ext| self.caminho(url, ext))
            .find(|path| path.is_file())?;

        self.pedidas
            .insert(url.to_string(), Estado::Pronta(achado.clone()));
        Some(achado)
    }

    /// Pede uma capa que ainda nao esta em disco.
    ///
    /// Devolve `true` quando a busca comecou. `false` significa que esta URL ja
    /// foi pedida nesta sessao, ou que nao ha o que buscar.
    pub fn request(
        &mut self,
        url: &str,
        source: &Arc<dyn Artwork>,
        runtime: &tokio::runtime::Handle,
        tx: UnboundedSender<Ready>,
    ) -> bool {
        if url.is_empty() || self.pedidas.contains_key(url) {
            return false;
        }

        self.pedidas.insert(url.to_string(), Estado::Pedida);

        let source = source.clone();
        let url = url.to_string();
        let dir = self.dir.clone();
        let base = hash(&url);

        runtime.spawn(async move {
            for tentativa in 0..=ESPERAS_DE_NOVA_TENTATIVA.len() {
                match source.fetch(&url).await {
                Ok(bytes) => {
                    let Some(extensao) = formato(&bytes) else {
                        tracing::debug!(url, "capa em formato que o Morune nao desenha");
                        return;
                    };
                    let path = dir.join(format!("{base:016x}.{extensao}"));

                    // Grava em temporario e renomeia: uma interrupcao no meio
                    // deixaria um arquivo truncado que o decodificador tentaria
                    // abrir em toda abertura seguinte.
                    let temporario = dir.join(format!("{base:016x}.parcial"));
                    if std::fs::write(&temporario, &bytes).is_ok()
                        && std::fs::rename(&temporario, &path).is_ok()
                    {
                        limpar_se_passou_do_teto(&dir);
                        let _ = tx.send(Ready { url, path });
                    }
                    return;
                }
                Err(error) if error.is_retryable() => {
                    let Some(espera) = espera_de_nova_tentativa(tentativa) else {
                        tracing::debug!(url, error = %error, "capa nao baixou apos novas tentativas");
                        return;
                    };
                    tracing::debug!(url, error = %error, tentativa = tentativa + 1, "capa nao baixou; tentara de novo");
                    tokio::time::sleep(espera).await;
                }
                Err(error) => {
                    tracing::debug!(url, error = %error, "capa nao baixou");
                    return;
                }
                }
            }
        });

        true
    }

    /// Registra o resultado que a interface recolheu.
    pub fn settle(&mut self, ready: &Ready) {
        self.pedidas
            .insert(ready.url.clone(), Estado::Pronta(ready.path.clone()));
    }

    fn caminho(&self, url: &str, extensao: &str) -> PathBuf {
        self.dir.join(format!("{:016x}.{extensao}", hash(url)))
    }
}

/// Espera depois de uma falha, ou `None` quando o limite ja foi atingido.
fn espera_de_nova_tentativa(tentativa: usize) -> Option<Duration> {
    ESPERAS_DE_NOVA_TENTATIVA.get(tentativa).copied()
}

/// Extensao correspondente ao formato da imagem, pela assinatura.
///
/// O `Content-Type` da resposta seria o caminho obvio, mas o contrato
/// [`Artwork`] devolve so os bytes -- e a assinatura e mais confiavel que o
/// cabecalho de qualquer jeito. `None` para o que o Slint nao desenha, e
/// nesse caso nada e gravado: um arquivo que nunca vai abrir so ocuparia
/// espaco no teto do cache.
fn formato(bytes: &[u8]) -> Option<&'static str> {
    match bytes {
        [0xff, 0xd8, 0xff, ..] => Some("jpg"),
        [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, ..] => Some("png"),
        _ => None,
    }
}

/// Hash FNV-1a de 64 bits.
///
/// Escrito a mao para nao trazer uma dependencia por causa de um nome de
/// arquivo. Nao e criptografico e nao precisa ser: colisao aqui custa uma capa
/// errada, e o espaco de 64 bits torna isso improvavel o bastante para um cache
/// de alguns milhares de itens.
fn hash(texto: &str) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    texto
        .bytes()
        .fold(OFFSET, |acc, b| (acc ^ u64::from(b)).wrapping_mul(PRIME))
}

/// Descarta as capas menos usadas recentemente quando o teto e ultrapassado.
///
/// "Menos usada recentemente" e lida do proprio sistema de arquivos, pelo
/// horario de acesso, com queda para o de modificacao onde o sistema nao
/// atualiza o primeiro -- que e o caso do NTFS com `lastaccess` desligado, o
/// padrao no Windows. Nesse caso o criterio vira "mais antiga", que e pior mas
/// nunca errado: o que sai continua sendo o que sera baixado de novo mais
/// tarde.
fn limpar_se_passou_do_teto(dir: &Path) {
    let Ok(entradas) = std::fs::read_dir(dir) else {
        return;
    };

    let mut arquivos: Vec<(PathBuf, u64, std::time::SystemTime)> = Vec::new();
    let mut total = 0u64;

    for entrada in entradas.flatten() {
        let Ok(meta) = entrada.metadata() else {
            continue;
        };
        if !meta.is_file() {
            continue;
        }
        let quando = meta
            .accessed()
            .or_else(|_| meta.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        total += meta.len();
        arquivos.push((entrada.path(), meta.len(), quando));
    }

    if total <= TETO_BYTES {
        return;
    }

    arquivos.sort_by_key(|(_, _, quando)| *quando);

    for (path, tamanho, _) in arquivos {
        if total <= ALVO_APOS_LIMPEZA {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            total = total.saturating_sub(tamanho);
        }
    }

    tracing::debug!(restante = total, "cache de capas limpo");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use morune_core::{CoreError, CoreResult};

    /// Pasta temporaria propria.
    ///
    /// Escrita a mao, como em `morune-theme`, para nao trazer uma dependencia
    /// de teste por causa de quatro linhas.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(marca: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "morune-capas-{marca}-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&path).expect("pasta temporaria");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn cache(marca: &str) -> (TempDir, ArtworkCache) {
        let dir = TempDir::new(marca);
        let cache = ArtworkCache::new(dir.path().to_path_buf());
        (dir, cache)
    }

    #[test]
    fn a_url_becomes_a_filename_that_cannot_escape_the_cache() {
        // Uma URL tem barra e `?`; usada crua, escreveria fora da pasta.
        let (dir, cache) = cache("escapar");
        let path = cache.caminho("https://i.scdn.co/image/ab67616d/../../senha", "jpg");

        assert_eq!(path.parent(), Some(dir.path()));
        assert!(path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .ends_with(".jpg"));
    }

    #[test]
    fn the_same_url_always_lands_on_the_same_file() {
        let (_dir, cache) = cache("mesmo-arquivo");
        let a = cache.caminho("https://i.scdn.co/image/abc", "jpg");
        let b = cache.caminho("https://i.scdn.co/image/abc", "jpg");
        let outra = cache.caminho("https://i.scdn.co/image/def", "jpg");

        assert_eq!(a, b);
        assert_ne!(a, outra);
    }

    #[test]
    fn a_cover_already_on_disk_is_found_without_network() {
        let (dir, mut cache) = cache("em-disco");
        let url = "https://i.scdn.co/image/abc";
        std::fs::write(cache.caminho(url, "jpg"), b"jpeg").unwrap();

        let achado = cache.cached(url).expect("capa em disco");
        assert_eq!(achado.parent(), Some(dir.path()));
        // Segunda consulta vem da memoria, sem tocar no disco de novo.
        assert_eq!(cache.cached(url), Some(achado));
    }

    #[test]
    fn a_cover_that_is_not_there_is_not_invented() {
        let (_dir, mut cache) = cache("ausente");
        assert!(cache.cached("https://i.scdn.co/image/nao-existe").is_none());
    }

    #[test]
    fn the_ceiling_discards_the_oldest_first() {
        let dir = TempDir::new("teto");

        // Tres arquivos que somados passam do teto, gravados em ordem.
        let grande = vec![0u8; (TETO_BYTES / 2) as usize];
        for nome in ["a.jpg", "b.jpg", "c.jpg"] {
            std::fs::write(dir.path().join(nome), &grande).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        limpar_se_passou_do_teto(dir.path());

        let restantes: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();

        // O mais antigo saiu, e o cache voltou para dentro do teto.
        assert!(
            !restantes.contains(&"a.jpg".to_string()),
            "restaram: {restantes:?}"
        );
        assert!(restantes.len() < 3);
    }

    #[test]
    fn the_extension_comes_from_the_bytes_not_from_the_url() {
        // O decodificador do Slint escolhe o formato pelo nome do arquivo.
        // Gravar um JPEG como `.img` fazia todas as capas falharem ao abrir,
        // mesmo com os bytes corretos em disco.
        assert_eq!(formato(&[0xff, 0xd8, 0xff, 0xe0, 0, 0]), Some("jpg"));
        assert_eq!(
            formato(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0]),
            Some("png")
        );
        // Formato que o Slint nao desenha nao vira arquivo: ocuparia teto do
        // cache para nunca abrir.
        assert_eq!(formato(b"GIF89a..."), None);
        assert_eq!(formato(b""), None);
    }

    #[test]
    fn a_cover_below_the_ceiling_is_never_discarded() {
        let dir = TempDir::new("abaixo");
        std::fs::write(dir.path().join("a.jpg"), b"pequena").unwrap();

        limpar_se_passou_do_teto(dir.path());

        assert!(dir.path().join("a.jpg").is_file());
    }

    #[test]
    fn falhas_transitorias_tem_retentativas_limitadas_e_crescentes() {
        assert_eq!(espera_de_nova_tentativa(0), Some(Duration::from_secs(1)));
        assert_eq!(espera_de_nova_tentativa(1), Some(Duration::from_secs(3)));
        assert_eq!(espera_de_nova_tentativa(2), None);
    }

    struct FalhaUmaVez {
        tentativas: AtomicUsize,
    }

    impl Artwork for FalhaUmaVez {
        fn fetch<'a>(
            &'a self,
            _url: &'a str,
        ) -> Pin<Box<dyn Future<Output = CoreResult<Vec<u8>>> + Send + 'a>> {
            Box::pin(async move {
                if self.tentativas.fetch_add(1, Ordering::SeqCst) == 0 {
                    Err(CoreError::Network("queda breve".into()))
                } else {
                    Ok(vec![0xff, 0xd8, 0xff])
                }
            })
        }
    }

    #[test]
    fn uma_falha_transitoria_baixa_a_capa_na_segunda_tentativa() {
        let (_dir, mut cache) = cache("repetir");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("runtime de teste");
        let flaky = Arc::new(FalhaUmaVez {
            tentativas: AtomicUsize::new(0),
        });
        let source: Arc<dyn Artwork> = flaky.clone();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        assert!(cache.request(
            "https://i.scdn.co/image/repetir",
            &source,
            runtime.handle(),
            tx,
        ));

        let ready = runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(2), rx.recv())
                .await
                .expect("a segunda tentativa deve terminar")
                .expect("a tarefa deve enviar a capa pronta")
        });

        assert_eq!(flaky.tentativas.load(Ordering::SeqCst), 2);
        assert!(ready.path.is_file());
    }
}
