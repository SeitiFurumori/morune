//! Notificacao do Windows quando a faixa muda.
//!
//! **Para quem e.** Quem troca de faixa pelo atalho global dentro do jogo nao ve
//! a janela do Morune, e sem isto nao sabe o que comecou a tocar. O Spotify tem
//! a mesma opcao ("Mostrar notificacoes na area de trabalho ao mudar de
//! musica"), ligada por padrao.
//!
//! **O que ela nao faz.** Em jogo de tela cheia *exclusiva* o Windows segura
//! toda notificacao ate o jogo sair de frente. Em janela sem borda ela aparece.
//! Isso e do sistema, e nao ha o que um aplicativo faca a respeito sem desenhar
//! por cima do jogo -- o que esta fora do orcamento do projeto.
//!
//! **Por que o registro.** Aplicativo de area de trabalho nao empacotado so
//! consegue mostrar notificacao com um AppUserModelID que o Windows conheca.
//! Registrar a chave em `HKCU\Software\Classes\AppUserModelId` basta; nao
//! precisa de atalho no menu Iniciar com o ID gravado, e nao mudamos o ID do
//! processo -- mudar separaria a janela do atalho fixado na barra de tarefas.
//! O desinstalador apaga a chave.
//!
//! Silenciosa de proposito: som de notificacao por cima do audio do jogo e da
//! propria musica e justamente o incomodo que o produto quer evitar.

use std::ffi::c_void;
use std::path::Path;
use std::time::{Duration, Instant};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::core::{w, HSTRING, PCWSTR};
use windows::Data::Xml::Dom::XmlDocument;
use windows::Win32::Foundation::{ERROR_SUCCESS, HWND};
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE,
    REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager, ToastNotifier};

use crate::smtc::MediaStatus;

/// Identidade das notificacoes do Morune no Windows.
///
/// Fixa para sempre: trocar este texto deixaria a chave antiga orfa no
/// registro e as notificacoes antigas na central com outro nome. O
/// desinstalador (`installer/morune.nsi`) repete o mesmo valor.
const AUMID: &str = "Morune.Player";
const AUMID_KEY: PCWSTR = w!("Software\\Classes\\AppUserModelId\\Morune.Player");

/// Quanto esperar a capa antes de mostrar sem ela.
///
/// A capa da faixa nova costuma chegar ao cache em menos de um segundo. Sem
/// esta espera a notificacao sairia quase sempre com a capa em branco; com
/// uma espera longa ela chegaria quando a pessoa ja esqueceu que trocou.
const COVER_GRACE: Duration = Duration::from_millis(1200);

pub struct TrackNotifier {
    notifier: ToastNotifier,
    /// Faixa ja anunciada, ou vista na abertura. Nao repetir a mesma.
    anunciada: Option<(String, String)>,
    /// Faixa nova esperando a capa, e desde quando.
    esperando: Option<((String, String), Instant)>,
    /// A primeira faixa vista nao e "troca": e o que estava tocando quando o
    /// Morune abriu, ou a fila restaurada.
    primeira: bool,
}

impl TrackNotifier {
    pub fn new() -> Result<Self, NotifyError> {
        registrar_aumid()?;
        let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(AUMID))?;
        Ok(Self {
            notifier,
            anunciada: None,
            esperando: None,
            primeira: true,
        })
    }

    /// Chamado a cada giro do temporizador. Silencioso quando nada mudou.
    ///
    /// `ligada` e a preferencia da pessoa; `janela` serve para nao anunciar o
    /// que ela esta vendo -- com o Morune em primeiro plano a barra de
    /// reproducao ja diz tudo.
    pub fn update(
        &mut self,
        status: Option<&MediaStatus>,
        ligada: bool,
        janela: &impl HasWindowHandle,
    ) {
        let Some(status) = status else {
            self.esperando = None;
            return;
        };
        let chave = (status.title.clone(), status.artist.clone());

        if self.primeira {
            self.primeira = false;
            self.anunciada = Some(chave);
            return;
        }
        if self.anunciada.as_ref() == Some(&chave) {
            return;
        }
        if self.esperando.as_ref().map(|(c, _)| c) != Some(&chave) {
            self.esperando = Some((chave.clone(), Instant::now()));
        }

        let Some((_, desde)) = self.esperando.as_ref() else {
            return;
        };
        let pronta = status.cover.is_some() || desde.elapsed() >= COVER_GRACE;
        if !pronta {
            return;
        }

        self.esperando = None;
        self.anunciada = Some(chave);
        // Pausado ninguem quer aviso: a troca aconteceu, mas nada esta soando.
        if !ligada || !status.playing || em_primeiro_plano(janela) {
            return;
        }
        if let Err(error) = self.mostrar(status) {
            tracing::warn!(%error, "notificacao de troca de faixa recusada");
        }
    }

    fn mostrar(&self, status: &MediaStatus) -> Result<(), NotifyError> {
        let doc = XmlDocument::new()?;
        doc.LoadXml(&HSTRING::from(toast_xml(status)))?;
        let toast = ToastNotification::CreateToastNotification(&doc)?;
        // Mesma etiqueta sempre: a faixa nova **substitui** a anterior na
        // central de notificacoes, em vez de empilhar uma por musica.
        toast.SetTag(&HSTRING::from("faixa"))?;
        toast.SetGroup(&HSTRING::from("tocando"))?;
        self.notifier.Show(&toast)?;
        Ok(())
    }
}

/// Monta o XML da notificacao.
fn toast_xml(status: &MediaStatus) -> String {
    let capa = status
        .cover
        .as_deref()
        .and_then(uri_de_arquivo)
        .map(|uri| {
            format!(
                r#"<image placement="appLogoOverride" src="{}"/>"#,
                escapar(&uri)
            )
        })
        .unwrap_or_default();
    format!(
        concat!(
            r#"<toast duration="short"><visual><binding template="ToastGeneric">"#,
            "<text>{}</text><text>{}</text>{}",
            r#"</binding></visual><audio silent="true"/></toast>"#
        ),
        escapar(&status.title),
        escapar(&status.artist),
        capa
    )
}

/// Caminho de capa em disco como URI `file:`. Ver `stream_de_arquivo` em
/// [`crate::smtc`], que segue a mesma regra.
fn uri_de_arquivo(caminho: &Path) -> Option<String> {
    let texto = caminho.to_str()?;
    Some(format!("file:///{}", texto.replace('\\', "/")))
}

/// Titulo de musica pode ter `&` e `<` ("Rock & Roll", "<3"); sem escapar, o
/// XML nao carrega e a notificacao some sem aviso.
fn escapar(texto: &str) -> String {
    let mut saida = String::with_capacity(texto.len());
    for c in texto.chars() {
        match c {
            '&' => saida.push_str("&amp;"),
            '<' => saida.push_str("&lt;"),
            '>' => saida.push_str("&gt;"),
            '"' => saida.push_str("&quot;"),
            '\'' => saida.push_str("&apos;"),
            _ => saida.push(c),
        }
    }
    saida
}

/// `true` quando a janela do Morune e a que esta na frente.
fn em_primeiro_plano(janela: &impl HasWindowHandle) -> bool {
    let Ok(handle) = janela.window_handle() else {
        return false;
    };
    let RawWindowHandle::Win32(raw) = handle.as_raw() else {
        return false;
    };
    let nossa = HWND(raw.hwnd.get() as *mut c_void);
    // SAFETY: sem parametros; devolve a janela em primeiro plano ou nula.
    let frente = unsafe { GetForegroundWindow() };
    // SAFETY: HWND valido enquanto a janela existir.
    frente == nossa
        && unsafe { windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(nossa) }.as_bool()
}

/// Registra o nome que o Windows mostra no alto da notificacao.
///
/// Reescrito a cada abertura, e nao so na instalacao: e barato, e cobre quem
/// roda o executavel solto, fora do instalador.
fn registrar_aumid() -> Result<(), NotifyError> {
    let mut key = HKEY::default();
    // SAFETY: cria/abre uma chave do usuario atual com caminho constante.
    let created = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            AUMID_KEY,
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
    };
    if created != ERROR_SUCCESS {
        return Err(NotifyError::Registry(created.0));
    }

    let nome: Vec<u16> = "Otomori\0".encode_utf16().collect();
    // SAFETY: REG_SZ recebe os bytes UTF-16 com o terminador; o buffer vive
    // durante a chamada.
    let bytes = unsafe { std::slice::from_raw_parts(nome.as_ptr().cast::<u8>(), nome.len() * 2) };
    // SAFETY: chave valida, nome constante.
    let written = unsafe { RegSetValueExW(key, w!("DisplayName"), None, REG_SZ, Some(bytes)) };
    // SAFETY: fecha exatamente a chave criada acima.
    let _ = unsafe { RegCloseKey(key) };
    if written == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(NotifyError::Registry(written.0))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NotifyError {
    #[error("o Windows recusou o registro das notificacoes (codigo {0})")]
    Registry(u32),
    #[error("{0}")]
    Windows(#[from] windows::core::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn faixa(title: &str, artist: &str) -> MediaStatus {
        MediaStatus {
            title: title.into(),
            artist: artist.into(),
            album: String::new(),
            cover: None,
            playing: true,
        }
    }

    #[test]
    fn titulo_com_e_comercial_vira_xml_valido() {
        let xml = toast_xml(&faixa("Rock & Roll <3", "Fulano \"F\""));
        assert!(xml.contains("Rock &amp; Roll &lt;3"));
        assert!(xml.contains("Fulano &quot;F&quot;"));
        let doc = XmlDocument::new().unwrap();
        doc.LoadXml(&HSTRING::from(xml)).expect("xml carrega");
    }

    #[test]
    fn a_notificacao_e_silenciosa() {
        assert!(toast_xml(&faixa("a", "b")).contains(r#"<audio silent="true"/>"#));
    }

    #[test]
    fn capa_em_disco_entra_como_uri_de_arquivo() {
        let mut status = faixa("a", "b");
        status.cover = Some(r"C:\Users\x\capa.jpg".into());
        assert!(toast_xml(&status).contains(r#"src="file:///C:/Users/x/capa.jpg""#));
    }
}
