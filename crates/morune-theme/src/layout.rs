use serde::{Deserialize, Serialize};

/// Onde a barra lateral fica.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SidebarPosition {
    #[default]
    Left,
    Right,
    /// Sem barra lateral: navegacao vai para o topo.
    Hidden,
}

/// Onde a barra de reproducao fica.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlayerPosition {
    #[default]
    Bottom,
    Top,
    /// Player integrado ao rodape da barra lateral.
    Sidebar,
}

/// Como uma colecao e apresentada.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ViewMode {
    #[default]
    Grid,
    List,
    /// Lista compacta, sem capa.
    Compact,
}

/// Densidade geral da interface.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Density {
    Comfortable,
    #[default]
    Normal,
    Compact,
}

impl Density {
    /// Multiplicador aplicado a espacamentos e alturas de linha.
    pub fn factor(self) -> f32 {
        match self {
            Density::Comfortable => 1.25,
            Density::Normal => 1.0,
            Density::Compact => 0.8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WindowLayout {
    pub default_width: f32,
    pub default_height: f32,
    pub min_width: f32,
    pub min_height: f32,
    /// Desenha a propria barra de titulo em vez de usar a do sistema.
    pub custom_titlebar: bool,
    /// Mostra o titulo da pagina atual na barra de titulo.
    pub show_page_title: bool,
}

impl Default for WindowLayout {
    fn default() -> Self {
        Self {
            default_width: 1180.0,
            default_height: 760.0,
            // Minimos baixos o bastante para caber num monitor 1366x768 com
            // escala 125%, que ainda e comum no Windows.
            min_width: 720.0,
            min_height: 480.0,
            custom_titlebar: false,
            show_page_title: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SidebarLayout {
    pub position: SidebarPosition,
    pub width: f32,
    /// Largura quando recolhida (so icones).
    pub collapsed_width: f32,
    pub collapsible: bool,
    pub start_collapsed: bool,
    pub show_icons: bool,
    pub show_labels: bool,
    /// Mostra a lista de playlists abaixo da navegacao.
    pub show_playlists: bool,
    /// Altura de cada linha da lista de playlists.
    ///
    /// E ela que define o tamanho da capa: recolhida, a barra vira uma coluna
    /// de capas, e essa altura passa a ser a unica coisa que separa uma
    /// playlist da outra.
    pub playlist_height: f32,
    /// Ordem dos itens de navegacao. Nomes desconhecidos sao ignorados com aviso.
    pub items: Vec<String>,
}

/// Tela de uma lista aberta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DetailLayout {
    /// Lado da capa no cabecalho.
    pub cover_size: f32,
}

impl Default for DetailLayout {
    fn default() -> Self {
        Self { cover_size: 160.0 }
    }
}

impl Default for SidebarLayout {
    fn default() -> Self {
        Self {
            position: SidebarPosition::Left,
            width: 232.0,
            collapsed_width: 64.0,
            collapsible: true,
            start_collapsed: false,
            show_icons: true,
            show_labels: true,
            show_playlists: true,
            playlist_height: 40.0,
            items: ["home", "search", "library"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PlayerLayout {
    pub position: PlayerPosition,
    pub height: f32,
    pub show_artwork: bool,
    pub artwork_size: f32,
    pub show_progress: bool,
    /// Barra de progresso colada na borda da janela, sem margem.
    pub progress_edge_to_edge: bool,
    pub show_volume: bool,
    pub show_shuffle_repeat: bool,
    pub show_queue_button: bool,
    /// Mostra tempo decorrido/total ao lado da barra.
    pub show_times: bool,
    /// Centraliza os controles de transporte em vez de alinhar a esquerda.
    pub center_controls: bool,
}

impl Default for PlayerLayout {
    fn default() -> Self {
        Self {
            position: PlayerPosition::Bottom,
            height: 88.0,
            show_artwork: true,
            artwork_size: 56.0,
            show_progress: true,
            progress_edge_to_edge: false,
            show_volume: true,
            show_shuffle_repeat: true,
            show_queue_button: true,
            show_times: true,
            center_controls: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContentLayout {
    pub density: Density,
    /// Modo de exibicao padrao de colecoes.
    pub view_mode: ViewMode,
    /// Largura alvo de um card no grid. O numero de colunas e derivado dela.
    pub card_width: f32,
    /// Altura de uma linha em modo lista.
    pub row_height: f32,
    /// Largura maxima do conteudo. `0` = ocupa tudo.
    pub max_content_width: f32,
    /// Mostra o cabecalho grande com capa nas paginas de album/playlist.
    pub show_hero: bool,
    pub hero_height: f32,
    /// Numero de faixas por pagina ao rolar listas longas.
    pub page_size: u32,
}

impl Default for ContentLayout {
    fn default() -> Self {
        Self {
            density: Density::Normal,
            view_mode: ViewMode::Grid,
            card_width: 168.0,
            row_height: 44.0,
            max_content_width: 0.0,
            show_hero: true,
            hero_height: 240.0,
            page_size: 100,
        }
    }
}

/// Composicao completa da interface.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LayoutSpec {
    pub window: WindowLayout,
    pub sidebar: SidebarLayout,
    #[serde(default)]
    pub detail: DetailLayout,
    pub player: PlayerLayout,
    pub content: ContentLayout,
}

impl LayoutSpec {
    /// Corrige valores que tornariam a interface inutilizavel.
    ///
    /// Preferimos consertar em silencio e avisar a recusar o tema: o objetivo e
    /// que um tema mal escrito nunca impeca o aplicativo de abrir.
    pub fn sanitize(&mut self) -> Vec<String> {
        let mut warnings = Vec::new();
        let mut clamp = |name: &str, value: &mut f32, min: f32, max: f32| {
            if *value < min || *value > max || !value.is_finite() {
                let old = *value;
                *value = if value.is_finite() {
                    value.clamp(min, max)
                } else {
                    min
                };
                warnings.push(format!(
                    "{name}: {old} fora da faixa [{min}, {max}], ajustado para {}",
                    *value
                ));
            }
        };

        clamp(
            "window.default_width",
            &mut self.window.default_width,
            480.0,
            8000.0,
        );
        clamp(
            "window.default_height",
            &mut self.window.default_height,
            360.0,
            8000.0,
        );
        clamp(
            "window.min_width",
            &mut self.window.min_width,
            480.0,
            4000.0,
        );
        clamp(
            "window.min_height",
            &mut self.window.min_height,
            360.0,
            4000.0,
        );
        clamp("sidebar.width", &mut self.sidebar.width, 120.0, 600.0);
        clamp(
            "sidebar.collapsed_width",
            &mut self.sidebar.collapsed_width,
            40.0,
            200.0,
        );
        // Abaixo de 24 a capa some; acima de 96 cabem meia duzia de playlists
        // na barra inteira, o que a torna inutil como navegacao.
        clamp(
            "sidebar.playlist_height",
            &mut self.sidebar.playlist_height,
            24.0,
            96.0,
        );
        // Abaixo de 64 a capa nao serve de cabecalho; acima de 320 ela empurra a
        // lista inteira para fora da primeira tela.
        clamp(
            "detail.cover_size",
            &mut self.detail.cover_size,
            64.0,
            320.0,
        );
        clamp("player.height", &mut self.player.height, 48.0, 260.0);
        clamp(
            "player.artwork_size",
            &mut self.player.artwork_size,
            0.0,
            240.0,
        );
        clamp(
            "content.card_width",
            &mut self.content.card_width,
            80.0,
            480.0,
        );
        clamp(
            "content.row_height",
            &mut self.content.row_height,
            24.0,
            120.0,
        );
        clamp(
            "content.hero_height",
            &mut self.content.hero_height,
            0.0,
            600.0,
        );

        if self.content.page_size == 0 || self.content.page_size > 1000 {
            warnings.push(format!(
                "content.page_size: {} fora da faixa [1, 1000], ajustado para 100",
                self.content.page_size
            ));
            self.content.page_size = 100;
        }

        if self.window.min_width > self.window.default_width {
            warnings.push("window.min_width maior que default_width, igualados".into());
            self.window.default_width = self.window.min_width;
        }
        if self.window.min_height > self.window.default_height {
            warnings.push("window.min_height maior que default_height, igualados".into());
            self.window.default_height = self.window.min_height;
        }

        // Uma sidebar sem icone e sem rotulo seria uma coluna vazia clicavel.
        if !self.sidebar.show_icons && !self.sidebar.show_labels {
            warnings.push("sidebar sem icones e sem rotulos, rotulos reativados".into());
            self.sidebar.show_labels = true;
        }

        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_layout_needs_no_correction() {
        let mut l = LayoutSpec::default();
        assert!(l.sanitize().is_empty());
    }

    #[test]
    fn absurd_dimensions_are_clamped_not_rejected() {
        let mut l = LayoutSpec::default();
        l.sidebar.width = 9999.0;
        l.player.height = 1.0;
        let w = l.sanitize();
        assert_eq!(l.sidebar.width, 600.0);
        assert_eq!(l.player.height, 48.0);
        assert_eq!(w.len(), 2);
    }

    #[test]
    fn non_finite_values_do_not_survive() {
        let mut l = LayoutSpec::default();
        l.content.card_width = f32::NAN;
        l.window.default_width = f32::INFINITY;
        l.sanitize();
        assert!(l.content.card_width.is_finite());
        assert!(l.window.default_width.is_finite());
    }

    #[test]
    fn minimum_larger_than_default_is_reconciled() {
        let mut l = LayoutSpec::default();
        l.window.min_width = 1600.0;
        l.window.default_width = 900.0;
        l.sanitize();
        assert_eq!(l.window.default_width, 1600.0);
    }

    #[test]
    fn invisible_sidebar_items_are_restored() {
        let mut l = LayoutSpec::default();
        l.sidebar.show_icons = false;
        l.sidebar.show_labels = false;
        assert_eq!(l.sanitize().len(), 1);
        assert!(l.sidebar.show_labels);
    }

    #[test]
    fn zero_page_size_would_stall_scrolling() {
        let mut l = LayoutSpec::default();
        l.content.page_size = 0;
        l.sanitize();
        assert_eq!(l.content.page_size, 100);
    }

    #[test]
    fn layout_round_trips_through_toml() {
        let l = LayoutSpec::default();
        let text = toml::to_string(&l).unwrap();
        assert_eq!(toml::from_str::<LayoutSpec>(&text).unwrap(), l);
    }

    #[test]
    fn partial_layout_keeps_defaults_for_the_rest() {
        let l: LayoutSpec = toml::from_str("[sidebar]\nposition = \"right\"\n").unwrap();
        assert_eq!(l.sidebar.position, SidebarPosition::Right);
        assert_eq!(l.sidebar.width, SidebarLayout::default().width);
        assert_eq!(l.player, PlayerLayout::default());
    }

    #[test]
    fn density_factors_are_ordered() {
        assert!(Density::Compact.factor() < Density::Normal.factor());
        assert!(Density::Normal.factor() < Density::Comfortable.factor());
    }
}
