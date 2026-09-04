//! Painel de reproducao do Windows (SMTC).
//!
//! E a coisa que aparece no canto superior esquerdo quando se mexe no volume,
//! na tela de bloqueio e no Ctrl+Alt+Del: capa, titulo, artista e os botoes de
//! anterior, tocar/pausar e proxima. O Windows monta esse painel a partir do
//! `SystemMediaTransportControls` de cada aplicativo.
//!
//! **Por que ele nao vinha de graca.** As teclas de midia do teclado ja
//! funcionavam por `WM_APPCOMMAND` (ver [`crate::taskbar`]), e e facil concluir
//! que o painel viria junto. Nao vem: `WM_APPCOMMAND` chega quando a janela
//! esta em primeiro plano, e o painel do sistema so existe para quem se
//! registra aqui. Sem este modulo, mexer no volume durante um jogo mostrava um
//! painel vazio -- ou o do navegador aberto atras.
//!
//! **Registrar tambem muda o caminho das teclas.** Com um SMTC ativo, o Windows
//! passa a entregar as teclas de midia tambem por evento daqui, inclusive com a
//! janela escondida na bandeja. Os dois caminhos coexistem, e por isso os
//! comandos sao explicitos (`Play` e `Pause`, e nao so alternar): o painel do
//! sistema tem botoes separados, e mandar alternar em cima de um `Play`
//! pausaria justamente o que o usuario acabou de mandar tocar.
//!
//! A unica fronteira insegura e o `HWND` vindo do `raw_window_handle`; o resto
//! e WinRT tipado pelo crate `windows`.

use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::core::HSTRING;
use windows::Foundation::{TypedEventHandler, Uri};
use windows::Media::{
    MediaPlaybackStatus, MediaPlaybackType, SystemMediaTransportControls,
    SystemMediaTransportControlsButton, SystemMediaTransportControlsButtonPressedEventArgs,
    SystemMediaTransportControlsDisplayUpdater,
};
use windows::Storage::Streams::RandomAccessStreamReference;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::WinRT::ISystemMediaTransportControlsInterop;

/// O que o painel do sistema pediu.
///
/// Separado de [`crate::taskbar::TaskbarCommand`] porque o painel distingue
/// tocar de pausar, e os botoes da barra de tarefas nao.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaCommand {
    Play,
    Pause,
    TogglePlay,
    Next,
    Previous,
}

/// O que o Morune quer que o painel mostre.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MediaStatus {
    pub title: String,
    pub artist: String,
    pub album: String,
    /// Capa ja baixada, quando o cache em disco a tem.
    pub cover: Option<PathBuf>,
    pub playing: bool,
}

/// O que o painel esta mostrando agora.
///
/// Comparado antes de escrever: `Update()` atravessa a fronteira do processo
/// ate o servico de midia do Windows, e o laco da interface roda a cada 150 ms.
/// Reescrever a mesma faixa dez vezes por segundo e exatamente o gasto
/// invisivel que o orcamento de desempenho do projeto proibe -- ja aconteceu
/// uma vez com o menu da bandeja, e custou 0,22% de um nucleo em repouso.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Mostrado {
    faixa: Option<MediaStatus>,
}

pub struct MediaControls {
    controls: SystemMediaTransportControls,
    updater: SystemMediaTransportControlsDisplayUpdater,
    commands: Receiver<MediaCommand>,
    /// Token da inscricao no `ButtonPressed`, guardado so para documentar que
    /// ela existe: quem a cancela e o `Drop` do proprio SMTC.
    _token: i64,
    mostrado: Mostrado,
}

impl MediaControls {
    /// Registra o Morune no painel de midia do Windows.
    ///
    /// Precisa do `HWND`: o SMTC de um aplicativo de area de trabalho e obtido
    /// **por janela**, e nao por processo. Dai a chamada passar pela interface
    /// de interop, e nao pelo construtor WinRT que valeria num aplicativo
    /// empacotado.
    pub fn new(window: &slint::Window) -> Result<Self, MediaError> {
        let handle = window.window_handle();
        let RawWindowHandle::Win32(raw) = handle.window_handle()?.as_raw() else {
            return Err(MediaError::UnsupportedWindow);
        };
        let hwnd = HWND(raw.hwnd.get() as *mut c_void);

        let interop = windows::core::factory::<
            SystemMediaTransportControls,
            ISystemMediaTransportControlsInterop,
        >()?;
        // SAFETY: HWND da janela viva do aplicativo. O objeto devolvido tem
        // contagem de referencia propria e e liberado pelo `Drop` do WinRT.
        let controls: SystemMediaTransportControls = unsafe { interop.GetForWindow(hwnd)? };

        controls.SetIsEnabled(true)?;
        controls.SetIsPlayEnabled(true)?;
        controls.SetIsPauseEnabled(true)?;
        controls.SetIsNextEnabled(true)?;
        controls.SetIsPreviousEnabled(true)?;
        // Parar nao existe no Morune -- fechar vai para a bandeja e a musica
        // continua. O painel nao deve oferecer um botao que nao faz nada.
        controls.SetIsStopEnabled(false)?;

        let (sender, commands) = mpsc::channel();
        let token = controls.ButtonPressed(&TypedEventHandler::<
            SystemMediaTransportControls,
            SystemMediaTransportControlsButtonPressedEventArgs,
        >::new(move |_, args| {
            // Isto **nao** roda na thread da interface. Dai o canal: o
            // `AppState` vive num `RefCell` de thread unica, e mexer nele daqui
            // seria corrida garantida.
            if let Some(args) = args.as_ref() {
                if let Some(comando) = traduzir(args.Button()?) {
                    let _ = sender.send(comando);
                }
            }
            Ok(())
        }))?;

        let updater = controls.DisplayUpdater()?;
        updater.SetType(MediaPlaybackType::Music)?;

        Ok(Self {
            controls,
            updater,
            commands,
            _token: token,
            mostrado: Mostrado::default(),
        })
    }

    /// Comandos acumulados desde a ultima leitura.
    pub fn poll(&mut self) -> Vec<MediaCommand> {
        self.commands.try_iter().collect()
    }

    /// Manda para o painel o que esta tocando. Silencioso quando nada mudou.
    pub fn update(&mut self, status: Option<&MediaStatus>) -> Result<(), MediaError> {
        let novo = Mostrado {
            faixa: status.cloned(),
        };
        if novo == self.mostrado {
            return Ok(());
        }

        let Some(faixa) = novo.faixa.as_ref() else {
            self.controls
                .SetPlaybackStatus(MediaPlaybackStatus::Closed)?;
            self.updater.ClearAll()?;
            self.updater.Update()?;
            self.mostrado = novo;
            return Ok(());
        };

        self.controls.SetPlaybackStatus(if faixa.playing {
            MediaPlaybackStatus::Playing
        } else {
            MediaPlaybackStatus::Paused
        })?;

        // `SetType` de novo: um `ClearAll` anterior o zera, e sem tipo o painel
        // nao mostra campo nenhum.
        self.updater.SetType(MediaPlaybackType::Music)?;
        let musica = self.updater.MusicProperties()?;
        musica.SetTitle(&HSTRING::from(faixa.title.as_str()))?;
        musica.SetArtist(&HSTRING::from(faixa.artist.as_str()))?;
        musica.SetAlbumTitle(&HSTRING::from(faixa.album.as_str()))?;

        match faixa.cover.as_deref().and_then(stream_de_arquivo) {
            Some(fluxo) => self.updater.SetThumbnail(&fluxo)?,
            // A capa ainda nao chegou ao cache. Sem miniatura o painel usa o
            // icone do aplicativo, que e melhor que manter a capa da faixa
            // anterior -- e a proxima passagem por aqui a coloca.
            None => self.updater.SetThumbnail(None)?,
        }

        self.updater.Update()?;
        self.mostrado = novo;
        Ok(())
    }
}

impl Drop for MediaControls {
    fn drop(&mut self) {
        // Sem isto o painel continuaria anunciando a ultima faixa depois de o
        // Morune sair, ate o Windows perceber que o processo morreu.
        let _ = self.controls.SetPlaybackStatus(MediaPlaybackStatus::Closed);
        let _ = self.controls.SetIsEnabled(false);
    }
}

/// Referencia de fluxo para uma capa que ja esta em disco.
///
/// Vai por URI `file:`, e nao por `StorageFile`: obter um `StorageFile` e
/// operacao assincrona, e esperar por ela na thread da interface -- que e uma
/// STA -- e o caminho classico para travar o aplicativo inteiro.
fn stream_de_arquivo(caminho: &Path) -> Option<RandomAccessStreamReference> {
    let texto = caminho.to_str()?;
    let uri = Uri::CreateUri(&HSTRING::from(format!(
        "file:///{}",
        texto.replace('\\', "/")
    )))
    .ok()?;
    RandomAccessStreamReference::CreateFromUri(&uri).ok()
}

/// Traduz o botao do painel. `None` para o que o Morune nao faz.
fn traduzir(botao: SystemMediaTransportControlsButton) -> Option<MediaCommand> {
    match botao {
        SystemMediaTransportControlsButton::Play => Some(MediaCommand::Play),
        SystemMediaTransportControlsButton::Pause => Some(MediaCommand::Pause),
        SystemMediaTransportControlsButton::Next => Some(MediaCommand::Next),
        SystemMediaTransportControlsButton::Previous => Some(MediaCommand::Previous),
        // O teclado manda isto quando a tecla e a unica de tocar/pausar.
        SystemMediaTransportControlsButton::Stop => Some(MediaCommand::TogglePlay),
        // Gravar, avancar rapido, rebobinar e canal: o Morune nao faz nenhum
        // deles, e ignorar e melhor que aproximar para o comando errado.
        _ => None,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("a janela nao e do Windows")]
    UnsupportedWindow,
    #[error("nao foi possivel obter o handle da janela: {0}")]
    WindowHandle(#[from] raw_window_handle::HandleError),
    #[error("{0}")]
    Windows(#[from] windows::core::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_do_painel_vira_alternar() {
        assert_eq!(
            traduzir(SystemMediaTransportControlsButton::Play),
            Some(MediaCommand::Play)
        );
        assert_eq!(
            traduzir(SystemMediaTransportControlsButton::Pause),
            Some(MediaCommand::Pause)
        );
        assert_eq!(traduzir(SystemMediaTransportControlsButton::Record), None);
    }

    /// A URI da capa precisa sair no formato que o WinRT aceita: barras normais
    /// e tres barras depois do esquema. Um caminho do Windows com contrabarra
    /// nao vira URI valida.
    #[test]
    fn caminho_do_windows_vira_uri_de_arquivo() {
        let uri = Uri::CreateUri(&HSTRING::from(format!(
            "file:///{}",
            r"C:\Users\x\capa.jpg".replace('\\', "/")
        )))
        .expect("uri valida");
        assert_eq!(
            uri.ToString().unwrap().to_string(),
            "file:///C:/Users/x/capa.jpg"
        );
    }
}
