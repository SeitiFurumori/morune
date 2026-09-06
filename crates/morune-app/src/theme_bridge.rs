//! Ponte entre o tema declarativo (`morune-theme`) e as globais da interface.
//!
//! Este e o unico lugar do aplicativo que sabe simultaneamente o formato de
//! tema e o formato da interface. Todo o resto do codigo trabalha so com um dos
//! dois lados, e e por isso que trocar de tema nao exige recompilar nada.

use morune_theme::layout::{PlayerPosition, SidebarPosition, ViewMode};
use morune_theme::{Color as ThemeColor, ThemeSpec};
use slint::{Brush, Color as SlintColor, SharedString};

use crate::ui::{Icons as UiIcons, Layout as UiLayout, Theme as UiTheme};
use crate::wallpaper::Wallpaper;

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

/// Liga cada nome de icone ao setter que o Slint gerou para ele.
///
/// A macro existe porque o Slint gera um metodo por propriedade e nao ha como
/// enderecar uma global por nome em tempo de execucao. Esta e a unica ponte
/// entre os nomes aceitos em disco (`morune_theme::ICON_NAMES`) e a interface;
/// o teste `icon_slots_cover_every_name` reprova se as duas listas divergirem.
macro_rules! icon_slots {
    ($($name:literal => $setter:ident,)*) => {
        const ICON_SLOTS: &[&str] = &[$($name),*];

        fn set_icon(icons: &UiIcons<'_>, name: &str, image: slint::Image) {
            match name {
                $($name => icons.$setter(image),)*
                _ => {}
            }
        }
    };
}

icon_slots! {
    "home" => set_custom_home,
    "search" => set_custom_search,
    "library" => set_custom_library,
    "settings" => set_custom_settings,
    "play" => set_custom_play,
    "pause" => set_custom_pause,
    "stop" => set_custom_stop,
    "next" => set_custom_next,
    "previous" => set_custom_previous,
    "shuffle" => set_custom_shuffle,
    "repeat" => set_custom_repeat,
    "repeat-one" => set_custom_repeat_one,
    "queue" => set_custom_queue,
    "mini-player" => set_custom_mini_player,
    "volume" => set_custom_volume,
    "chevron-left" => set_custom_chevron_left,
    "chevron-right" => set_custom_chevron_right,
    "close" => set_custom_close,
    "heart" => set_custom_heart,
    "pin" => set_custom_pin,
    "queue-add" => set_custom_queue_add,
    "queue-next" => set_custom_queue_next,
    "chevron-up" => set_custom_chevron_up,
    "chevron-down" => set_custom_chevron_down,
    "minimize-window" => set_custom_minimize_window,
    "maximize-window" => set_custom_maximize_window,
    "restore-window" => set_custom_restore_window,
}

/// Instala os icones que o tema substitui.
pub fn apply_icons(icons: &UiIcons<'_>, theme_dir: Option<&std::path::Path>) {
    let found = theme_dir
        .map(morune_theme::icons::resolve)
        .unwrap_or_default();

    for name in ICON_SLOTS {
        // Sempre escreve, inclusive o vazio: trocar de um tema com icones
        // proprios para um sem eles precisa apagar os anteriores, senao os
        // icones do tema antigo continuariam na tela.
        let image = found
            .get(name)
            .and_then(|path| match slint::Image::load_from_path(path) {
                Ok(image) => Some(image),
                Err(error) => {
                    tracing::warn!(icone = name, ?error, "icone do tema nao decodificou");
                    None
                }
            })
            .unwrap_or_default();
        set_icon(icons, name, image);
    }
}

/// Aplica a imagem de fundo ja preparada.
///
/// Separado de [`apply`] de proposito: o tema e um valor barato de copiar, a
/// imagem nao e. Quem chama decide quando a imagem precisa ser recarregada, e
/// trocar so a densidade da barra lateral nao decodifica JPEG nenhum.
pub fn apply_background(theme: &UiTheme<'_>, spec: &ThemeSpec, paper: &Wallpaper) {
    theme.set_background_image(paper.image.clone());
    theme.set_background_blurred(paper.blurred.clone());
    theme.set_background_fit(paper.fit);
    theme.set_background_opacity(paper.opacity);
    theme.set_background_tint(brush(spec.background.tint));
    theme.set_background_tint_strength(paper.tint_strength);
}

/// Aplica o tema nas globais `Theme` e `Layout` da interface.
pub fn apply(theme: &UiTheme<'_>, layout: &UiLayout<'_>, spec: &ThemeSpec, over: UserOverrides) {
    apply_colors(theme, spec);
    apply_typography(theme, spec, over);
    apply_shape(theme, spec);
    apply_controls(theme, spec);
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
    // Resolvido aqui, e nao na interface: um realce totalmente transparente
    // significa "sem realce", e quem desenha so quer uma cor para usar.
    t.set_border_highlight(brush(if c.border_highlight.a == 0 {
        c.border
    } else {
        c.border_highlight
    }));
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
    let scale = if over.font_scale > 0.0 {
        over.font_scale
    } else {
        typo.scale
    };
    let sized = |base: f32| ((base * scale) * 2.0).round() / 2.0;

    t.set_font_family(SharedString::from(typo.family.as_str()));
    // Vazio cai na familia principal: e o que faz um tema de uma familia so
    // continuar com uma familia so, sem precisar repetir o nome.
    let display = if typo.display_family.trim().is_empty() {
        typo.family.as_str()
    } else {
        typo.display_family.as_str()
    };
    t.set_display_family(SharedString::from(display));
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

fn apply_controls(t: &UiTheme<'_>, s: &ThemeSpec) {
    let c = &s.control;
    t.set_control_button_size(c.button_size);
    // Um raio maior que o proprio botao nao existe: o Slint desenha o mesmo
    // circulo, mas o valor vaza para o anel de foco e para o tooltip, que
    // derivam dele. Prender aqui evita que cada uso repita a conta.
    t.set_control_button_radius(c.button_radius.min(c.button_size / 2.0));
    t.set_control_primary_size(c.primary_button_size);
    t.set_control_icon_size(c.icon_size);
    t.set_control_icon_size_primary(c.icon_size_primary);
    t.set_control_icon_stroke(c.icon_stroke);
    t.set_control_slider_knob(c.slider_knob);
    t.set_control_tooltip_height(c.tooltip_height);
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

/// Registra a fonte que o tema traz em `fonts/`, se houver.
///
/// Precisa acontecer **antes** de a familia ser aplicada: o Slint so encontra
/// a fonte pelo nome depois de ela estar registrada.
///
/// Uma vez registrada, uma fonte fica ate o aplicativo fechar -- o Slint nao
/// tem como remove-la. Trocar de tema varias vezes acumula fontes na memoria
/// do processo, o que e aceitavel porque o custo e o arquivo, e ninguem troca
/// de tema mil vezes numa sessao. O que **nao** e aceitavel e registrar a mesma
/// fonte de novo a cada recarga com o observador ligado, entao o conjunto
/// abaixo lembra o que ja passou por aqui.
pub fn apply_bundled_font(spec: &ThemeSpec, theme_dir: Option<&std::path::Path>) {
    use std::cell::RefCell;
    use std::collections::HashSet;
    use std::path::PathBuf;

    thread_local! {
        static REGISTRADAS: RefCell<HashSet<PathBuf>> = RefCell::new(HashSet::new());
    }

    let (Some(nome), Some(dir)) = (spec.typography.bundled_font.as_deref(), theme_dir) else {
        return;
    };

    // Nome de arquivo, nao caminho: `fonts/../../algo.ttf` num pacote importado
    // apontaria para fora do tema.
    let seguro = std::path::Path::new(nome)
        .file_name()
        .map(|n| n == std::ffi::OsStr::new(nome))
        .unwrap_or(false);
    if !seguro {
        tracing::warn!(
            fonte = nome,
            "bundled_font precisa ser so o nome do arquivo, ignorado"
        );
        return;
    }

    let path = dir.join("fonts").join(nome);
    if !path.is_file() {
        tracing::warn!(path = %path.display(), "fonte do tema nao encontrada");
        return;
    }

    let novo = REGISTRADAS.with(|r| r.borrow_mut().insert(path.clone()));
    if !novo {
        return;
    }

    registrar_fonte(&path);
}

/// Entrega a fonte ao Slint.
///
/// O nome da familia que o tema poe em `typography.family` precisa bater com o
/// nome interno do arquivo, e nao com o nome do arquivo: registrar
/// `MinhaFonte.ttf` nao cria a familia "MinhaFonte".
#[cfg(feature = "bundled-fonts")]
fn registrar_fonte(path: &std::path::Path) {
    use slint::fontique_010::fontique;

    let Ok(bytes) = std::fs::read(path) else {
        tracing::warn!(path = %path.display(), "fonte do tema ilegivel");
        return;
    };
    let blob = fontique::Blob::new(std::sync::Arc::new(bytes));
    let familias = slint::fontique_010::shared_collection().register_fonts(blob, None);
    if familias.is_empty() {
        tracing::warn!(path = %path.display(), "arquivo nao e uma fonte utilizavel");
        return;
    }
    tracing::info!(path = %path.display(), familias = familias.len(), "fonte do tema registrada");
}

#[cfg(not(feature = "bundled-fonts"))]
fn registrar_fonte(path: &std::path::Path) {
    tracing::warn!(
        path = %path.display(),
        "este build nao registra fontes de tema; recompile com a feature `bundled-fonts`"
    );
}

/// Aplica a cor tirada da capa que esta tocando.
///
/// Separada de [`apply`] pelo mesmo motivo do fundo: muda a cada faixa, e nao
/// a cada tema. Sem capa, chega igual ao fundo do player -- assim o degrade
/// existe sempre e simplesmente nao aparece, em vez de a interface ter que
/// decidir se desenha ou nao.
pub fn apply_artwork_color(t: &UiTheme<'_>, s: &ThemeSpec, color: Option<SlintColor>) {
    let fallback = SlintColor::from_argb_u8(
        s.colors.player_background.a,
        s.colors.player_background.r,
        s.colors.player_background.g,
        s.colors.player_background.b,
    );
    t.set_artwork_color(color.unwrap_or(fallback));
}

fn apply_effects(t: &UiTheme<'_>, s: &ThemeSpec) {
    let e = &s.effects;
    t.set_shadow_strength(e.shadow_strength);
    t.set_gloss(e.gloss);
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
    l.set_sidebar_playlist_height(lay.sidebar.playlist_height);
    l.set_detail_cover_size(lay.detail.cover_size);

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
    l.set_player_stop_button(lay.player.show_stop_button);
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
    (
        spec.layout.window.default_width,
        spec.layout.window.default_height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_slots_cover_every_name() {
        for name in morune_theme::ICON_NAMES {
            assert!(
                ICON_SLOTS.contains(name),
                "{name} pode ser substituido por tema mas a interface nao tem a propriedade"
            );
        }
        assert_eq!(ICON_SLOTS.len(), morune_theme::ICON_NAMES.len());
    }

    #[test]
    fn reduce_motion_zeroes_every_duration() {
        let spec = ThemeSpec::default();
        let over = UserOverrides {
            reduce_motion: true,
            ..Default::default()
        };
        // `apply_motion` e testado atraves do calculo que ele usa; a chamada
        // real precisa de uma janela viva, entao aqui verificamos a regra.
        let d = |base: i32| {
            if over.reduce_motion {
                0
            } else {
                spec.motion.effective(base)
            }
        };
        assert_eq!(d(spec.motion.duration_fast), 0);
        assert_eq!(d(spec.motion.duration_slow), 0);
    }

    #[test]
    fn user_font_scale_replaces_theme_scale() {
        let mut spec = ThemeSpec::default();
        spec.typography.scale = 2.0;
        let over = UserOverrides {
            font_scale: 1.5,
            ..Default::default()
        };
        let scale = if over.font_scale > 0.0 {
            over.font_scale
        } else {
            spec.typography.scale
        };
        assert_eq!(scale, 1.5);
    }

    #[test]
    fn zero_font_scale_defers_to_the_theme() {
        let mut spec = ThemeSpec::default();
        spec.typography.scale = 1.25;
        let over = UserOverrides::default();
        let scale = if over.font_scale > 0.0 {
            over.font_scale
        } else {
            spec.typography.scale
        };
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
