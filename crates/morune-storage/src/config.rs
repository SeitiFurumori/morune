use std::fs;
use std::path::Path;

use morune_core::queue::RepeatMode;
use serde::{Deserialize, Serialize};

/// Versao do arquivo de configuracao.
///
/// Serve para migrar configuracoes antigas em silencio quando o formato muda.
pub const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub appearance: AppearanceConfig,
    pub playback: PlaybackConfig,
    pub window: WindowConfig,
    pub developer: DeveloperConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            appearance: AppearanceConfig::default(),
            playback: PlaybackConfig::default(),
            window: WindowConfig::default(),
            developer: DeveloperConfig::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppearanceConfig {
    /// Id do tema ativo. Vazio ou desconhecido = tema embutido.
    pub theme: String,
    /// Sobrepoe a escala tipografica do tema, quando o usuario ajusta na
    /// tela de configuracoes. `0` = usa o valor do tema.
    pub font_scale_override: f32,
    /// Desliga animacoes independentemente do tema.
    pub reduce_motion: bool,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self { theme: "midnight".into(), font_scale_override: 0.0, reduce_motion: false }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PlaybackConfig {
    /// Volume linear em `[0.0, 1.0]`.
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    /// Normalizacao de volume entre faixas.
    pub normalize: bool,
    /// Qualidade pedida ao backend de streaming, em kbps. Valores aceitos
    /// dependem do backend; um valor invalido cai para o mais proximo.
    pub bitrate: u32,
    /// Tamanho maximo do cache de audio em MiB. `0` desliga o cache.
    pub audio_cache_mb: u32,
    /// Nome do dispositivo de saida. Vazio = padrao do sistema.
    pub output_device: String,
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        Self {
            volume: 0.7,
            shuffle: false,
            repeat: RepeatMode::Off,
            normalize: true,
            bitrate: 160,
            audio_cache_mb: 1024,
            output_device: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WindowConfig {
    /// Ultimo tamanho da janela. `0` = usa o do tema.
    pub width: f32,
    pub height: f32,
    pub maximized: bool,
    /// Fechar a janela esconde na bandeja e mantem a reproducao.
    ///
    /// Padrao ligado: e o comportamento que a pessoa espera de um player, e
    /// fechar sem querer no meio de uma musica e mais irritante que ter de
    /// sair pela bandeja. Desligavel em Configuracoes.
    pub close_to_tray: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self { width: 0.0, height: 0.0, maximized: false, close_to_tray: true }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DeveloperConfig {
    /// Liga o Developer Mode: recarga a quente, sobreposicao de performance,
    /// ids de componente e logs detalhados.
    pub enabled: bool,
    pub hot_reload: bool,
    pub performance_overlay: bool,
    pub show_component_ids: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("erro de arquivo: {0}")]
    Io(#[from] std::io::Error),
    #[error("configuracao invalida: {0}")]
    Parse(String),
}

impl Config {
    /// Le a configuracao de `path`.
    ///
    /// Arquivo ausente devolve o padrao. Arquivo corrompido tambem devolve o
    /// padrao, e o original e preservado com sufixo `.bak` -- perder a
    /// configuracao do usuario em silencio seria pior que ignora-la.
    pub fn load(path: &Path) -> Self {
        let raw = match fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Self::default(),
            Err(e) => {
                tracing::warn!(error = %e, "configuracao ilegivel, usando padroes");
                return Self::default();
            }
        };

        match toml::from_str::<Config>(&raw) {
            Ok(mut config) => {
                config.migrate();
                config.sanitize();
                config
            }
            Err(e) => {
                tracing::error!(error = %e, "configuracao invalida, usando padroes");
                let backup = path.with_extension("toml.bak");
                if let Err(e) = fs::write(&backup, &raw) {
                    tracing::warn!(error = %e, "nao foi possivel salvar copia da configuracao");
                }
                Self::default()
            }
        }
    }

    /// Grava a configuracao.
    ///
    /// Escreve num arquivo temporario e renomeia por cima: uma queda de energia
    /// no meio da gravacao deixa a configuracao anterior intacta em vez de um
    /// arquivo truncado.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self).map_err(|e| ConfigError::Parse(e.to_string()))?;
        let temp = path.with_extension("toml.tmp");
        fs::write(&temp, text)?;
        fs::rename(&temp, path)?;
        Ok(())
    }

    fn migrate(&mut self) {
        if self.version < CONFIG_VERSION {
            tracing::info!(from = self.version, to = CONFIG_VERSION, "migrando configuracao");
            self.version = CONFIG_VERSION;
        }
    }

    /// Corrige valores fora de faixa. Nunca rejeita a configuracao inteira.
    pub fn sanitize(&mut self) {
        if !self.playback.volume.is_finite() {
            self.playback.volume = PlaybackConfig::default().volume;
        }
        self.playback.volume = self.playback.volume.clamp(0.0, 1.0);
        self.playback.audio_cache_mb = self.playback.audio_cache_mb.min(64 * 1024);

        if !(0.0..=3.0).contains(&self.appearance.font_scale_override)
            || !self.appearance.font_scale_override.is_finite()
        {
            self.appearance.font_scale_override = 0.0;
        }

        for v in [&mut self.window.width, &mut self.window.height] {
            if !v.is_finite() || *v < 0.0 || *v > 16_000.0 {
                *v = 0.0;
            }
        }

        // Sub-opcoes do Developer Mode nao fazem sentido com ele desligado, e
        // deixa-las ligadas custaria uma thread de observacao a toa.
        if !self.developer.enabled {
            self.developer.hot_reload = false;
            self.developer.performance_overlay = false;
            self.developer.show_component_ids = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("morune-config-{name}-{}.toml", std::process::id()))
    }

    #[test]
    fn missing_file_yields_defaults() {
        let c = Config::load(Path::new("D:/nao/existe/config.toml"));
        assert_eq!(c, Config::default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let path = temp_file("roundtrip");
        let mut c = Config::default();
        c.appearance.theme = "paper".into();
        c.playback.volume = 0.42;
        c.developer.enabled = true;
        c.developer.hot_reload = true;

        c.save(&path).unwrap();
        assert_eq!(Config::load(&path), c);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn corrupt_file_falls_back_and_keeps_a_backup() {
        let path = temp_file("corrupt");
        fs::write(&path, "isto nao e toml {{{").unwrap();

        let c = Config::load(&path);
        assert_eq!(c, Config::default());
        assert!(path.with_extension("toml.bak").is_file(), "backup nao foi criado");

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("toml.bak"));
    }

    #[test]
    fn partial_config_keeps_defaults_for_missing_sections() {
        let path = temp_file("partial");
        fs::write(&path, "[playback]\nvolume = 0.25\n").unwrap();

        let c = Config::load(&path);
        assert_eq!(c.playback.volume, 0.25);
        assert_eq!(c.appearance, AppearanceConfig::default());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn out_of_range_values_are_clamped() {
        let mut c = Config::default();
        c.playback.volume = 12.0;
        c.appearance.font_scale_override = f32::NAN;
        c.window.width = -5.0;
        c.sanitize();

        assert_eq!(c.playback.volume, 1.0);
        assert_eq!(c.appearance.font_scale_override, 0.0);
        assert_eq!(c.window.width, 0.0);
    }

    #[test]
    fn developer_suboptions_require_developer_mode() {
        let mut c = Config::default();
        c.developer.hot_reload = true;
        c.developer.performance_overlay = true;
        c.sanitize();
        assert!(!c.developer.hot_reload);
        assert!(!c.developer.performance_overlay);
    }

    #[test]
    fn saving_does_not_leave_a_temporary_file_behind() {
        let path = temp_file("atomic");
        Config::default().save(&path).unwrap();
        assert!(!path.with_extension("toml.tmp").exists());
        let _ = fs::remove_file(&path);
    }
}
