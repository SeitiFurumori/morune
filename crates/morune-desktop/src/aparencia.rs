//! Cor, paleta e transparencia dos paineis escolhidas pela pessoa.
//!
//! Ficam num arquivo proprio, e nao no `config.toml`: a configuracao recusa
//! campo que nao conhece (`deny_unknown_fields`), e o Morune 1.0 instalado leria
//! o arquivo novo como corrompido e voltaria tudo ao padrao.
//!
//! Os valores sao so nomes e numeros. Que cor cada nome vira e decisao da tela
//! (`ui/src/lib/tema.ts`), que conhece o tema aberto -- claro ou escuro.

use std::path::{Path, PathBuf};

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
}

impl Aparencia {
    /// Tira o que um arquivo mexido a mao poderia trazer de estranho.
    fn limpa(mut self) -> Self {
        let nome = |s: &str| s.len() <= 32 && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'-');
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
        }
        .limpa();
        assert_eq!(a, Aparencia::default());
    }

    #[test]
    fn paineis_nao_somem() {
        let a = Aparencia { paineis: 0.01, ..Default::default() }.limpa();
        assert_eq!(a.paineis, 0.2);
    }

    #[test]
    fn grava_e_le_de_volta() {
        let pasta = std::env::temp_dir().join(format!("morune-aparencia-{}", std::process::id()));
        std::fs::create_dir_all(&pasta).unwrap();
        let a = Aparencia { acento: "coral".into(), paleta: "noite".into(), paineis: 0.6 };
        gravar(&pasta, &a);
        assert_eq!(ler(&pasta), a);
        let _ = std::fs::remove_dir_all(&pasta);
    }
}
