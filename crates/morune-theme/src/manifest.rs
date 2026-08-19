use serde::{Deserialize, Serialize};

/// Versao do esquema de tema que este binario entende.
///
/// Incrementada apenas quando uma mudanca quebra temas antigos. Temas com
/// versao menor sao migrados; temas com versao maior sao recusados com uma
/// mensagem clara, em vez de carregados pela metade.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Identificacao de um pacote de tema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeManifest {
    /// Versao do esquema em que este tema foi escrito.
    pub schema_version: u32,

    /// Identificador estavel, usado como nome de pasta e em configuracao.
    /// Restrito a `[a-z0-9_-]` para nunca virar um caminho perigoso.
    pub id: String,

    pub name: String,

    #[serde(default)]
    pub version: String,

    #[serde(default)]
    pub author: String,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub license: String,

    #[serde(default)]
    pub homepage: String,

    /// Id do tema do qual este herda. Os tokens do pai sao aplicados primeiro.
    #[serde(default)]
    pub based_on: Option<String>,

    /// Versao minima do aplicativo. Vazio = sem exigencia.
    #[serde(default)]
    pub min_app_version: String,

    /// Preferencia de esquema declarada pelo autor, usada so para agrupar temas
    /// na tela de selecao.
    #[serde(default)]
    pub appearance: Appearance,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    #[default]
    Dark,
    Light,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ManifestError {
    #[error("id de tema vazio")]
    EmptyId,
    #[error("id de tema invalido: {0:?} (use apenas a-z, 0-9, hifen e sublinhado)")]
    InvalidId(String),
    #[error("nome de tema vazio")]
    EmptyName,
    #[error(
        "tema escrito para o esquema {found}, mas este aplicativo entende ate o {supported}"
    )]
    SchemaTooNew { found: u32, supported: u32 },
    #[error("tema herda de si mesmo")]
    SelfInheritance,
}

impl ThemeManifest {
    /// Manifesto minimo valido, usado pelo tema embutido e ao duplicar temas.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            id: id.into(),
            name: name.into(),
            version: "1.0.0".into(),
            author: String::new(),
            description: String::new(),
            license: String::new(),
            homepage: String::new(),
            based_on: None,
            min_app_version: String::new(),
            appearance: Appearance::Dark,
        }
    }

    /// Valida o manifesto.
    ///
    /// A checagem do `id` e de seguranca, nao de estilo: o id vira nome de
    /// pasta em disco, entao qualquer barra, ponto-ponto ou caractere de
    /// unidade seria um caminho arbitrario disfarcado.
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.id.is_empty() {
            return Err(ManifestError::EmptyId);
        }
        if !is_safe_id(&self.id) {
            return Err(ManifestError::InvalidId(self.id.clone()));
        }
        if self.name.trim().is_empty() {
            return Err(ManifestError::EmptyName);
        }
        if self.schema_version > CURRENT_SCHEMA_VERSION {
            return Err(ManifestError::SchemaTooNew {
                found: self.schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }
        if self.based_on.as_deref() == Some(self.id.as_str()) {
            return Err(ManifestError::SelfInheritance);
        }
        Ok(())
    }
}

/// `true` para ids compostos so de minusculas ASCII, digitos, hifen e sublinhado.
pub fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_manifest_is_valid() {
        assert!(ThemeManifest::new("midnight", "Midnight").validate().is_ok());
    }

    #[test]
    fn path_traversal_ids_are_rejected() {
        for bad in ["../evil", "..", "a/b", "a\\b", "C:", "con", "nome com espaco", "MAIUSCULA"] {
            let m = ThemeManifest::new(bad, "x");
            // "con" e um nome reservado no Windows mas passa no filtro de
            // caracteres; o que importa aqui e que nada com separador ou
            // maiuscula escape.
            if bad == "con" {
                assert!(m.validate().is_ok());
                continue;
            }
            assert!(
                matches!(m.validate(), Err(ManifestError::InvalidId(_))),
                "id perigoso aceito: {bad:?}"
            );
        }
    }

    #[test]
    fn empty_fields_are_rejected() {
        assert_eq!(ThemeManifest::new("", "x").validate(), Err(ManifestError::EmptyId));
        assert_eq!(ThemeManifest::new("ok", "   ").validate(), Err(ManifestError::EmptyName));
    }

    #[test]
    fn newer_schema_is_refused_with_both_versions_named() {
        let mut m = ThemeManifest::new("x", "X");
        m.schema_version = CURRENT_SCHEMA_VERSION + 5;
        let err = m.validate().unwrap_err();
        assert_eq!(
            err,
            ManifestError::SchemaTooNew {
                found: CURRENT_SCHEMA_VERSION + 5,
                supported: CURRENT_SCHEMA_VERSION
            }
        );
        assert!(err.to_string().contains(&(CURRENT_SCHEMA_VERSION + 5).to_string()));
    }

    #[test]
    fn self_inheritance_is_refused() {
        let mut m = ThemeManifest::new("x", "X");
        m.based_on = Some("x".into());
        assert_eq!(m.validate(), Err(ManifestError::SelfInheritance));
    }

    #[test]
    fn long_ids_are_bounded() {
        assert!(!is_safe_id(&"a".repeat(65)));
        assert!(is_safe_id(&"a".repeat(64)));
    }

    #[test]
    fn manifest_round_trips_through_toml() {
        let m = ThemeManifest::new("paper", "Paper");
        let text = toml::to_string(&m).unwrap();
        assert_eq!(toml::from_str::<ThemeManifest>(&text).unwrap(), m);
    }

    #[test]
    fn unknown_manifest_field_is_an_error_not_a_silent_drop() {
        let text = "schema_version = 1\nid = \"x\"\nname = \"X\"\nautor = \"typo\"\n";
        assert!(toml::from_str::<ThemeManifest>(text).is_err());
    }
}
