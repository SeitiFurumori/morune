//! Registro em arquivo (morune.log), com o mesmo teto e rotacao do app em Slint.

const LOG_MAX_BYTES: u64 = 4 * 1024 * 1024;

/// Liga o log em arquivo, e tambem no console quando ele existe.
///
/// **Por que arquivo e obrigatorio:** o build de release e
/// `windows_subsystem = "windows"`, ou seja, roda sem console nenhum. Tudo que
/// o `tracing` escrevia em `stdout` era descartado em silencio -- e um usuario
/// que ve "Sem conexão com o Spotify" nao tinha como saber o motivo, nem tinha
/// o que anexar a um relato de defeito. O caminho e
/// [`AppPaths::log_file`], que ja existia e nunca era usado.
///
/// Escrever e sincrono, sob trava. No nivel padrao (`info`) sao poucas linhas
/// por sessao; `MORUNE_LOG=debug` custa I/O e e opcional, para quem esta
/// investigando.
/// Manda todo panic para o log antes do processo morrer.
///
/// **Por que existe:** o release e `panic = "abort"` e
/// `windows_subsystem = "windows"`. Sem console, a mensagem que o Rust escreve
/// em `stderr` some, e o processo desaparece sem deixar uma linha sequer -- foi
/// exatamente o que aconteceu numa queda relatada e nao houve o que investigar.
/// O gancho roda antes do abort, entao a mensagem chega ao arquivo.
///
/// Precisa ser instalado depois de `init_logging`: antes dele nao ha para onde
/// escrever.
pub fn log_panics() {
    let anterior = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let local = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "local desconhecido".into());
        // O `Display` do `PanicHookInfo` ja traz a mensagem e a localizacao no
        // formato que o Rust usa no console; registrar os dois separados torna
        // o log pesquisavel por arquivo.
        tracing::error!(
            local = %local,
            thread = ?std::thread::current().name(),
            "panic: {info}"
        );
        anterior(info);
    }));
}

pub fn init_logging(paths: &morune_storage::AppPaths) {
    use tracing_subscriber::filter::EnvFilter;

    let filter = EnvFilter::try_from_env("MORUNE_LOG").unwrap_or_else(|_| EnvFilter::new("info"));
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact();

    let (file, aviso) = open_log_file(paths);
    match file {
        // Sem cor: o arquivo e lido em editor de texto, e o codigo de escape
        // apareceria como lixo no meio da mensagem.
        Some(file) => builder
            .with_ansi(false)
            .with_writer(std::sync::Mutex::new(file))
            .init(),
        None => builder.init(),
    }

    // Emitido depois de `init` de proposito: e a primeira linha do arquivo
    // alternativo, e explica por que o principal ficou para tras.
    if let Some(aviso) = aviso {
        tracing::warn!("{aviso}");
    }
}

/// Abre o log para acrescimo, rodando o anterior quando passa do teto.
///
/// Falhar aqui **nao** pode impedir o aplicativo de abrir. Mas tambem nao pode
/// passar despercebido: em release nao existe console, entao um `None` aqui
/// apaga a sessao inteira do registro -- ela roda, faz tudo, e nao deixa uma
/// linha. Ja aconteceu, e transformou um travamento numa investigacao sem
/// nenhuma evidencia.
///
/// Por isso, quando o caminho principal nao abre -- normalmente porque outro
/// programa esta com ele travado, um antivirus ou um indexador --, a sessao vai
/// para um arquivo proprio com o numero do processo no nome. O segundo valor
/// devolvido e o aviso a registrar assim que o log existir.
fn open_log_file(paths: &morune_storage::AppPaths) -> (Option<std::fs::File>, Option<String>) {
    let path = paths.log_file();

    if std::fs::metadata(&path).is_ok_and(|m| m.len() >= LOG_MAX_BYTES) {
        let _ = std::fs::rename(&path, path.with_extension("log.old"));
    }

    let abrir = |path: &std::path::Path| {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
    };

    let erro = match abrir(&path) {
        Ok(file) => return (Some(file), None),
        Err(e) => e,
    };

    let alternativo = path.with_file_name(format!("morune-{}.log", std::process::id()));
    match abrir(&alternativo) {
        Ok(file) => (
            Some(file),
            Some(format!(
                "{} indisponivel ({erro}); esta sessao esta sendo registrada aqui",
                path.display()
            )),
        ),
        // Nem o alternativo abriu: disco cheio ou pasta sem permissao. Nao ha
        // para onde escrever o aviso, e a unica escolha que resta e seguir.
        Err(_) => (None, None),
    }
}
