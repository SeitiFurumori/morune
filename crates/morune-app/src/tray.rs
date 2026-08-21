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

use std::time::Duration;

use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

/// O que o usuario pediu pela bandeja.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    /// Trazer a janela de volta.
    Show,
    /// Abrir o menu proprio, ancorado no icone.
    ///
    /// As coordenadas sao fisicas e descrevem o retangulo do icone na bandeja,
    /// que e o que o Windows entrega junto do clique. Quem abre o menu decide
    /// de que lado do retangulo ele cabe.
    OpenMenu(IconAnchor),
}

/// Retangulo do icone na bandeja, em pixels fisicos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconAnchor {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Falha ao criar a bandeja.
#[derive(Debug, thiserror::Error)]
pub enum TrayError {
    #[error("icone da bandeja: {0}")]
    Icon(#[from] tray_icon::Error),
}

/// Icone de bandeja ativo.
///
/// A bandeja para de existir quando este valor e descartado, por isso ele
/// precisa viver enquanto o aplicativo viver.
///
/// **Sem menu nativo.** Um `HMENU` e desenhado pelo Windows e nao aceita cor,
/// tipografia nem forma: dentro dele o Morune deixava de parecer o Morune. O
/// clique com o botao direito passa a ser um evento comum, e quem responde e a
/// janela de [`crate::tray_menu`].
pub struct Tray {
    _icon: TrayIcon,
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
        let icon = TrayIconBuilder::new()
            .with_tooltip("Morune")
            .with_icon(brand_icon())
            .build()?;

        Ok(Self { _icon: icon })
    }

    /// Le os eventos pendentes da bandeja.
    ///
    /// Nao bloqueia: devolve o que chegou desde a ultima chamada, na ordem.
    pub fn poll(&self) -> Vec<TrayCommand> {
        let mut out = Vec::new();

        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            match event {
                // Clique duplo no icone traz a janela, que e o gesto que as
                // pessoas ja esperam de qualquer aplicativo de bandeja.
                TrayIconEvent::DoubleClick {
                    button: tray_icon::MouseButton::Left,
                    ..
                } => out.push(TrayCommand::Show),

                // O menu abre ao soltar o botao, e nao ao apertar: soltar e o
                // momento em que o Windows abre os menus de bandeja, e reagir
                // ao aperto faria o menu nascer sob um botao ainda pressionado.
                TrayIconEvent::Click {
                    button: tray_icon::MouseButton::Right,
                    button_state: tray_icon::MouseButtonState::Up,
                    rect,
                    ..
                } => out.push(TrayCommand::OpenMenu(IconAnchor {
                    x: rect.position.x as i32,
                    y: rect.position.y as i32,
                    width: rect.size.width as i32,
                    height: rect.size.height as i32,
                })),

                _ => {}
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
        assert!(
            transparent > 300,
            "quase nada transparente: {transparent} px"
        );
    }

    #[test]
    fn brand_icon_builds() {
        brand_icon();
    }
}
