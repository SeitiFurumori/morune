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

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

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
    /// Ja foi pedida uma vez nesta sessao. Cobre tanto o download em andamento
    /// quanto o que falhou: nos dois casos a resposta e a mesma -- nao pedir de
    /// novo. Insistir numa URL que nao respondeu gastaria rede em toda rolagem
    /// da tela.
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
        Self { dir, pedidas: HashMap::new() }
    }

    /// Caminho da capa, se ela ja estiver em disco.
    ///
    /// Sincrono e barato de proposito: e chamado enquanto a tela e montada, e
    /// uma capa ja baixada tem de aparecer no primeiro quadro.
    pub fn cached(&mut self, url: &str) -> Option<PathBuf> {
        if let Some(Estado::Pronta(path)) = self.pedidas.get(url) {
            return Some(path.clone());
        }

        let path = self.caminho(url);
        path.is_file().then(|| {
            self.pedidas.insert(url.to_string(), Estado::Pronta(path.clone()));
            path
        })
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
        let path = self.caminho(&url);
        let dir = self.dir.clone();

        runtime.spawn(async move {
            match source.fetch(&url).await {
                Ok(bytes) => {
                    // Grava em temporario e renomeia: uma interrupcao no meio
                    // deixaria um arquivo truncado que o decodificador tentaria
                    // abrir em toda abertura seguinte.
                    let temporario = path.with_extension("parcial");
                    if std::fs::write(&temporario, &bytes).is_ok()
                        && std::fs::rename(&temporario, &path).is_ok()
                    {
                        limpar_se_passou_do_teto(&dir);
                        let _ = tx.send(Ready { url, path });
                    }
                }
                Err(e) => tracing::debug!(url, error = %e, "capa nao baixou"),
            }
        });

        true
    }

    /// Registra o resultado que a interface recolheu.
    pub fn settle(&mut self, ready: &Ready) {
        self.pedidas.insert(ready.url.clone(), Estado::Pronta(ready.path.clone()));
    }

    fn caminho(&self, url: &str) -> PathBuf {
        self.dir.join(format!("{:016x}.img", hash(url)))
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

    texto.bytes().fold(OFFSET, |acc, b| (acc ^ u64::from(b)).wrapping_mul(PRIME))
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
    let Ok(entradas) = std::fs::read_dir(dir) else { return };

    let mut arquivos: Vec<(PathBuf, u64, std::time::SystemTime)> = Vec::new();
    let mut total = 0u64;

    for entrada in entradas.flatten() {
        let Ok(meta) = entrada.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        let quando = meta.accessed().or_else(|_| meta.modified()).unwrap_or(std::time::UNIX_EPOCH);
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
        let path = cache.caminho("https://i.scdn.co/image/ab67616d/../../senha");

        assert_eq!(path.parent(), Some(dir.path()));
        assert!(path.file_name().unwrap().to_str().unwrap().ends_with(".img"));
    }

    #[test]
    fn the_same_url_always_lands_on_the_same_file() {
        let (_dir, cache) = cache("mesmo-arquivo");
        let a = cache.caminho("https://i.scdn.co/image/abc");
        let b = cache.caminho("https://i.scdn.co/image/abc");
        let outra = cache.caminho("https://i.scdn.co/image/def");

        assert_eq!(a, b);
        assert_ne!(a, outra);
    }

    #[test]
    fn a_cover_already_on_disk_is_found_without_network() {
        let (dir, mut cache) = cache("em-disco");
        let url = "https://i.scdn.co/image/abc";
        std::fs::write(cache.caminho(url), b"jpeg").unwrap();

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
        for nome in ["a.img", "b.img", "c.img"] {
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
        assert!(!restantes.contains(&"a.img".to_string()), "restaram: {restantes:?}");
        assert!(restantes.len() < 3);
    }

    #[test]
    fn a_cover_below_the_ceiling_is_never_discarded() {
        let dir = TempDir::new("abaixo");
        std::fs::write(dir.path().join("a.img"), b"pequena").unwrap();

        limpar_se_passou_do_teto(dir.path());

        assert!(dir.path().join("a.img").is_file());
    }
}
