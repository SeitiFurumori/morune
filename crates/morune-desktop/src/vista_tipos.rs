//! Tipos que substituem os do Slint na ponte com a tela web.
//!
//! O motor foi escrito contra a janela do Slint. Estes tipos tem os mesmos nomes
//! e o mesmo uso, mas so guardam dados que viram JSON.
use serde::{Serialize, Serializer};
use std::path::Path;

pub type SharedString = String;
pub type ModelRc<T> = Vec<T>;

/// Uma imagem em disco, vista pela tela como endereco do protocolo `capa`.
/// Vazia quando nao ha imagem: a tela mostra a ficha no lugar.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Imagem(pub String);

impl Imagem {
    pub fn de_arquivo(path: &Path) -> Self {
        Self(crate::capas::endereco(path))
    }
}

/// Cor RGBA. Vai para a tela como `#rrggbbaa`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Cor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Cor {
    pub fn from_argb_u8(a: u8, r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_rgb_u8(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn red(&self) -> u8 {
        self.r
    }
    pub fn green(&self) -> u8 {
        self.g
    }
    pub fn blue(&self) -> u8 {
        self.b
    }
    pub fn alpha(&self) -> u8 {
        self.a
    }
}

impl From<morune_theme::Color> for Cor {
    fn from(c: morune_theme::Color) -> Self {
        Self::from_argb_u8(c.a, c.r, c.g, c.b)
    }
}

impl Serialize for Cor {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            self.r, self.g, self.b, self.a
        ))
    }
}
