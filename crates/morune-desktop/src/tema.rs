//! O que sobrou da ponte de tema do Slint: os ajustes da pessoa e a hora local.
//! Aplicar o tema agora e trabalho da tela web, a partir de `AppState::tema_json`.

/// Ajustes da pessoa por cima do tema.
///
/// Existem separados do `ThemeSpec` de proposito: sao preferencias da pessoa,
/// nao do tema, e precisam sobreviver a uma troca de tema.
#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
pub struct UserOverrides {
    /// `0.0` = usa a escala do tema.
    pub font_scale: f32,
    /// Desliga animacoes independentemente do que o tema pede.
    pub reduce_motion: bool,
    /// Barra lateral recolhida.
    pub sidebar_collapsed: bool,
}

/// Hora local em horas fracionadas (13,5 = 13h30).
pub fn hora_local() -> f32 {
    #[cfg(windows)]
    {
        // SAFETY: preenche e devolve uma estrutura por valor; sem ponteiros.
        let agora = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
        agora.wHour as f32 + agora.wMinute as f32 / 60.0
    }
    #[cfg(not(windows))]
    {
        0.0
    }
}
