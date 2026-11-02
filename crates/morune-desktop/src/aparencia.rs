//! Cor, paleta e transparencia dos paineis escolhidas pela pessoa.
//!
//! Ficam num arquivo proprio, e nao no `config.toml`: a configuracao recusa
//! campo que nao conhece (`deny_unknown_fields`), e o Morune 1.0 instalado leria
//! o arquivo novo como corrompido e voltaria tudo ao padrao.
//!
//! Os valores sao so nomes e numeros. Que cor cada nome vira e decisao da tela
//! (`ui/src/lib/tema.ts`), que conhece o tema aberto -- claro ou escuro.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use morune_theme::{Color, ThemeSpec};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Aparencia {
    /// Cor de destaque escolhida. Vazio = a do tema.
    pub acento: String,
    /// Paleta escolhida. Vazio = a do tema.
    pub paleta: String,
    /// Opacidade dos paineis, em `[0.2, 1.0]`. `0` = a do tema.
    pub paineis: f32,
    /// O modo liberdade total: o que a pessoa trocou peca por peca.
    pub livre: Livre,
}

/// Cores que o modo liberdade total deixa trocar, uma a uma.
///
/// So as que a pessoa reconhece na tela. Os estados (hover, selecionado, foco)
/// continuam vindo do tema: sao derivados e ninguem os procura por nome.
pub const CORES_LIVRES: &[&str] = &[
    "background",
    "surface",
    "surface_raised",
    "sidebar_background",
    "player_background",
    "text",
    "text_muted",
    "accent",
    "border",
];

/// Fontes que vem com o Windows 10 e 11. Uma lista, e nao campo livre: o nome
/// vai parar no CSS, e uma fonte que nao existe cairia na do sistema sem aviso.
pub const FONTES: &[&str] = &[
    "Segoe UI",
    "Segoe UI Variable Display",
    "Bahnschrift",
    "Cascadia Code",
    "Consolas",
    "Arial",
    "Verdana",
    "Tahoma",
    "Trebuchet MS",
    "Georgia",
    "Cambria",
    "Palatino Linotype",
    "Candara",
    "Corbel",
    "Comic Sans MS",
    "Ink Free",
];

/// Ajustes do modo liberdade total. Tudo opcional: vazio = o do tema.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Livre {
    /// Cor por nome do token do tema (`CORES_LIVRES`), em `#rrggbb`.
    pub cores: BTreeMap<String, String>,
    /// Raio dos paineis, em pixels; os outros cantos acompanham na proporcao.
    pub cantos: Option<f32>,
    /// Multiplica todos os espacamentos.
    pub espaco: Option<f32>,
    /// Espessura da borda dos paineis, em pixels.
    pub borda: Option<f32>,
    pub fonte: String,
    pub fonte_titulo: String,
    /// Velocidade das animacoes: `2` = duas vezes mais rapido.
    pub velocidade: Option<f32>,
    /// Brilho em volta da capa, de 0 a 1.
    pub brilho_capa: Option<f32>,
    /// Quanto o fundo se tinge com a capa, de 0 a 1.
    pub fundo_capa: Option<f32>,
}

const NUMEROS: [&str; 6] = [
    "cantos",
    "espaco",
    "borda",
    "velocidade",
    "brilho_capa",
    "fundo_capa",
];

/// Faixa aceita de cada numero do modo livre.
pub fn faixa(chave: &str) -> Option<(f32, f32)> {
    Some(match chave {
        "cantos" => (0.0, 32.0),
        "espaco" => (0.5, 1.6),
        "borda" => (0.0, 4.0),
        "velocidade" => (0.4, 2.5),
        "brilho_capa" | "fundo_capa" => (0.0, 1.0),
        _ => return None,
    })
}

impl Livre {
    pub fn numero_mut(&mut self, chave: &str) -> Option<&mut Option<f32>> {
        Some(match chave {
            "cantos" => &mut self.cantos,
            "espaco" => &mut self.espaco,
            "borda" => &mut self.borda,
            "velocidade" => &mut self.velocidade,
            "brilho_capa" => &mut self.brilho_capa,
            "fundo_capa" => &mut self.fundo_capa,
            _ => return None,
        })
    }

    fn limpa(mut self) -> Self {
        self.cores
            .retain(|k, v| CORES_LIVRES.contains(&k.as_str()) && cor_valida(v));
        for chave in NUMEROS {
            let (min, max) = faixa(chave).unwrap_or((0.0, 1.0));
            if let Some(n) = self.numero_mut(chave) {
                *n = n.filter(|v| v.is_finite()).map(|v| v.clamp(min, max));
            }
        }
        if !FONTES.contains(&self.fonte.as_str()) {
            self.fonte.clear();
        }
        if !FONTES.contains(&self.fonte_titulo.as_str()) {
            self.fonte_titulo.clear();
        }
        self
    }

    /// Poe os ajustes por cima do tema. A transparencia de cada cor continua a
    /// do tema: quem troca a cor do painel do Vidro nao quer um painel opaco.
    pub fn aplicar(&self, spec: &mut ThemeSpec) {
        let c = &mut spec.colors;
        for (chave, valor) in &self.cores {
            let Ok(nova) = valor.parse::<Color>() else {
                continue;
            };
            let alvo = match chave.as_str() {
                "background" => &mut c.background,
                "surface" => &mut c.surface,
                "surface_raised" => &mut c.surface_raised,
                "sidebar_background" => &mut c.sidebar_background,
                "player_background" => &mut c.player_background,
                "text" => &mut c.text,
                "text_muted" => &mut c.text_muted,
                "accent" => &mut c.accent,
                "border" => &mut c.border,
                _ => continue,
            };
            *alvo = nova.with_alpha(alvo.a);
            match chave.as_str() {
                "accent" => {
                    c.accent_hover = nova.mix(Color::rgb(255, 255, 255), 0.18);
                    // Texto em cima do destaque: o que tiver mais contraste.
                    let preto = Color::rgb(0x11, 0x11, 0x11);
                    let branco = Color::rgb(255, 255, 255);
                    c.text_on_accent = if nova.contrast_ratio(preto) >= nova.contrast_ratio(branco)
                    {
                        preto
                    } else {
                        branco
                    };
                }
                "border" => c.border_highlight = c.border,
                _ => {}
            }
        }

        let sh = &mut spec.shape;
        if let Some(raio) = self.cantos {
            // Os outros cantos mantem a proporcao que o tema deu a eles, e
            // nunca passam do raio dos paineis.
            let fator = raio / sh.radius_lg.max(1.0);
            sh.radius_sm = (sh.radius_sm * fator).min(raio);
            sh.radius_md = (sh.radius_md * fator).min(raio);
            sh.radius_artwork = (sh.radius_artwork * fator).min(raio);
            sh.radius_lg = raio;
        }
        if let Some(f) = self.espaco {
            for v in [
                &mut sh.spacing_xs,
                &mut sh.spacing_sm,
                &mut sh.spacing_md,
                &mut sh.spacing_lg,
                &mut sh.spacing_xl,
            ] {
                *v = (*v * f).round();
            }
        }
        if let Some(b) = self.borda {
            sh.border_width = b;
        }

        if !self.fonte.is_empty() {
            spec.typography.family = self.fonte.clone();
            spec.typography.display_family = self.fonte.clone();
            spec.typography.bundled_font = None;
            spec.typography.bundled_fonts.clear();
        }
        if !self.fonte_titulo.is_empty() {
            spec.typography.display_family = self.fonte_titulo.clone();
        }

        if let Some(v) = self.velocidade {
            spec.motion.speed = v;
        }
        if let Some(v) = self.brilho_capa {
            spec.effects.artwork_glow = v;
        }
        if let Some(v) = self.fundo_capa {
            spec.effects.artwork_tint = v > 0.0;
            spec.effects.artwork_tint_strength = v;
        }
    }
}

/// So `#rrggbb`: a transparencia vem do tema, nao do seletor de cor.
pub fn cor_valida(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

impl Aparencia {
    /// Tira o que um arquivo mexido a mao poderia trazer de estranho.
    fn limpa(mut self) -> Self {
        let nome =
            |s: &str| s.len() <= 32 && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'-');
        if !nome(&self.acento) {
            self.acento.clear();
        }
        if !nome(&self.paleta) {
            self.paleta.clear();
        }
        if !self.paineis.is_finite() || self.paineis <= 0.0 {
            self.paineis = 0.0;
        } else {
            self.paineis = self.paineis.clamp(0.2, 1.0);
        }
        self.livre = self.livre.limpa();
        self
    }
}

pub fn arquivo(pasta_config: &Path) -> PathBuf {
    pasta_config.join("aparencia.json")
}

pub fn ler(pasta_config: &Path) -> Aparencia {
    let Ok(texto) = std::fs::read_to_string(arquivo(pasta_config)) else {
        return Aparencia::default();
    };
    serde_json::from_str::<Aparencia>(&texto)
        .map(Aparencia::limpa)
        .unwrap_or_else(|error| {
            tracing::warn!(%error, "aparencia ilegivel; volta a do tema");
            Aparencia::default()
        })
}

/// Grava num arquivo temporario e troca: queda no meio nao deixa meio arquivo.
pub fn gravar(pasta_config: &Path, aparencia: &Aparencia) {
    let destino = arquivo(pasta_config);
    let temp = destino.with_extension("json.tmp");
    let resultado = serde_json::to_vec(aparencia)
        .map_err(std::io::Error::other)
        .and_then(|bytes| std::fs::write(&temp, bytes))
        .and_then(|_| std::fs::rename(&temp, &destino));
    if let Err(error) = resultado {
        tracing::warn!(%error, "nao foi possivel gravar a aparencia");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valor_estranho_volta_ao_tema() {
        let a = Aparencia {
            acento: "../../x".into(),
            paleta: "Noite".into(),
            paineis: f32::NAN,
            livre: Livre {
                cores: [
                    ("background".into(), "red; x".into()),
                    ("outra".into(), "#ffffff".into()),
                ]
                .into(),
                fonte: "x\"; } body { color: red".into(),
                cantos: Some(f32::INFINITY),
                ..Default::default()
            },
        }
        .limpa();
        assert_eq!(a, Aparencia::default());
    }

    #[test]
    fn paineis_nao_somem() {
        let a = Aparencia {
            paineis: 0.01,
            ..Default::default()
        }
        .limpa();
        assert_eq!(a.paineis, 0.2);
    }

    #[test]
    fn cor_livre_guarda_a_transparencia_do_tema() {
        let mut spec = ThemeSpec::default();
        spec.colors.surface = Color::rgba(0, 0, 0, 0x80);
        let livre = Livre {
            cores: [
                ("surface".into(), "#ff0000".into()),
                ("accent".into(), "#ffee00".into()),
            ]
            .into(),
            cantos: Some(0.0),
            ..Default::default()
        };
        livre.aplicar(&mut spec);
        assert_eq!(spec.colors.surface, Color::rgba(255, 0, 0, 0x80));
        // Amarelo pede texto escuro em cima.
        assert!(spec.colors.text_on_accent.relative_luminance() < 0.1);
        assert_eq!((spec.shape.radius_sm, spec.shape.radius_lg), (0.0, 0.0));
    }

    #[test]
    fn grava_e_le_de_volta() {
        let pasta = std::env::temp_dir().join(format!("morune-aparencia-{}", std::process::id()));
        std::fs::create_dir_all(&pasta).unwrap();
        let a = Aparencia {
            acento: "coral".into(),
            paleta: "noite".into(),
            paineis: 0.6,
            livre: Livre {
                cores: [("surface".into(), "#123456".into())].into(),
                cantos: Some(0.0),
                fonte: "Georgia".into(),
                ..Default::default()
            },
        };
        gravar(&pasta, &a);
        assert_eq!(ler(&pasta), a);
        let _ = std::fs::remove_dir_all(&pasta);
    }
}
