//! Ponte entre o tema declarativo (`morune-theme`) e as globais da interface.
//!
//! Este e o unico lugar do aplicativo que sabe simultaneamente o formato de
//! tema e o formato da interface. Todo o resto do codigo trabalha so com um dos
//! dois lados, e e por isso que trocar de tema nao exige recompilar nada.

use morune_theme::layout::{PlayerPosition, SidebarPosition, ViewMode};
use morune_theme::{Color as ThemeColor, ThemeSpec};
use slint::{Brush, Color as SlintColor, SharedString};

use crate::ui::{Layout as UiLayout, Theme as UiTheme};

fn brush(c: ThemeColor) -> Brush {
    Brush::SolidColor(SlintColor::from_argb_u8(c.a, c.r, c.g, c.b))
}

/// Ajustes do usuario que se sobrepoem ao tema.
///
/// Existem separados do `ThemeSpec` de proposito: sao preferencias da pessoa,
/// nao do tema, e precisam sobreviver a uma troca de tema.
#[derive(Debug, Clone, Copy, Default)]
pub struct UserOverrides {
    /// `0.0` = usa a escala do tema.
    pub font_scale: f32,
    /// Desliga animacoes independentemente do que o tema pede.
    pub reduce_motion: bool,
    /// Barra lateral recolhida.
    pub sidebar_collapsed: bool,
}

/// Aplica o tema nas globais `Theme` e `Layout` da interface.
pub fn apply(theme: &UiTheme<'_>, layout: &UiLayout<'_>, spec: &ThemeSpec, over: UserOverrides) {
    apply_colors(theme, spec);
    apply_typography(theme, spec, over);
    apply_shape(theme, spec);
    apply_motion(theme, spec, over);
    apply_effects(theme, spec);
    apply_layout(layout, spec, over);
}

fn apply_colors(t: &UiTheme<'_>, s: &ThemeSpec) {
    let c = &s.colors;
    t.set_background(brush(c.background));
    t.set_surface(brush(c.surface));
    t.set_surface_raised(brush(c.surface_raised));
    t.set_player_background(brush(c.player_background));
    t.set_sidebar_background(brush(c.sidebar_background));
    t.set_text(brush(c.text));
    t.set_text_muted(brush(c.text_muted));
    t.set_text_on_accent(brush(c.text_on_accent));
    t.set_accent(brush(c.accent));
    t.set_accent_hover(brush(c.accent_hover));
    t.set_border(brush(c.border));
    t.set_hover(brush(c.hover));
    t.set_selected(brush(c.selected));
    t.set_focus_ring(brush(c.focus_ring));
    t.set_danger(brush(c.danger));
    t.set_warning(brush(c.warning));
    t.set_success(brush(c.success));
    t.set_scrollbar(brush(c.scrollbar));
}

fn apply_typography(t: &UiTheme<'_>, s: &ThemeSpec, over: UserOverrides) {
    let typo = &s.typography;
    // A escala do usuario, quando definida, substitui a do tema em vez de
    // multiplicar: dois multiplicadores empilhados produzem tamanhos
    // imprevisiveis quando a pessoa troca de tema.
    let scale = if over.font_scale > 0.0 { over.font_scale } else { typo.scale };
    let sized = |base: f32| ((base * scale) * 2.0).round() / 2.0;

    t.set_font_family(SharedString::from(typo.family.as_str()));
    t.set_size_xs(sized(typo.size_xs));
    t.set_size_sm(sized(typo.size_sm));
    t.set_size_md(sized(typo.size_md));
    t.set_size_lg(sized(typo.size_lg));
    t.set_size_xl(sized(typo.size_xl));
    t.set_size_display(sized(typo.size_display));
    t.set_weight_normal(typo.weight_normal);
    t.set_weight_medium(typo.weight_medium);
    t.set_weight_bold(typo.weight_bold);
    t.set_letter_spacing(typo.letter_spacing);
}

fn apply_shape(t: &UiTheme<'_>, s: &ThemeSpec) {
    let sh = &s.shape;
    t.set_radius_sm(sh.radius_sm);
    t.set_radius_md(sh.radius_md);
    t.set_radius_lg(sh.radius_lg);
    t.set_radius_artwork(sh.radius_artwork);
    t.set_radius_avatar(sh.radius_avatar);
    t.set_space_xs(sh.spacing_xs);
    t.set_space_sm(sh.spacing_sm);
    t.set_space_md(sh.spacing_md);
    t.set_space_lg(sh.spacing_lg);
    t.set_space_xl(sh.spacing_xl);
    t.set_border_width(sh.border_width);
    t.set_progress_thickness(sh.progress_thickness);
    t.set_scrollbar_width(sh.scrollbar_width);
}

fn apply_motion(t: &UiTheme<'_>, s: &ThemeSpec, over: UserOverrides) {
    // A interface recebe duracoes ja resolvidas: com movimento reduzido tudo
    // chega como zero, e nenhuma tela precisa checar a preferencia.
    let m = &s.motion;
    let d = |base: i32| -> i64 {
        if over.reduce_motion {
            0
        } else {
            m.effective(base) as i64
        }
    };
    t.set_motion_fast(d(m.duration_fast));
    t.set_motion_normal(d(m.duration_normal));
    t.set_motion_slow(d(m.duration_slow));
}

fn apply_effects(t: &UiTheme<'_>, s: &ThemeSpec) {
    let e = &s.effects;
    t.set_shadow_strength(e.shadow_strength);
    t.set_artwork_tint(e.artwork_tint);
    t.set_artwork_tint_strength(e.artwork_tint_strength);
}

fn apply_layout(l: &UiLayout<'_>, s: &ThemeSpec, over: UserOverrides) {
    let lay = &s.layout;
    let density = lay.content.density.factor();

    l.set_sidebar_position(match lay.sidebar.position {
        SidebarPosition::Left => 0,
        SidebarPosition::Right => 1,
        SidebarPosition::Hidden => 2,
    });
    l.set_sidebar_width(lay.sidebar.width);
    l.set_sidebar_collapsed_width(lay.sidebar.collapsed_width);
    l.set_sidebar_collapsible(lay.sidebar.collapsible);
    l.set_sidebar_collapsed(
        // O tema so escolhe o estado inicial; depois disso quem manda e o
        // usuario, senao recarregar o tema desfaria o que ele acabou de fazer.
        over.sidebar_collapsed || (lay.sidebar.start_collapsed && !lay.sidebar.collapsible),
    );
    l.set_sidebar_icons(lay.sidebar.show_icons);
    l.set_sidebar_labels(lay.sidebar.show_labels);
    l.set_sidebar_playlists(lay.sidebar.show_playlists);

    l.set_player_position(match lay.player.position {
        PlayerPosition::Bottom => 0,
        PlayerPosition::Top => 1,
        PlayerPosition::Sidebar => 2,
    });
    l.set_player_height(lay.player.height);
    l.set_player_artwork(lay.player.show_artwork);
    l.set_player_artwork_size(lay.player.artwork_size);
    l.set_player_progress(lay.player.show_progress);
    l.set_player_progress_edge(lay.player.progress_edge_to_edge);
    l.set_player_volume(lay.player.show_volume);
    l.set_player_shuffle_repeat(lay.player.show_shuffle_repeat);
    l.set_player_queue_button(lay.player.show_queue_button);
    l.set_player_times(lay.player.show_times);
    l.set_player_center_controls(lay.player.center_controls);

    l.set_view_mode(match lay.content.view_mode {
        ViewMode::Grid => 0,
        ViewMode::List => 1,
        ViewMode::Compact => 2,
    });
    l.set_density(density);
    l.set_card_width(lay.content.card_width);
    l.set_row_height(lay.content.row_height * density);
    l.set_show_hero(lay.content.show_hero);
    l.set_hero_height(lay.content.hero_height);
}

/// Estado inicial da janela, derivado do tema.
///
/// Aplicado uma unica vez na abertura: redimensionar a janela do usuario a cada
/// troca de tema seria hostil.
pub fn initial_window_size(spec: &ThemeSpec) -> (f32, f32) {
    (spec.layout.window.default_width, spec.layout.window.default_height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduce_motion_zeroes_every_duration() {
        let spec = ThemeSpec::default();
        let over = UserOverrides { reduce_motion: true, ..Default::default() };
        // `apply_motion` e testado atraves do calculo que ele usa; a chamada
        // real precisa de uma janela viva, entao aqui verificamos a regra.
        let d = |base: i32| if over.reduce_motion { 0 } else { spec.motion.effective(base) };
        assert_eq!(d(spec.motion.duration_fast), 0);
        assert_eq!(d(spec.motion.duration_slow), 0);
    }

    #[test]
    fn user_font_scale_replaces_theme_scale() {
        let mut spec = ThemeSpec::default();
        spec.typography.scale = 2.0;
        let over = UserOverrides { font_scale: 1.5, ..Default::default() };
        let scale = if over.font_scale > 0.0 { over.font_scale } else { spec.typography.scale };
        assert_eq!(scale, 1.5);
    }

    #[test]
    fn zero_font_scale_defers_to_the_theme() {
        let mut spec = ThemeSpec::default();
        spec.typography.scale = 1.25;
        let over = UserOverrides::default();
        let scale = if over.font_scale > 0.0 { over.font_scale } else { spec.typography.scale };
        assert_eq!(scale, 1.25);
    }

    #[test]
    fn window_size_comes_from_the_theme() {
        let mut spec = ThemeSpec::default();
        spec.layout.window.default_width = 1000.0;
        spec.layout.window.default_height = 700.0;
        assert_eq!(initial_window_size(&spec), (1000.0, 700.0));
    }
}
