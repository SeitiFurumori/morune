//! Icone de bandeja e continuidade em segundo plano.
//!
//! O comportamento esperado e o do Discord: fechar a janela **nao** encerra o
//! aplicativo. A janela some, a musica continua, e o icone na bandeja e o jeito
//! de trazer tudo de volta ou de sair de verdade.
//!
//! Duas consequencias de projeto vem disso:
//!
//! - o laco de eventos nao pode terminar quando a ultima janela fecha, entao o
//!   aplicativo usa [`slint::run_event_loop_until_quit`] em vez de `Window::run`;
//! - sair precisa ser sempre alcancavel. Se a bandeja falhar ao ser criada,
//!   fechar a janela volta a encerrar o aplicativo -- caso contrario o processo
//!   ficaria vivo sem nenhuma forma visivel de mata-lo.

use std::cell::RefCell;
use std::time::Duration;

use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

/// O que o usuario pediu pela bandeja.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    /// Trazer a janela de volta.
    Show,
    TogglePlay,
    Next,
    Previous,
    /// Encerrar o aplicativo de verdade.
    Quit,
}

/// Falha ao criar a bandeja.
///
/// As duas bibliotecas envolvidas (icone e menu) tem tipos de erro proprios que
/// nao se convertem entre si; unifica-los aqui deixa o chamador com uma decisao
/// so: ha bandeja ou nao ha.
#[derive(Debug, thiserror::Error)]
pub enum TrayError {
    #[error("menu da bandeja: {0}")]
    Menu(#[from] tray_icon::menu::Error),
    #[error("icone da bandeja: {0}")]
    Icon(#[from] tray_icon::Error),
}

/// Icone de bandeja ativo.
///
/// A bandeja para de existir quando este valor e descartado, por isso ele
/// precisa viver enquanto o aplicativo viver.
pub struct Tray {
    _icon: TrayIcon,
    id_show: tray_icon::menu::MenuId,
    id_toggle: tray_icon::menu::MenuId,
    id_next: tray_icon::menu::MenuId,
    id_previous: tray_icon::menu::MenuId,
    id_quit: tray_icon::menu::MenuId,
    item_toggle: MenuItem,
    item_now: MenuItem,
    /// Ultimo estado escrito no menu.
    ///
    /// Sem isto, `update` faria meia duzia de chamadas Win32 a cada leitura e o
    /// custo em repouso deixaria de ser zero -- foi medido: 0,22% de um nucleo
    /// contra 0,00% com a comparacao.
    last_shown: RefCell<(Option<String>, bool)>,
}

impl std::fmt::Debug for Tray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tray").finish_non_exhaustive()
    }
}

/// Intervalo de leitura dos eventos da bandeja.
///
/// A biblioteca entrega eventos por canal, nao por callback no laco do Slint.
/// 150 ms e imperceptivel para um clique de menu e mantem o custo em repouso
/// proximo de zero -- o teste de CPU ociosa nao pode regredir por causa disto.
pub const POLL_INTERVAL: Duration = Duration::from_millis(150);

impl Tray {
    /// Cria o icone de bandeja.
    ///
    /// O icone e o simbolo da marca, fixo -- ver [`ICON_RGBA`].
    pub fn new() -> Result<Self, TrayError> {
        let item_show = MenuItem::new("Abrir Morune", true, None);
        let item_now = MenuItem::new("Nada tocando", false, None);
        let item_toggle = MenuItem::new("Tocar", true, None);
        let item_previous = MenuItem::new("Anterior", true, None);
        let item_next = MenuItem::new("Proxima", true, None);
        let item_quit = MenuItem::new("Sair do Morune", true, None);

        let menu = Menu::new();
        menu.append_items(&[
            &item_show,
            &PredefinedMenuItem::separator(),
            &item_now,
            &item_toggle,
            &item_previous,
            &item_next,
            &PredefinedMenuItem::separator(),
            &item_quit,
        ])?;

        let icon = TrayIconBuilder::new()
            .with_tooltip("Morune")
            .with_menu(Box::new(menu))
            .with_icon(brand_icon())
            .build()?;

        Ok(Self {
            id_show: item_show.id().clone(),
            id_toggle: item_toggle.id().clone(),
            id_next: item_next.id().clone(),
            id_previous: item_previous.id().clone(),
            id_quit: item_quit.id().clone(),
            item_toggle,
            last_shown: RefCell::new((Some(String::new()), true)),
            item_now,
            _icon: icon,
        })
    }

    /// Atualiza o menu com o estado atual da reproducao.
    ///
    /// Nao faz nada quando nada mudou, que e o caso na esmagadora maioria das
    /// leituras.
    pub fn update(&self, now_playing: Option<&str>, playing: bool) {
        let mut last = self.last_shown.borrow_mut();
        if last.0.as_deref() == now_playing && last.1 == playing {
            return;
        }

        self.item_now.set_text(now_playing.unwrap_or("Nada tocando"));
        self.item_toggle.set_text(if playing { "Pausar" } else { "Tocar" });
        self.item_toggle.set_enabled(now_playing.is_some());

        *last = (now_playing.map(str::to_owned), playing);
    }

    /// Le os eventos pendentes da bandeja.
    ///
    /// Nao bloqueia: devolve o que chegou desde a ultima chamada, na ordem.
    pub fn poll(&self) -> Vec<TrayCommand> {
        let mut out = Vec::new();

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            let command = if event.id == self.id_show {
                TrayCommand::Show
            } else if event.id == self.id_toggle {
                TrayCommand::TogglePlay
            } else if event.id == self.id_next {
                TrayCommand::Next
            } else if event.id == self.id_previous {
                TrayCommand::Previous
            } else if event.id == self.id_quit {
                TrayCommand::Quit
            } else {
                continue;
            };
            out.push(command);
        }

        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            // Clique duplo no icone traz a janela, que e o gesto que as pessoas
            // ja esperam de qualquer aplicativo de bandeja no Windows.
            if let TrayIconEvent::DoubleClick { button: tray_icon::MouseButton::Left, .. } = event {
                out.push(TrayCommand::Show);
            }
        }

        out
    }
}

/// Tamanho do icone em pixels. 32x32 e o que o Windows pede para a bandeja em
/// 100%, e existe um desenho proprio da marca nesse tamanho -- reduzir o de 512
/// no lugar dele borraria os tracos finos.
const ICON_SIZE: u32 = 32;

/// Pixels do simbolo da marca, em RGBA, decodificados no build a partir de
/// `assets/brand/morune-logo-system/morune-symbol-32.png`.
///
/// O icone deixou de acompanhar a cor do tema: ele identifica o aplicativo na
/// bandeja, ao lado de dezenas de outros, e trocar de cor a cada tema tiraria
/// justamente o que faz alguem reconhece-lo. Cor de tema continua valendo para
/// tudo dentro da janela.
const ICON_RGBA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/brand-tray.rgba"));

fn brand_icon() -> Icon {
    Icon::from_rgba(ICON_RGBA.to_vec(), ICON_SIZE, ICON_SIZE)
        .expect("icone da marca tem tamanho valido")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brand_icon_has_expected_pixel_count() {
        assert_eq!(ICON_RGBA.len(), (ICON_SIZE * ICON_SIZE * 4) as usize);
    }

    #[test]
    fn brand_icon_is_a_drawing_and_not_a_block() {
        // Um buffer todo transparente ou todo opaco significa que a conversao
        // do build entregou lixo -- e o icone da bandeja e o unico lugar onde
        // isso passaria despercebido ate alguem olhar o canto da tela.
        let alpha: Vec<u8> = ICON_RGBA.chunks_exact(4).map(|px| px[3]).collect();
        let opaque = alpha.iter().filter(|&&a| a > 250).count();
        let transparent = alpha.iter().filter(|&&a| a < 5).count();

        assert!(opaque > 50, "quase nada opaco: {opaque} px");
        assert!(transparent > 300, "quase nada transparente: {transparent} px");
    }

    #[test]
    fn brand_icon_builds() {
        brand_icon();
    }
}
