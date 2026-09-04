//! Atualizacao pelo proprio aplicativo.
//!
//! O Morune e um binario nativo: nao existe camada web para trocar sem tocar no
//! executavel. O que da para evitar e a viagem ate o navegador -- o aplicativo
//! pergunta ao GitHub qual e a ultima versao, baixa o mesmo instalador que
//! qualquer pessoa baixaria e o executa em modo silencioso.
//!
//! **Tres decisoes que definem o modulo:**
//!
//! 1. **Verifica sozinho no maximo uma vez por dia, e mais nada.** A versao
//!    anterior so verificava por clique, e isso falhou em uso real: o Morune
//!    inicia com o Windows e fica semanas aberto sem que ninguem abra as
//!    Configuracoes -- o dono passou duas semanas num binario velho achando que
//!    estava em dia. Uma consulta por dia nao briga com o criterio de
//!    desempenho do projeto (nao atrapalhar quem esta jogando); um relogio de
//!    minutos brigaria. A marca do "ja verifiquei hoje" fica em disco, e nao em
//!    memoria, porque o caso que importa e o do processo que reinicia junto com
//!    a sessao do Windows todo dia.
//!    Verificacao automatica **nao escreve erro na tela**: quem nao perguntou
//!    nao pode receber "sem resposta do GitHub" no meio da musica.
//! 2. **Baixar e instalar sao dois cliques.** Instalar fecha o aplicativo, e
//!    fechar o aplicativo interrompe a musica. Isso nunca pode acontecer sem
//!    que a pessoa tenha pedido, entao o download termina num botao novo, e nao
//!    numa reinicializacao.
//! 3. **O hash e conferido antes de executar qualquer coisa.** O `.sha256`
//!    publicado ao lado do instalador protege contra download corrompido ou
//!    truncado. Ele **nao** e prova de origem -- vem do mesmo servidor que o
//!    arquivo. Quem faz essa parte e a assinatura de codigo, que ainda nao
//!    existe: ver `docs/SIGNING.md`.
//!
//! A rede vive numa thread propria e conversa por canal, no mesmo desenho que
//! `session.rs` e `browse.rs` usam: quem clica nao bloqueia, e o resultado e
//! recolhido no temporizador que ja roda.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

use sha2::{Digest, Sha256};

/// De onde vem a resposta sobre a ultima versao.
///
/// **Nao e o `/releases/latest`**, e a diferenca importa: aquele endpoint
/// ignora pre-lancamentos, e todo lancamento que o Morune ja publicou e um
/// (`v0.1.0-alpha.5` em diante). Ele responderia 404 e o botao diria "não foi
/// possível verificar" para sempre.
///
/// A lista vem em ordem decrescente de criacao, mas quem decide qual e a mais
/// nova e a comparacao de versao, nao a posicao: republicar uma tag antiga --
/// que o `workflow_dispatch` do projeto permite -- reordenaria a lista.
const RELEASES_API: &str = "https://api.github.com/repos/SeitiFurumori/morune/releases?per_page=30";

/// Pagina de lancamentos, para quando o download falha.
pub const RELEASES_URL: &str = "https://github.com/SeitiFurumori/morune/releases/latest";

/// Tempo maximo para a consulta de versao.
///
/// Curto de proposito: e um clique com resposta na tela, e uma pessoa em rede
/// ruim prefere "nao consegui verificar" a um botao girando por um minuto.
const CHECK_TIMEOUT: Duration = Duration::from_secs(15);

/// Tempo maximo do download inteiro. O instalador tem cerca de 10 MB; quinze
/// minutos cobrem uma conexao bem ruim sem deixar a thread presa para sempre.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(900);

/// Teto do que aceitamos baixar, para que uma resposta inesperada nao encha o
/// disco de quem clicou em atualizar.
const MAX_SETUP_BYTES: u64 = 128 * 1024 * 1024;

/// Versao semantica, no minimo necessario para responder "isto e mais novo?".
///
/// Nao usa o crate `semver` porque essa e a unica pergunta feita aqui, e o
/// formato das tags do projeto ja e verificado pelo workflow de release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
    /// Sufixo de pre-lancamento, sem o hifen. `None` e uma versao final.
    pre: Option<String>,
}

impl Version {
    /// Le `1.2.3`, `v1.2.3` ou `v1.2.3-alpha.1`.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let text = text.strip_prefix('v').unwrap_or(text);
        let (nums, pre) = match text.split_once('-') {
            Some((nums, pre)) if !pre.is_empty() => (nums, Some(pre.to_string())),
            Some(_) => return None,
            None => (text, None),
        };

        let mut parts = nums.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        if parts.next().is_some() {
            return None;
        }

        Some(Self {
            major,
            minor,
            patch,
            pre,
        })
    }

    /// `true` para `0.2.0-alpha.1`; `false` para `0.2.0`.
    pub fn is_prerelease(&self) -> bool {
        self.pre.is_some()
    }

    /// `true` num binario compilado fora do workflow de publicacao.
    ///
    /// O rotulo vem de `embed_release_tag` em `build.rs`, que sem a variavel do
    /// workflow monta `v0.1.0-dev.<hash>`. Serve para a tela parar de prometer
    /// atualizacao automatica a um binario que nao tem lancamento de onde vir.
    pub fn is_dev(&self) -> bool {
        self.pre
            .as_deref()
            .is_some_and(|pre| pre == "dev" || pre.starts_with("dev."))
    }

    /// Versao deste executavel.
    ///
    /// Vem da **tag** do lancamento, e nao do `Cargo.toml`. Os dois nao sao a
    /// mesma coisa no Morune: `workspace.package.version` fica em `0.1.0`
    /// enquanto as tags avancam em `v0.1.0-alpha.5`, `alpha.6`, `alpha.7`. Lido
    /// do Cargo.toml, um alpha.5 instalado se anunciaria como `0.1.0` -- uma
    /// versao *final*, que por semver e mais nova que qualquer alpha -- e o
    /// botao responderia "você já está na versão mais recente" com tres
    /// lancamentos novos publicados. Ver `embed_release_tag` em build.rs.
    pub fn current() -> Self {
        let tag = env!("MORUNE_RELEASE");
        Self::parse(tag).unwrap_or_else(|| {
            // Um `MORUNE_RELEASE_TAG` malformado passado ao build nao pode
            // derrubar a tela de configuracoes; a versao do crate e o piso.
            tracing::warn!(tag, "tag de lancamento invalida; usando a do crate");
            Self::parse(morune_core::VERSION).expect("a versao do crate e valida")
        })
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.pre {
            write!(f, "-{pre}")?;
        }
        Ok(())
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (&self.pre, &other.pre) {
                // Semver: 1.0.0-alpha vem antes de 1.0.0.
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(a), Some(b)) => compare_prerelease(a, b),
            })
    }
}

/// Compara dois sufixos de pre-lancamento pelas regras do semver.
///
/// **Nao e comparacao de texto**, e a diferenca ja apareceria neste projeto:
/// `alpha.10` e menor que `alpha.9` em ordem alfabetica, e o Morune esta em
/// `alpha.8`. Um identificador puramente numerico compara como numero; os
/// demais, como texto; e um numero perde para um nome. Quem tem menos
/// identificadores vem antes, quando todos os anteriores empatam.
fn compare_prerelease(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let mut esquerda = a.split('.');
    let mut direita = b.split('.');

    loop {
        let ordem = match (esquerda.next(), direita.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => match (x.parse::<u64>(), y.parse::<u64>()) {
                (Ok(x), Ok(y)) => x.cmp(&y),
                (Ok(_), Err(_)) => Ordering::Less,
                (Err(_), Ok(_)) => Ordering::Greater,
                (Err(_), Err(_)) => x.cmp(y),
            },
        };

        if ordem != Ordering::Equal {
            return ordem;
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Um lancamento publicado, ja reduzido ao que a tela e o download precisam.
#[derive(Debug, Clone)]
pub struct Release {
    pub version: Version,
    /// Corpo da release, cortado no que cabe na tela.
    pub notes: String,
    pub setup_url: String,
    pub setup_name: String,
    pub sha_url: String,
    pub size: u64,
}

/// Em que ponto do fluxo o aplicativo esta. E o que a tela desenha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    /// Ninguem verificou nesta sessao.
    Idle,
    Checking,
    /// Verificou, e ja esta na ultima versao.
    UpToDate,
    /// Ha versao nova, ainda nao baixada.
    Available,
    /// Baixando, com o percentual ja conhecido.
    Downloading(u8),
    /// Instalador na maquina e com o hash conferido.
    Ready,
    /// Deu errado. A mensagem esta pronta para a tela.
    Failed(String),
}

/// O que a thread de rede manda de volta.
enum Message {
    /// `None` quando a versao publicada nao e mais nova que a instalada.
    ///
    /// `Box` porque `Release` e de longe a maior variante, e sem ele toda
    /// mensagem de progresso pagaria o tamanho dela.
    Checked(Result<Option<Box<Release>>, String>),
    Progress(u8),
    Downloaded(Result<PathBuf, String>),
}

/// Verificacao e download da proxima versao.
pub struct Updater {
    current: Version,
    phase: Phase,
    release: Option<Release>,
    /// Instalador baixado e conferido, esperando a confirmacao de instalar.
    ready: Option<PathBuf>,
    /// Trabalho em andamento, quando ha.
    pending: Option<Receiver<Message>>,
    /// Onde os instaladores baixados ficam.
    dir: PathBuf,
    /// A verificacao em voo comecou sozinha, e nao por clique.
    ///
    /// Muda o desfecho de uma falha: quem clicou espera resposta na tela, e
    /// quem nao clicou nao pode receber um erro de rede no meio do que estava
    /// fazendo. Ver [`Updater::poll`].
    automatica: bool,
    /// Versao que a verificacao automatica achou e que a tela ainda nao
    /// anunciou. Recolhida uma unica vez por [`Updater::take_anuncio`].
    anuncio: Option<Version>,
}

impl std::fmt::Debug for Updater {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Updater")
            .field("atual", &self.current)
            .field("fase", &self.phase)
            .finish_non_exhaustive()
    }
}

impl Updater {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            current: Version::current(),
            phase: Phase::Idle,
            release: None,
            ready: None,
            pending: None,
            dir,
            automatica: false,
            anuncio: None,
        }
    }

    /// `true` quando este binario nao saiu do workflow de publicacao.
    pub fn is_dev(&self) -> bool {
        self.current.is_dev()
    }

    /// Recolhe o anuncio pendente da verificacao automatica, se houver.
    ///
    /// Consome: a mensagem na tela e escrita uma vez, e nao a cada tique.
    pub fn take_anuncio(&mut self) -> Option<Version> {
        self.anuncio.take()
    }

    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    pub fn release(&self) -> Option<&Release> {
        self.release.as_ref()
    }

    pub fn current(&self) -> &Version {
        &self.current
    }

    /// `true` enquanto ha rede em voo. A tela desabilita o botao.
    pub fn busy(&self) -> bool {
        matches!(self.phase, Phase::Checking | Phase::Downloading(_))
    }

    /// Instalador pronto para executar, se ja baixado e conferido.
    pub fn installer(&self) -> Option<&Path> {
        self.ready.as_deref()
    }

    /// Verifica sozinho, no maximo uma vez por dia.
    ///
    /// **Por que existe:** o botao "Procurar atualizações" so roda quando
    /// alguem clica, e o Morune inicia com o Windows e fica semanas aberto sem
    /// que ninguem abra as Configuracoes. O resultado real foi um aplicativo
    /// rodando um binario de duas semanas atras com o dono achando que estava
    /// em dia.
    ///
    /// A marca fica em disco, e nao em memoria, justamente porque o caso que
    /// importa e o do processo que reinicia com a sessao do Windows todo dia:
    /// uma marca em memoria verificaria a cada boot.
    ///
    /// Build local nao verifica: nao ha lancamento publicado que sirva de
    /// atualizacao para ele (ver [`Version::is_dev`]), e a requisicao seria
    /// gasto sem desfecho possivel.
    pub fn auto_check(&mut self) {
        if self.busy() || self.is_dev() || !self.auto_check_due() {
            return;
        }

        self.stamp_auto_check();
        self.automatica = true;
        self.check_inner();
    }

    /// Passou um dia desde a ultima verificacao automatica.
    ///
    /// Marca ilegivel ou no futuro conta como vencida: o relogio do sistema
    /// pode ter andado para tras, e o pior desfecho de verificar a mais e uma
    /// requisicao extra.
    fn auto_check_due(&self) -> bool {
        const INTERVALO: Duration = Duration::from_secs(24 * 60 * 60);

        let Ok(texto) = std::fs::read_to_string(self.stamp_path()) else {
            return true;
        };
        let Ok(marcado) = texto.trim().parse::<u64>() else {
            return true;
        };
        let Ok(agora) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) else {
            return true;
        };

        agora.as_secs().saturating_sub(marcado) >= INTERVALO.as_secs()
    }

    /// Grava a marca **antes** de verificar, e nao depois.
    ///
    /// Uma falha de rede nao pode transformar cada tique em uma tentativa nova:
    /// sem internet, gravar so no sucesso faria o aplicativo bater no GitHub a
    /// cada 100 ms.
    fn stamp_auto_check(&self) {
        let Ok(agora) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) else {
            return;
        };
        let _ = std::fs::create_dir_all(&self.dir);
        if let Err(erro) = std::fs::write(self.stamp_path(), agora.as_secs().to_string()) {
            // Sem a marca a verificacao volta a acontecer a cada abertura --
            // chato, nao quebrado. Nao vale interromper nada por isso.
            tracing::debug!(%erro, "nao consegui gravar a marca de verificacao");
        }
    }

    fn stamp_path(&self) -> PathBuf {
        self.dir.join("ultima-verificacao")
    }

    /// Pergunta ao GitHub qual e a ultima versao.
    ///
    /// Num build local nao chega a perguntar: nenhum lancamento publicado pode
    /// ser mais novo que ele (ver [`Version::is_dev`]), e a resposta seria
    /// sempre a mesma. A tela explica o que esse desfecho significa.
    pub fn check(&mut self) {
        if self.busy() {
            return;
        }
        if self.is_dev() {
            self.release = None;
            self.phase = Phase::UpToDate;
            return;
        }
        self.automatica = false;
        self.check_inner();
    }

    fn check_inner(&mut self) {
        let current = self.current.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("morune-update-check".into())
            .spawn(move || {
                let result = fetch_latest(&current)
                    .map(|found| found.map(Box::new))
                    .map_err(|e| e.to_string());
                let _ = tx.send(Message::Checked(result));
            });

        match spawned {
            Ok(_) => {
                self.phase = Phase::Checking;
                self.pending = Some(rx);
            }
            // Falta de recurso do sistema. A tela precisa sair de "verificando"
            // de qualquer jeito.
            Err(e) => self.phase = Phase::Failed(format!("Não foi possível verificar: {e}")),
        }
    }

    /// Baixa o instalador da versao encontrada e confere o hash.
    pub fn download(&mut self) {
        if self.busy() {
            return;
        }
        let Some(release) = self.release.clone() else {
            return;
        };

        let dir = self.dir.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let progress = tx.clone();
        let spawned = std::thread::Builder::new()
            .name("morune-update-download".into())
            .spawn(move || {
                let result = fetch_installer(&release, &dir, &move |pct| {
                    let _ = progress.send(Message::Progress(pct));
                })
                .map_err(|e| e.to_string());
                let _ = tx.send(Message::Downloaded(result));
            });

        match spawned {
            Ok(_) => {
                self.phase = Phase::Downloading(0);
                self.pending = Some(rx);
            }
            Err(e) => self.phase = Phase::Failed(format!("Não foi possível baixar: {e}")),
        }
    }

    /// Recolhe o que a thread de rede produziu.
    ///
    /// Devolve `true` quando algo mudou, para que so entao a tela seja
    /// reespelhada -- este metodo roda no mesmo tique de 100 ms do backend.
    pub fn poll(&mut self) -> bool {
        let Some(rx) = self.pending.as_ref() else {
            return false;
        };

        let mut mudou = false;
        loop {
            match rx.try_recv() {
                Ok(Message::Progress(pct)) => {
                    let novo = Phase::Downloading(pct);
                    if self.phase != novo {
                        self.phase = novo;
                        mudou = true;
                    }
                }
                Ok(Message::Checked(Ok(Some(release)))) => {
                    tracing::info!(versao = %release.version, "atualizacao disponivel");
                    if self.automatica {
                        self.anuncio = Some(release.version.clone());
                    }
                    self.release = Some(*release);
                    self.phase = Phase::Available;
                    self.pending = None;
                    self.automatica = false;
                    return true;
                }
                Ok(Message::Checked(Ok(None))) => {
                    self.release = None;
                    // Uma verificacao que ninguem pediu nao escreve "você já
                    // está na versão mais recente" por cima da tela: quem nao
                    // perguntou nao esta esperando resposta.
                    if !self.automatica {
                        self.phase = Phase::UpToDate;
                    }
                    self.pending = None;
                    self.automatica = false;
                    return true;
                }
                Ok(Message::Checked(Err(e))) | Ok(Message::Downloaded(Err(e))) => {
                    tracing::warn!(erro = %e, "atualizacao falhou");
                    // Idem para o erro, e aqui pesa mais: sem internet, a
                    // verificacao automatica jogaria "Sem resposta do GitHub"
                    // na tela de quem so queria ouvir musica.
                    if !self.automatica {
                        self.phase = Phase::Failed(e);
                    }
                    self.pending = None;
                    self.automatica = false;
                    return true;
                }
                Ok(Message::Downloaded(Ok(path))) => {
                    tracing::info!(arquivo = %path.display(), "instalador pronto");
                    self.ready = Some(path);
                    self.phase = Phase::Ready;
                    self.pending = None;
                    return true;
                }
                Err(TryRecvError::Empty) => return mudou,
                // A thread terminou sem responder. Sem este ramo a tela ficaria
                // "verificando" para sempre.
                Err(TryRecvError::Disconnected) => {
                    self.pending = None;
                    if self.busy() {
                        self.phase = Phase::Failed("A verificação foi interrompida.".into());
                    }
                    return true;
                }
            }
        }
    }
}

/// Erros do caminho de atualizacao, ja com texto de tela.
#[derive(Debug, thiserror::Error)]
enum UpdateError {
    #[error("Sem resposta do GitHub. Verifique a conexão.")]
    Network(#[from] reqwest::Error),
    #[error("O GitHub respondeu {0}.")]
    Status(u16),
    #[error("Resposta do GitHub em formato inesperado.")]
    Format,
    #[error("Este lançamento não traz instalador para Windows.")]
    NoInstaller,
    #[error("O arquivo baixado não confere com o publicado. Nada foi executado.")]
    HashMismatch,
    #[error("O instalador é maior do que o esperado. Nada foi executado.")]
    TooLarge,
    #[error("Não foi possível gravar o download: {0}")]
    Io(#[from] std::io::Error),
}

/// Cliente HTTP do modulo.
///
/// `User-Agent` proprio porque a API do GitHub recusa requisicao sem ele. Sem
/// token: `/releases/latest` de repositorio publico e anonimo, e pedir uma
/// credencial so para verificar atualizacao seria um preco absurdo.
fn client(timeout: Duration) -> Result<reqwest::blocking::Client, UpdateError> {
    Ok(reqwest::blocking::Client::builder()
        .user_agent(concat!("morune/", env!("CARGO_PKG_VERSION")))
        .timeout(timeout)
        .build()?)
}

/// Consulta os lancamentos e decide se algum e mais novo que o instalado.
fn fetch_latest(current: &Version) -> Result<Option<Release>, UpdateError> {
    let response = client(CHECK_TIMEOUT)?
        .get(RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .send()?;

    let status = response.status();
    if !status.is_success() {
        return Err(UpdateError::Status(status.as_u16()));
    }

    let body: serde_json::Value = response.json()?;
    let releases = body.as_array().ok_or(UpdateError::Format)?;

    let Some((version, entry)) = newest(releases, current) else {
        return Ok(None);
    };

    let assets = entry
        .get("assets")
        .and_then(|v| v.as_array())
        .ok_or(UpdateError::Format)?;

    let by_suffix = |suffix: &str| {
        assets.iter().find(|a| {
            a.get("name")
                .and_then(|n| n.as_str())
                .is_some_and(|n| n.ends_with(suffix))
        })
    };

    // O nome do `.sha256` tambem termina em `.exe`, entao o instalador precisa
    // ser procurado pelo sufixo completo -- `.exe` sozinho acharia os dois, na
    // ordem em que o GitHub resolvesse devolver.
    let setup = by_suffix("-setup.exe").ok_or(UpdateError::NoInstaller)?;
    let sha = by_suffix("-setup.exe.sha256").ok_or(UpdateError::NoInstaller)?;

    let text = |value: &serde_json::Value, key: &str| {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .ok_or(UpdateError::Format)
    };

    Ok(Some(Release {
        notes: notes(entry.get("body").and_then(|v| v.as_str()).unwrap_or("")),
        setup_name: text(setup, "name")?,
        setup_url: text(setup, "browser_download_url")?,
        sha_url: text(sha, "browser_download_url")?,
        size: setup.get("size").and_then(|v| v.as_u64()).unwrap_or(0),
        version,
    }))
}

/// Escolhe, entre os lancamentos publicados, o mais novo que serve para quem
/// esta rodando `current`. `None` quando nenhum e mais novo.
///
/// **A regra de canal:** quem esta num pre-lancamento recebe pre-lancamentos;
/// quem esta numa versao final so recebe versoes finais. Ela resolve as duas
/// pontas de uma vez -- hoje todo lancamento do Morune e alpha, e sem
/// pre-lancamentos o botao nunca acharia nada; amanha, quando existir um
/// `v1.0.0`, ninguem que instalou a versao final vai ser empurrado para dentro
/// de um alpha por ter clicado em "Procurar atualizações".
///
/// Quem esta num alpha e recebe uma final continua sendo atendido: por semver
/// `1.0.0` e maior que `1.0.0-alpha.9`, entao ela ganha a comparacao.
fn newest<'a>(
    releases: &'a [serde_json::Value],
    current: &Version,
) -> Option<(Version, &'a serde_json::Value)> {
    releases
        .iter()
        .filter(|entry| {
            // Rascunho nao esta publicado: os arquivos dele nem sao baixaveis
            // sem credencial.
            !entry
                .get("draft")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .filter_map(|entry| {
            let tag = entry.get("tag_name")?.as_str()?;
            Some((Version::parse(tag)?, entry))
        })
        .filter(|(version, _)| current.is_prerelease() || !version.is_prerelease())
        .filter(|(version, _)| version > current)
        .max_by(|(a, _), (b, _)| a.cmp(b))
}

/// Quantas linhas das notas de versao a tela mostra.
const NOTES_LINES: usize = 12;

/// Reduz o corpo da release ao que cabe na tela.
///
/// O texto vem em Markdown e a caixa mostra texto puro: trazer um renderizador
/// de Markdown para um paragrafo seria caro demais. O que se faz aqui e tirar a
/// marcacao que, sem renderizador, apareceria como sujeira na tela -- os `#` de
/// titulo, os `**` de negrito, as crases, e a linha de HTML do badge "Made with
/// Slint" que toda release do projeto carrega no fim.
fn notes(body: &str) -> String {
    body.lines()
        .map(|line| line.trim_start_matches('#').trim())
        // Uma linha que comeca em `<` e HTML solto, nao texto para ler.
        .filter(|line| !line.is_empty() && !line.starts_with('<'))
        .map(|line| line.replace("**", "").replace('`', ""))
        .take(NOTES_LINES)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Baixa o instalador, confere o hash publicado e devolve o caminho local.
///
/// A ordem importa: o arquivo so ganha o nome definitivo depois de conferido.
/// Enquanto baixa ele e um `.part`, para que uma interrupcao no meio nunca
/// deixe no disco algo que pareca um instalador pronto para ser clicado.
fn fetch_installer(
    release: &Release,
    dir: &Path,
    progress: &dyn Fn(u8),
) -> Result<PathBuf, UpdateError> {
    std::fs::create_dir_all(dir)?;
    let client = client(DOWNLOAD_TIMEOUT)?;

    let esperado = expected_hash(&client, &release.sha_url)?;

    let destino = dir.join(&release.setup_name);
    let parcial = destino.with_extension("part");

    let mut response = client.get(&release.setup_url).send()?;
    let status = response.status();
    if !status.is_success() {
        return Err(UpdateError::Status(status.as_u16()));
    }

    let total = response.content_length().unwrap_or(release.size);
    if total > MAX_SETUP_BYTES {
        return Err(UpdateError::TooLarge);
    }

    let mut file = std::io::BufWriter::new(std::fs::File::create(&parcial)?);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    let mut lidos: u64 = 0;
    let mut ultimo_pct = u8::MAX;

    loop {
        let n = response.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        lidos += n as u64;
        // Um `Content-Length` mentiroso nao pode virar um disco cheio.
        if lidos > MAX_SETUP_BYTES {
            drop(file);
            let _ = std::fs::remove_file(&parcial);
            return Err(UpdateError::TooLarge);
        }

        hasher.update(&buffer[..n]);
        std::io::Write::write_all(&mut file, &buffer[..n])?;

        // Sem tamanho conhecido nao ha percentual: a barra fica onde estava em
        // vez de inventar um numero.
        if let Some(pct) = (lidos * 100).checked_div(total) {
            let pct = pct.min(100) as u8;
            // Um evento por ponto percentual: cem mensagens no total, e nao as
            // milhares que um envio por bloco produziria.
            if pct != ultimo_pct {
                ultimo_pct = pct;
                progress(pct);
            }
        }
    }

    std::io::Write::flush(&mut file)?;
    drop(file);

    let obtido = hex(&hasher.finalize());
    if obtido != esperado {
        // Apagar e o ponto: um arquivo que nao confere nao pode sobrar no disco
        // esperando alguem clicar nele pelo Explorer.
        let _ = std::fs::remove_file(&parcial);
        tracing::error!(esperado, obtido, "hash do instalador nao confere");
        return Err(UpdateError::HashMismatch);
    }

    // `rename` sobre arquivo existente falha no Windows, e uma tentativa
    // anterior da mesma versao pode ter deixado o destino no lugar.
    let _ = std::fs::remove_file(&destino);
    std::fs::rename(&parcial, &destino)?;
    Ok(destino)
}

/// Le o `.sha256` publicado ao lado do instalador.
fn expected_hash(client: &reqwest::blocking::Client, url: &str) -> Result<String, UpdateError> {
    let response = client.get(url).send()?;
    let status = response.status();
    if !status.is_success() {
        return Err(UpdateError::Status(status.as_u16()));
    }
    parse_sha256(&response.text()?).ok_or(UpdateError::Format)
}

/// Extrai o hash do formato `<hex>  <arquivo>`, o mesmo que `sha256sum` usa.
fn parse_sha256(text: &str) -> Option<String> {
    let token = text.split_whitespace().next()?;
    let hash = token.to_ascii_lowercase();
    let valido = hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit());
    valido.then_some(hash)
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::with_capacity(64), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// Vigia o executavel em disco enquanto o processo roda.
///
/// **O defeito que isto pega:** o Windows deixa renomear e apagar um `.exe` em
/// uso, e o processo continua vivo com o codigo que ja carregou. Um Morune que
/// abre junto com a sessao e fica dias aberto atravessa uma instalacao nova sem
/// perceber -- e quem esta na frente dele testa, o tempo todo, um binario que
/// nao existe mais. Aconteceu: o aplicativo rodava de uma pasta ja removida, com
/// o dono achando que estava na versao do dia.
///
/// Nao reinicia nada sozinho: derrubar um aplicativo que esta tocando musica
/// para trocar de binario e decisao de quem esta ouvindo, nao do relogio.
pub struct BinaryWatch {
    /// Caminho do proprio executavel. `None` quando o sistema nao o informa --
    /// ai nao ha o que vigiar, e o vigia simplesmente nao opina.
    path: Option<PathBuf>,
    /// Tamanho e data de modificacao vistos na abertura.
    original: Option<(u64, std::time::SystemTime)>,
    changed: bool,
    last: std::time::Instant,
}

impl BinaryWatch {
    /// Quanto tempo entre duas olhadas no disco.
    ///
    /// O tique da interface roda a cada 100 ms, e uma consulta de metadado a
    /// cada tique seria I/O constante para responder uma pergunta que muda uma
    /// vez por semana. Um minuto e cedo o bastante: o que se quer evitar e uma
    /// tarde inteira testando o binario errado.
    const INTERVALO: Duration = Duration::from_secs(60);

    pub fn new() -> Self {
        let path = std::env::current_exe().ok();
        let original = path.as_deref().and_then(marca);
        Self {
            path,
            original,
            changed: false,
            last: std::time::Instant::now(),
        }
    }

    /// `true` **no tique em que** o executavel passa a estar diferente.
    ///
    /// Avisa uma vez so: repetir o alerta a cada minuto viraria ruido, e a
    /// informacao continua disponivel em [`BinaryWatch::changed`].
    pub fn poll(&mut self) -> bool {
        if self.changed || self.original.is_none() || self.last.elapsed() < Self::INTERVALO {
            return false;
        }
        self.last = std::time::Instant::now();

        let Some(path) = self.path.as_deref() else {
            return false;
        };

        // Apagado conta como mudado: e o caso da instalacao removida por baixo.
        let agora = marca(path);
        if agora == self.original {
            return false;
        }

        tracing::warn!(
            arquivo = %path.display(),
            existe = agora.is_some(),
            "o executavel mudou em disco desde que este processo abriu"
        );
        self.changed = true;
        true
    }

    /// O executavel ja mudou em algum momento desta sessao.
    pub fn changed(&self) -> bool {
        self.changed
    }
}

impl Default for BinaryWatch {
    fn default() -> Self {
        Self::new()
    }
}

/// Tamanho e data de modificacao de um arquivo, quando ele existe.
fn marca(path: &Path) -> Option<(u64, std::time::SystemTime)> {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.len(), meta.modified().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versao_le_os_formatos_que_o_projeto_publica() {
        assert_eq!(Version::parse("0.1.0").unwrap().to_string(), "0.1.0");
        assert_eq!(Version::parse("v1.2.3").unwrap().to_string(), "1.2.3");
        assert_eq!(
            Version::parse("v0.2.0-alpha.1").unwrap().to_string(),
            "0.2.0-alpha.1"
        );
    }

    #[test]
    fn versao_recusa_lixo() {
        for entrada in ["", "v", "1.2", "1.2.3.4", "abc", "v1.x.0", "1.2.3-"] {
            assert!(Version::parse(entrada).is_none(), "aceitou {entrada:?}");
        }
    }

    #[test]
    fn versao_maior_ganha_campo_a_campo() {
        let v = |s| Version::parse(s).unwrap();
        assert!(v("0.2.0") > v("0.1.0"));
        assert!(v("0.1.1") > v("0.1.0"));
        assert!(v("1.0.0") > v("0.99.99"));
        // O caso que uma comparacao textual erraria.
        assert!(v("0.10.0") > v("0.9.0"));
    }

    #[test]
    fn pre_lancamento_vem_antes_do_final() {
        let v = |s| Version::parse(s).unwrap();
        assert!(v("0.2.0") > v("0.2.0-alpha.1"));
        assert!(v("0.2.0-alpha.1") > v("0.1.0"));
    }

    /// O caso que uma comparacao de texto erraria, e que este projeto vai
    /// alcancar: hoje esta em `alpha.8`.
    #[test]
    fn alpha_10_vem_depois_de_alpha_9() {
        let v = |s| Version::parse(s).unwrap();
        assert!(v("0.1.0-alpha.10") > v("0.1.0-alpha.9"));
        assert!(v("0.1.0-alpha.9") > v("0.1.0-alpha.8"));
    }

    #[test]
    fn identificador_numerico_perde_para_o_alfanumerico() {
        // Regra 11.4.3 do semver, na ordem que ela produz.
        let v = |s| Version::parse(s).unwrap();
        assert!(v("1.0.0-alpha.beta") > v("1.0.0-alpha.1"));
        // Menos identificadores vem antes, quando o prefixo empata.
        assert!(v("1.0.0-alpha.1") > v("1.0.0-alpha"));
    }

    /// O caso que o botao decide: instalado igual ao publicado nao e novidade.
    #[test]
    fn versao_igual_nao_e_atualizacao() {
        let v = |s| Version::parse(s).unwrap();
        assert!(v("0.1.0") <= v("0.1.0"));
    }

    /// A versao do proprio binario precisa ser legivel: e a base de toda
    /// comparacao, e um valor invalido faria o botao mentir.
    #[test]
    fn a_versao_do_binario_e_legivel() {
        let tag = env!("MORUNE_RELEASE");
        assert!(
            Version::parse(tag).is_some(),
            "MORUNE_RELEASE={tag:?} nao e uma versao valida"
        );
        // Num build local a tag traz `-dev.<hash>`; num build de release, o
        // sufixo de pre-lancamento da tag publicada. Nos dois casos a parte
        // numerica bate, e e o `release.yml` que garante isso na publicacao.
        assert!(tag.starts_with(&format!("v{}", morune_core::VERSION)));
    }

    /// O defeito que isto pega: com a tag caindo em `v0.1.0`, todo build local
    /// se anunciava como uma versao **final** -- por semver, mais nova que
    /// qualquer alpha publicado. A verificacao nunca achava nada e a tela dizia
    /// "você já está na versão mais recente" com lancamentos novos no ar.
    #[test]
    fn build_local_e_pre_lancamento_e_nao_versao_final() {
        if env!("MORUNE_RELEASE_SOURCE") != "local" {
            // Compilado pelo workflow: a tag e a publicada, e uma release
            // final e legitima. Nada a exigir aqui.
            return;
        }

        let tag = env!("MORUNE_RELEASE");
        let versao = Version::parse(tag).unwrap();
        assert!(
            versao.is_dev(),
            "MORUNE_RELEASE={tag:?} devia ser um build local marcado como -dev"
        );
        assert!(
            versao.is_prerelease(),
            "MORUNE_RELEASE={tag:?} se anuncia como versao final"
        );
    }

    #[test]
    fn dev_e_reconhecido_e_alpha_nao() {
        assert!(Version::parse("v0.1.0-dev.268113d").unwrap().is_dev());
        assert!(Version::parse("v0.1.0-dev").unwrap().is_dev());
        assert!(!Version::parse("v0.1.0-alpha.9").unwrap().is_dev());
        assert!(!Version::parse("v0.1.0").unwrap().is_dev());
        // Nao basta comecar com as tres letras: `develop` seria outro rotulo.
        assert!(!Version::parse("v0.1.0-developer").unwrap().is_dev());
    }

    /// Um build local sai de commits que os alphas publicados ainda nao tem,
    /// entao ele e mais novo -- e por isso nenhum alpha lhe e oferecido.
    #[test]
    fn build_local_ordena_acima_dos_alphas() {
        let dev = Version::parse("v0.1.0-dev.268113d").unwrap();
        let alpha = Version::parse("v0.1.0-alpha.9").unwrap();
        assert!(dev > alpha);

        let lista = [
            entrada("v0.1.0-alpha.9", false),
            entrada("v0.1.0-alpha.8", false),
        ];
        assert!(
            newest(&lista, &dev).is_none(),
            "um alpha publicado seria um downgrade por cima do build local"
        );
    }

    /// Executavel apagado por baixo do processo -- a instalacao removida com o
    /// Morune ainda rodando -- precisa contar como "mudou", e nao passar batido
    /// por falta de metadado.
    #[test]
    fn arquivo_que_nao_existe_nao_tem_marca() {
        let inexistente = std::env::temp_dir().join("morune-nao-existe-mesmo.exe");
        assert!(marca(&inexistente).is_none());
    }

    /// Constroi a resposta do GitHub reduzida ao que `newest` le.
    fn entrada(tag: &str, draft: bool) -> serde_json::Value {
        serde_json::json!({ "tag_name": tag, "draft": draft })
    }

    #[test]
    fn escolhe_a_maior_versao_e_nao_a_primeira_da_lista() {
        // Fora de ordem de proposito: republicar uma tag antiga reordena a
        // lista que o GitHub devolve.
        let lista = [
            entrada("v0.1.0", false),
            entrada("v0.3.0", false),
            entrada("v0.2.0", false),
        ];
        let atual = Version::parse("0.1.0").unwrap();
        let (versao, _) = newest(&lista, &atual).unwrap();
        assert_eq!(versao.to_string(), "0.3.0");
    }

    #[test]
    fn nada_mais_novo_nao_e_atualizacao() {
        let lista = [entrada("v0.1.0", false), entrada("v0.0.9", false)];
        let atual = Version::parse("0.1.0").unwrap();
        assert!(newest(&lista, &atual).is_none());
    }

    #[test]
    fn rascunho_e_ignorado() {
        let lista = [entrada("v0.9.0", true), entrada("v0.2.0", false)];
        let atual = Version::parse("0.1.0").unwrap();
        let (versao, _) = newest(&lista, &atual).unwrap();
        assert_eq!(versao.to_string(), "0.2.0");
    }

    #[test]
    fn tag_ilegivel_nao_derruba_a_verificacao() {
        let lista = [entrada("nightly", false), entrada("v0.2.0", false)];
        let atual = Version::parse("0.1.0").unwrap();
        let (versao, _) = newest(&lista, &atual).unwrap();
        assert_eq!(versao.to_string(), "0.2.0");
    }

    /// Quem instalou uma versao final nao e empurrado para dentro de um alpha.
    #[test]
    fn versao_final_nao_recebe_pre_lancamento() {
        let lista = [entrada("v0.2.0-alpha.1", false)];
        let atual = Version::parse("0.1.0").unwrap();
        assert!(newest(&lista, &atual).is_none());
    }

    /// O caso de hoje: todo lancamento do Morune e um pre-lancamento, e quem
    /// esta num alpha precisa enxergar o alpha seguinte.
    #[test]
    fn quem_esta_num_alpha_recebe_o_alpha_seguinte() {
        let lista = [
            entrada("v0.1.0-alpha.5", false),
            entrada("v0.1.0-alpha.8", false),
            entrada("v0.1.0-alpha.7", false),
        ];
        let atual = Version::parse("v0.1.0-alpha.6").unwrap();
        let (versao, _) = newest(&lista, &atual).unwrap();
        assert_eq!(versao.to_string(), "0.1.0-alpha.8");
    }

    /// E quando a final finalmente sai, ela ganha do alpha.
    #[test]
    fn quem_esta_num_alpha_recebe_a_versao_final() {
        let lista = [entrada("v0.1.0-alpha.9", false), entrada("v0.1.0", false)];
        let atual = Version::parse("v0.1.0-alpha.8").unwrap();
        let (versao, _) = newest(&lista, &atual).unwrap();
        assert_eq!(versao.to_string(), "0.1.0");
    }

    #[test]
    fn hash_e_lido_do_formato_do_sha256sum() {
        let esperado = "ab12".repeat(16);
        let linha = format!("{esperado}  Morune-0.1.0-setup.exe\n");
        assert_eq!(parse_sha256(&linha).unwrap(), esperado);
    }

    #[test]
    fn hash_maiusculo_e_aceito_e_normalizado() {
        let linha = format!("{}  x.exe", "AB".repeat(32));
        assert_eq!(parse_sha256(&linha).unwrap(), "ab".repeat(32));
    }

    #[test]
    fn hash_invalido_e_recusado() {
        for entrada in ["", "naoehex", &"zz".repeat(32), &"ab".repeat(20)] {
            assert!(parse_sha256(entrada).is_none(), "aceitou {entrada:?}");
        }
    }

    #[test]
    fn hex_formata_com_dois_digitos_por_byte() {
        assert_eq!(hex(&[0x00, 0x0f, 0xff]), "000fff");
    }

    #[test]
    fn notas_perdem_marcacao_de_titulo_e_linhas_vazias() {
        let corpo = "## Novidades\n\n- um\n\n- dois\n";
        assert_eq!(notes(corpo), "Novidades\n- um\n- dois");
    }

    /// O corpo real das releases do Morune: negrito, crase e o badge do Slint.
    /// Sem renderizador de Markdown, tudo isso chegaria cru a tela.
    #[test]
    fn notas_perdem_negrito_crase_e_html() {
        let corpo = "**Antes de instalar**\n\
                     - confira o arquivo `.sha256` publicado;\n\
                     <a href=\"https://slint.dev\"><img alt=\"Made with Slint\"></a>\n";
        assert_eq!(
            notes(corpo),
            "Antes de instalar\n- confira o arquivo .sha256 publicado;"
        );
    }

    #[test]
    fn notas_sao_cortadas_no_limite_da_tela() {
        let corpo = (0..40).map(|i| format!("linha {i}")).collect::<Vec<_>>();
        assert_eq!(notes(&corpo.join("\n")).lines().count(), NOTES_LINES);
    }

    /// Vai a rede de verdade: consulta os lancamentos publicados e baixa o
    /// instalador do mais novo, conferindo o hash.
    ///
    /// `#[ignore]` porque depende do GitHub estar no ar e baixa ~10 MB -- nao
    /// pode entrar na suite que roda a cada push. E o teste que prova que o
    /// caminho inteiro funciona contra a API real, entao vale roda-lo a mao
    /// quando algo neste modulo mudar:
    ///
    /// ```text
    /// cargo test -p morune-app -- --ignored --nocapture atualizacao_de_verdade
    /// ```
    #[test]
    #[ignore = "usa a rede e baixa o instalador"]
    fn atualizacao_de_verdade() {
        // Uma versao antiga de proposito, para que haja o que encontrar.
        let antiga = Version::parse("v0.1.0-alpha.1").unwrap();
        let release = fetch_latest(&antiga)
            .expect("consulta ao GitHub")
            .expect("ha lancamento mais novo que o alpha.1");
        println!("encontrado: {} ({})", release.version, release.setup_name);

        let dir = std::env::temp_dir().join("morune-teste-update");
        let arquivo = fetch_installer(&release, &dir, &|pct| {
            if pct % 25 == 0 {
                println!("  {pct}%");
            }
        })
        .expect("download e verificacao do hash");

        assert!(arquivo.is_file(), "instalador nao ficou no disco");
        println!("verificado: {}", arquivo.display());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Nada de rede acontece antes de alguem clicar.
    #[test]
    fn comeca_parado() {
        let u = Updater::new(std::env::temp_dir().join("morune-test-updates"));
        assert_eq!(*u.phase(), Phase::Idle);
        assert!(!u.busy());
        assert!(u.installer().is_none());
        assert!(u.release().is_none());
    }
}
