//! Recarga a quente de temas.
//!
//! Fica atras da feature `hot-reload` porque um observador de sistema de
//! arquivos custa uma thread e alguns handles do sistema -- barato, mas nao
//! gratis, e a maioria dos usuarios nunca edita um tema.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecursiveMode, Watcher};

/// Janela de agrupamento de eventos.
///
/// Editores gravam um arquivo em varias operacoes (truncar, escrever, renomear);
/// sem agrupar, salvar uma vez dispararia tres recargas.
const DEBOUNCE: Duration = Duration::from_millis(180);

/// Observador de uma pasta de tema.
///
/// Chama o callback com o caminho da pasta sempre que algo relevante muda.
/// O callback roda numa thread propria: quem recebe deve apenas sinalizar a
/// interface, nunca fazer trabalho pesado ali.
pub struct ThemeWatcher {
    _watcher: notify::RecommendedWatcher,
    _thread: std::thread::JoinHandle<()>,
}

impl std::fmt::Debug for ThemeWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThemeWatcher").finish_non_exhaustive()
    }
}

impl ThemeWatcher {
    /// Observa `dir` recursivamente.
    ///
    /// O observador para quando o valor retornado e descartado.
    pub fn new<F>(dir: &Path, mut on_change: F) -> notify::Result<Self>
    where
        F: FnMut(PathBuf) + Send + 'static,
    {
        let (tx, rx) = mpsc::channel::<Event>();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        })?;
        watcher.watch(dir, RecursiveMode::Recursive)?;

        let watched = dir.to_path_buf();
        let thread = std::thread::Builder::new()
            .name("morune-theme-watch".into())
            .spawn(move || {
                let mut pending: Option<Instant> = None;
                loop {
                    // Espera curta o bastante para fechar a janela de debounce
                    // no tempo certo, longa o bastante para a thread ficar
                    // ociosa de verdade entre edicoes.
                    match rx.recv_timeout(DEBOUNCE) {
                        Ok(event) => {
                            if is_relevant(&event) {
                                pending = Some(Instant::now());
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        Err(mpsc::RecvTimeoutError::Disconnected) => return,
                    }

                    if pending.is_some_and(|t| t.elapsed() >= DEBOUNCE) {
                        pending = None;
                        on_change(watched.clone());
                    }
                }
            })
            .map_err(notify::Error::io)?;

        Ok(Self {
            _watcher: watcher,
            _thread: thread,
        })
    }
}

/// Ignora eventos de acesso e de arquivos que nao fazem parte do tema.
fn is_relevant(event: &Event) -> bool {
    if matches!(event.kind, EventKind::Access(_) | EventKind::Other) {
        return false;
    }
    event.paths.iter().any(|p| {
        // Arquivos temporarios de editor (`.swp`, `theme.toml~`, `.tmp1234`)
        // aparecem e somem; recarregar neles causaria piscadas na interface.
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') || name.ends_with('~') || name.ends_with(".tmp") {
            return false;
        }
        matches!(
            p.extension().and_then(|e| e.to_str()),
            Some("toml") | Some("png") | Some("jpg") | Some("svg") | Some("ttf") | Some("otf")
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{AccessKind, CreateKind, ModifyKind};

    fn event(kind: EventKind, path: &str) -> Event {
        Event {
            kind,
            paths: vec![PathBuf::from(path)],
            attrs: Default::default(),
        }
    }

    #[test]
    fn theme_files_are_relevant() {
        assert!(is_relevant(&event(
            EventKind::Modify(ModifyKind::Any),
            "C:/t/midnight/theme.toml"
        )));
        assert!(is_relevant(&event(
            EventKind::Create(CreateKind::File),
            "C:/t/midnight/assets/cover.png"
        )));
    }

    #[test]
    fn editor_temporaries_are_ignored() {
        for p in [
            "C:/t/midnight/theme.toml~",
            "C:/t/midnight/.theme.toml.swp",
            "C:/t/midnight/theme.toml.tmp",
        ] {
            assert!(
                !is_relevant(&event(EventKind::Modify(ModifyKind::Any), p)),
                "{p}"
            );
        }
    }

    #[test]
    fn unrelated_files_are_ignored() {
        assert!(!is_relevant(&event(
            EventKind::Modify(ModifyKind::Any),
            "C:/t/midnight/notes.md"
        )));
        assert!(!is_relevant(&event(
            EventKind::Modify(ModifyKind::Any),
            "C:/t/midnight/x.exe"
        )));
    }

    #[test]
    fn access_events_never_trigger_a_reload() {
        assert!(!is_relevant(&event(
            EventKind::Access(AccessKind::Read),
            "C:/t/midnight/theme.toml"
        )));
    }
}
