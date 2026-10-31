//! GERADO por gera_vista.py a partir de `morune-app/ui/app.slint` (AppWindow).
//! E a ponte entre o motor e a tela web: o motor escreve aqui com os mesmos
//! `set_*` que usava na janela do Slint, e o estado inteiro vai para a tela em JSON.
#![allow(dead_code, clippy::all)]
use std::cell::RefCell;
use serde::Serialize;
pub use crate::vista_tipos::*;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct TrackRow {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub cover: Imagem,
    pub initial: String,
    pub hue: f32,
    pub cover_pending: bool,
    pub duration: String,
    pub playable: bool,
    pub playing: bool,
    pub favorite: bool,
    pub artist_id: String,
    pub album_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct CardItem {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub cover: Imagem,
    pub initial: String,
    pub hue: f32,
    pub cover_pending: bool,
    pub pinned: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct FeedShelf {
    pub title: String,
    pub items: Vec<CardItem>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ThemeItem {
    pub id: String,
    pub name: String,
    pub author: String,
    pub builtin: bool,
    pub active: bool,
    pub preview_background: Cor,
    pub preview_surface: Cor,
    pub preview_sidebar: Cor,
    pub preview_player: Cor,
    pub preview_accent: Cor,
    pub preview_text: Cor,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Diagnostic {
    pub level: String,
    pub field: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Estado {
    pub native_backdrop_active: bool,
    pub page: i32,
    pub status_message: String,
    pub undo_available: bool,
    pub retry_available: bool,
    pub logged_in: bool,
    pub account_name: String,
    pub account_initial: String,
    pub auth_url: String,
    pub account_avatar: Imagem,
    pub home_feed: Vec<FeedShelf>,
    pub library_playlists: Vec<CardItem>,
    pub home_made_for_you: Vec<CardItem>,
    pub home_stations: Vec<CardItem>,
    pub home_retrospectives: Vec<CardItem>,
    pub home_playlists: Vec<CardItem>,
    pub home_liked: Vec<TrackRow>,
    pub home_recent: Vec<TrackRow>,
    pub sleep_label: String,
    pub recent_searches: Vec<String>,
    pub search_kind: i32,
    pub home_loaded: bool,
    pub search_tracks: Vec<TrackRow>,
    pub search_items: Vec<CardItem>,
    pub search_query: String,
    pub searching: bool,
    pub library_items: Vec<CardItem>,
    pub library_loaded: bool,
    pub queue_manual_tracks: Vec<TrackRow>,
    pub queue_context_tracks: Vec<TrackRow>,
    pub queue_context_name: String,
    pub themes: Vec<ThemeItem>,
    pub diagnostics: Vec<Diagnostic>,
    pub now_title: String,
    pub now_id: String,
    pub now_cover: Imagem,
    pub now_cover_initial: String,
    pub now_cover_hue: f32,
    pub now_cover_pending: bool,
    pub sidebar_playlists: Vec<CardItem>,
    pub detail_title: String,
    pub detail_subtitle: String,
    pub detail_kind: String,
    pub detail_save_label: String,
    pub detail_cover: Imagem,
    pub detail_cover_initial: String,
    pub detail_cover_hue: f32,
    pub detail_cover_pending: bool,
    pub detail_tracks: Vec<TrackRow>,
    pub detail_items: Vec<CardItem>,
    pub detail_related: Vec<CardItem>,
    pub detail_sort: i32,
    pub detail_descending: bool,
    pub detail_filtered: bool,
    pub detail_has_more: bool,
    pub detail_loading: bool,
    pub detail_loaded_count: i32,
    pub now_artist: String,
    /// Primeiro artista da faixa atual, para o nome no player levar a pagina dele.
    pub now_artist_id: String,
    pub playing: bool,
    pub has_track: bool,
    pub now_favorite: bool,
    pub progress: f32,
    pub elapsed: String,
    pub total: String,
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: i32,
    pub dev_mode: bool,
    pub perf_text: String,
    pub close_to_tray: bool,
    pub track_notifications: bool,
    pub autoplay: bool,
    pub bitrate: i32,
    pub normalize: bool,
    pub mono: bool,
    pub output_devices: Vec<String>,
    pub output_device: String,
    pub start_with_windows: bool,
    pub shortcuts_visible: bool,
    pub mini_player: bool,
    pub has_background: bool,
    pub background_name: String,
    pub background_fit: i32,
    pub background_opacity: f32,
    pub background_tint: f32,
    pub background_blur: f32,
    pub glass_blur: f32,
    pub font_scale: f32,
    pub window_opacity: f32,
    pub reduce_motion: bool,
    pub hot_reload: bool,
    pub update_phase: i32,
    pub update_title: String,
    pub update_detail: String,
    pub update_progress: f32,
    pub library_filtering: bool,
    pub home_greeting: String,
    pub library_kind: i32,
    pub library_sort: i32,
    pub offline: bool,
    pub editable_playlist_open: bool,
    pub live_accent: Cor,
    pub artwork_color: Cor,
    pub fundo: Imagem,
    pub acoes_playlists: Vec<CardItem>,
}

/// O que o motor enxerga como "a janela". Os `set_*` so guardam o valor.
#[derive(Default)]
pub struct AppWindow {
    pub estado: RefCell<Estado>,
}

impl AppWindow {
    pub fn set_native_backdrop_active(&self, v: bool) { self.estado.borrow_mut().native_backdrop_active = v.into(); }
    pub fn get_native_backdrop_active(&self) -> bool { self.estado.borrow().native_backdrop_active.clone() }
    pub fn set_page(&self, v: i32) { self.estado.borrow_mut().page = v.into(); }
    pub fn get_page(&self) -> i32 { self.estado.borrow().page.clone() }
    pub fn set_status_message(&self, v: String) { self.estado.borrow_mut().status_message = v.into(); }
    pub fn get_status_message(&self) -> String { self.estado.borrow().status_message.clone() }
    pub fn set_undo_available(&self, v: bool) { self.estado.borrow_mut().undo_available = v.into(); }
    pub fn get_undo_available(&self) -> bool { self.estado.borrow().undo_available.clone() }
    pub fn set_retry_available(&self, v: bool) { self.estado.borrow_mut().retry_available = v.into(); }
    pub fn get_retry_available(&self) -> bool { self.estado.borrow().retry_available.clone() }
    pub fn set_logged_in(&self, v: bool) { self.estado.borrow_mut().logged_in = v.into(); }
    pub fn get_logged_in(&self) -> bool { self.estado.borrow().logged_in.clone() }
    pub fn set_account_name(&self, v: String) { self.estado.borrow_mut().account_name = v.into(); }
    pub fn get_account_name(&self) -> String { self.estado.borrow().account_name.clone() }
    pub fn set_account_initial(&self, v: String) { self.estado.borrow_mut().account_initial = v.into(); }
    pub fn get_account_initial(&self) -> String { self.estado.borrow().account_initial.clone() }
    pub fn set_auth_url(&self, v: String) { self.estado.borrow_mut().auth_url = v.into(); }
    pub fn get_auth_url(&self) -> String { self.estado.borrow().auth_url.clone() }
    pub fn set_account_avatar(&self, v: Imagem) { self.estado.borrow_mut().account_avatar = v.into(); }
    pub fn get_account_avatar(&self) -> Imagem { self.estado.borrow().account_avatar.clone() }
    pub fn set_home_feed(&self, v: Vec<FeedShelf>) { self.estado.borrow_mut().home_feed = v.into(); }
    pub fn get_home_feed(&self) -> Vec<FeedShelf> { self.estado.borrow().home_feed.clone() }
    pub fn set_library_playlists(&self, v: Vec<CardItem>) { self.estado.borrow_mut().library_playlists = v.into(); }
    pub fn get_library_playlists(&self) -> Vec<CardItem> { self.estado.borrow().library_playlists.clone() }
    pub fn set_home_made_for_you(&self, v: Vec<CardItem>) { self.estado.borrow_mut().home_made_for_you = v.into(); }
    pub fn get_home_made_for_you(&self) -> Vec<CardItem> { self.estado.borrow().home_made_for_you.clone() }
    pub fn set_home_stations(&self, v: Vec<CardItem>) { self.estado.borrow_mut().home_stations = v.into(); }
    pub fn get_home_stations(&self) -> Vec<CardItem> { self.estado.borrow().home_stations.clone() }
    pub fn set_home_retrospectives(&self, v: Vec<CardItem>) { self.estado.borrow_mut().home_retrospectives = v.into(); }
    pub fn get_home_retrospectives(&self) -> Vec<CardItem> { self.estado.borrow().home_retrospectives.clone() }
    pub fn set_home_playlists(&self, v: Vec<CardItem>) { self.estado.borrow_mut().home_playlists = v.into(); }
    pub fn get_home_playlists(&self) -> Vec<CardItem> { self.estado.borrow().home_playlists.clone() }
    pub fn set_home_liked(&self, v: Vec<TrackRow>) { self.estado.borrow_mut().home_liked = v.into(); }
    pub fn get_home_liked(&self) -> Vec<TrackRow> { self.estado.borrow().home_liked.clone() }
    pub fn set_home_recent(&self, v: Vec<TrackRow>) { self.estado.borrow_mut().home_recent = v.into(); }
    pub fn get_home_recent(&self) -> Vec<TrackRow> { self.estado.borrow().home_recent.clone() }
    pub fn set_sleep_label(&self, v: String) { self.estado.borrow_mut().sleep_label = v.into(); }
    pub fn get_sleep_label(&self) -> String { self.estado.borrow().sleep_label.clone() }
    pub fn set_recent_searches(&self, v: Vec<String>) { self.estado.borrow_mut().recent_searches = v.into(); }
    pub fn get_recent_searches(&self) -> Vec<String> { self.estado.borrow().recent_searches.clone() }
    pub fn set_search_kind(&self, v: i32) { self.estado.borrow_mut().search_kind = v.into(); }
    pub fn get_search_kind(&self) -> i32 { self.estado.borrow().search_kind.clone() }
    pub fn set_home_loaded(&self, v: bool) { self.estado.borrow_mut().home_loaded = v.into(); }
    pub fn get_home_loaded(&self) -> bool { self.estado.borrow().home_loaded.clone() }
    pub fn set_search_tracks(&self, v: Vec<TrackRow>) { self.estado.borrow_mut().search_tracks = v.into(); }
    pub fn get_search_tracks(&self) -> Vec<TrackRow> { self.estado.borrow().search_tracks.clone() }
    pub fn set_search_items(&self, v: Vec<CardItem>) { self.estado.borrow_mut().search_items = v.into(); }
    pub fn get_search_items(&self) -> Vec<CardItem> { self.estado.borrow().search_items.clone() }
    pub fn set_search_query(&self, v: String) { self.estado.borrow_mut().search_query = v.into(); }
    pub fn get_search_query(&self) -> String { self.estado.borrow().search_query.clone() }
    pub fn set_searching(&self, v: bool) { self.estado.borrow_mut().searching = v.into(); }
    pub fn get_searching(&self) -> bool { self.estado.borrow().searching.clone() }
    pub fn set_library_items(&self, v: Vec<CardItem>) { self.estado.borrow_mut().library_items = v.into(); }
    pub fn get_library_items(&self) -> Vec<CardItem> { self.estado.borrow().library_items.clone() }
    pub fn set_library_loaded(&self, v: bool) { self.estado.borrow_mut().library_loaded = v.into(); }
    pub fn get_library_loaded(&self) -> bool { self.estado.borrow().library_loaded.clone() }
    pub fn set_queue_manual_tracks(&self, v: Vec<TrackRow>) { self.estado.borrow_mut().queue_manual_tracks = v.into(); }
    pub fn get_queue_manual_tracks(&self) -> Vec<TrackRow> { self.estado.borrow().queue_manual_tracks.clone() }
    pub fn set_queue_context_tracks(&self, v: Vec<TrackRow>) { self.estado.borrow_mut().queue_context_tracks = v.into(); }
    pub fn get_queue_context_tracks(&self) -> Vec<TrackRow> { self.estado.borrow().queue_context_tracks.clone() }
    pub fn set_queue_context_name(&self, v: String) { self.estado.borrow_mut().queue_context_name = v.into(); }
    pub fn get_queue_context_name(&self) -> String { self.estado.borrow().queue_context_name.clone() }
    pub fn set_themes(&self, v: Vec<ThemeItem>) { self.estado.borrow_mut().themes = v.into(); }
    pub fn get_themes(&self) -> Vec<ThemeItem> { self.estado.borrow().themes.clone() }
    pub fn set_diagnostics(&self, v: Vec<Diagnostic>) { self.estado.borrow_mut().diagnostics = v.into(); }
    pub fn get_diagnostics(&self) -> Vec<Diagnostic> { self.estado.borrow().diagnostics.clone() }
    pub fn set_now_title(&self, v: String) { self.estado.borrow_mut().now_title = v.into(); }
    pub fn get_now_title(&self) -> String { self.estado.borrow().now_title.clone() }
    pub fn set_now_id(&self, v: String) { self.estado.borrow_mut().now_id = v.into(); }
    pub fn get_now_id(&self) -> String { self.estado.borrow().now_id.clone() }
    pub fn set_now_cover(&self, v: Imagem) { self.estado.borrow_mut().now_cover = v.into(); }
    pub fn get_now_cover(&self) -> Imagem { self.estado.borrow().now_cover.clone() }
    pub fn set_now_cover_initial(&self, v: String) { self.estado.borrow_mut().now_cover_initial = v.into(); }
    pub fn get_now_cover_initial(&self) -> String { self.estado.borrow().now_cover_initial.clone() }
    pub fn set_now_cover_hue(&self, v: f32) { self.estado.borrow_mut().now_cover_hue = v.into(); }
    pub fn get_now_cover_hue(&self) -> f32 { self.estado.borrow().now_cover_hue.clone() }
    pub fn set_now_cover_pending(&self, v: bool) { self.estado.borrow_mut().now_cover_pending = v.into(); }
    pub fn get_now_cover_pending(&self) -> bool { self.estado.borrow().now_cover_pending.clone() }
    pub fn set_sidebar_playlists(&self, v: Vec<CardItem>) { self.estado.borrow_mut().sidebar_playlists = v.into(); }
    pub fn get_sidebar_playlists(&self) -> Vec<CardItem> { self.estado.borrow().sidebar_playlists.clone() }
    pub fn set_detail_title(&self, v: String) { self.estado.borrow_mut().detail_title = v.into(); }
    pub fn get_detail_title(&self) -> String { self.estado.borrow().detail_title.clone() }
    pub fn set_detail_subtitle(&self, v: String) { self.estado.borrow_mut().detail_subtitle = v.into(); }
    pub fn get_detail_subtitle(&self) -> String { self.estado.borrow().detail_subtitle.clone() }
    pub fn set_detail_kind(&self, v: String) { self.estado.borrow_mut().detail_kind = v.into(); }
    pub fn get_detail_kind(&self) -> String { self.estado.borrow().detail_kind.clone() }
    pub fn set_detail_save_label(&self, v: String) { self.estado.borrow_mut().detail_save_label = v.into(); }
    pub fn get_detail_save_label(&self) -> String { self.estado.borrow().detail_save_label.clone() }
    pub fn set_detail_cover(&self, v: Imagem) { self.estado.borrow_mut().detail_cover = v.into(); }
    pub fn get_detail_cover(&self) -> Imagem { self.estado.borrow().detail_cover.clone() }
    pub fn set_detail_cover_initial(&self, v: String) { self.estado.borrow_mut().detail_cover_initial = v.into(); }
    pub fn get_detail_cover_initial(&self) -> String { self.estado.borrow().detail_cover_initial.clone() }
    pub fn set_detail_cover_hue(&self, v: f32) { self.estado.borrow_mut().detail_cover_hue = v.into(); }
    pub fn get_detail_cover_hue(&self) -> f32 { self.estado.borrow().detail_cover_hue.clone() }
    pub fn set_detail_cover_pending(&self, v: bool) { self.estado.borrow_mut().detail_cover_pending = v.into(); }
    pub fn get_detail_cover_pending(&self) -> bool { self.estado.borrow().detail_cover_pending.clone() }
    pub fn set_detail_tracks(&self, v: Vec<TrackRow>) { self.estado.borrow_mut().detail_tracks = v.into(); }
    pub fn get_detail_tracks(&self) -> Vec<TrackRow> { self.estado.borrow().detail_tracks.clone() }
    pub fn set_detail_items(&self, v: Vec<CardItem>) { self.estado.borrow_mut().detail_items = v.into(); }
    pub fn get_detail_items(&self) -> Vec<CardItem> { self.estado.borrow().detail_items.clone() }
    pub fn set_detail_related(&self, v: Vec<CardItem>) { self.estado.borrow_mut().detail_related = v.into(); }
    pub fn get_detail_related(&self) -> Vec<CardItem> { self.estado.borrow().detail_related.clone() }
    pub fn set_detail_sort(&self, v: i32) { self.estado.borrow_mut().detail_sort = v.into(); }
    pub fn get_detail_sort(&self) -> i32 { self.estado.borrow().detail_sort.clone() }
    pub fn set_detail_descending(&self, v: bool) { self.estado.borrow_mut().detail_descending = v.into(); }
    pub fn get_detail_descending(&self) -> bool { self.estado.borrow().detail_descending.clone() }
    pub fn set_detail_filtered(&self, v: bool) { self.estado.borrow_mut().detail_filtered = v.into(); }
    pub fn get_detail_filtered(&self) -> bool { self.estado.borrow().detail_filtered.clone() }
    pub fn set_detail_has_more(&self, v: bool) { self.estado.borrow_mut().detail_has_more = v.into(); }
    pub fn get_detail_has_more(&self) -> bool { self.estado.borrow().detail_has_more.clone() }
    pub fn set_detail_loading(&self, v: bool) { self.estado.borrow_mut().detail_loading = v.into(); }
    pub fn get_detail_loading(&self) -> bool { self.estado.borrow().detail_loading.clone() }
    pub fn set_detail_loaded_count(&self, v: i32) { self.estado.borrow_mut().detail_loaded_count = v.into(); }
    pub fn get_detail_loaded_count(&self) -> i32 { self.estado.borrow().detail_loaded_count.clone() }
    pub fn set_now_artist(&self, v: String) { self.estado.borrow_mut().now_artist = v.into(); }
    pub fn get_now_artist(&self) -> String { self.estado.borrow().now_artist.clone() }
    pub fn set_now_artist_id(&self, v: String) { self.estado.borrow_mut().now_artist_id = v; }
    pub fn set_playing(&self, v: bool) { self.estado.borrow_mut().playing = v.into(); }
    pub fn get_playing(&self) -> bool { self.estado.borrow().playing.clone() }
    pub fn set_has_track(&self, v: bool) { self.estado.borrow_mut().has_track = v.into(); }
    pub fn get_has_track(&self) -> bool { self.estado.borrow().has_track.clone() }
    pub fn set_now_favorite(&self, v: bool) { self.estado.borrow_mut().now_favorite = v.into(); }
    pub fn get_now_favorite(&self) -> bool { self.estado.borrow().now_favorite.clone() }
    pub fn set_progress(&self, v: f32) { self.estado.borrow_mut().progress = v.into(); }
    pub fn get_progress(&self) -> f32 { self.estado.borrow().progress.clone() }
    pub fn set_elapsed(&self, v: String) { self.estado.borrow_mut().elapsed = v.into(); }
    pub fn get_elapsed(&self) -> String { self.estado.borrow().elapsed.clone() }
    pub fn set_total(&self, v: String) { self.estado.borrow_mut().total = v.into(); }
    pub fn get_total(&self) -> String { self.estado.borrow().total.clone() }
    pub fn set_volume(&self, v: f32) { self.estado.borrow_mut().volume = v.into(); }
    pub fn get_volume(&self) -> f32 { self.estado.borrow().volume.clone() }
    pub fn set_shuffle(&self, v: bool) { self.estado.borrow_mut().shuffle = v.into(); }
    pub fn get_shuffle(&self) -> bool { self.estado.borrow().shuffle.clone() }
    pub fn set_repeat(&self, v: i32) { self.estado.borrow_mut().repeat = v.into(); }
    pub fn get_repeat(&self) -> i32 { self.estado.borrow().repeat.clone() }
    pub fn set_dev_mode(&self, v: bool) { self.estado.borrow_mut().dev_mode = v.into(); }
    pub fn get_dev_mode(&self) -> bool { self.estado.borrow().dev_mode.clone() }
    pub fn set_perf_text(&self, v: String) { self.estado.borrow_mut().perf_text = v.into(); }
    pub fn get_perf_text(&self) -> String { self.estado.borrow().perf_text.clone() }
    pub fn set_close_to_tray(&self, v: bool) { self.estado.borrow_mut().close_to_tray = v.into(); }
    pub fn get_close_to_tray(&self) -> bool { self.estado.borrow().close_to_tray.clone() }
    pub fn set_track_notifications(&self, v: bool) { self.estado.borrow_mut().track_notifications = v.into(); }
    pub fn get_track_notifications(&self) -> bool { self.estado.borrow().track_notifications.clone() }
    pub fn set_autoplay(&self, v: bool) { self.estado.borrow_mut().autoplay = v.into(); }
    pub fn get_autoplay(&self) -> bool { self.estado.borrow().autoplay.clone() }
    pub fn set_bitrate(&self, v: i32) { self.estado.borrow_mut().bitrate = v.into(); }
    pub fn get_bitrate(&self) -> i32 { self.estado.borrow().bitrate.clone() }
    pub fn set_normalize(&self, v: bool) { self.estado.borrow_mut().normalize = v.into(); }
    pub fn get_normalize(&self) -> bool { self.estado.borrow().normalize.clone() }
    pub fn set_mono(&self, v: bool) { self.estado.borrow_mut().mono = v.into(); }
    pub fn get_mono(&self) -> bool { self.estado.borrow().mono.clone() }
    pub fn set_output_devices(&self, v: Vec<String>) { self.estado.borrow_mut().output_devices = v.into(); }
    pub fn get_output_devices(&self) -> Vec<String> { self.estado.borrow().output_devices.clone() }
    pub fn set_output_device(&self, v: String) { self.estado.borrow_mut().output_device = v.into(); }
    pub fn get_output_device(&self) -> String { self.estado.borrow().output_device.clone() }
    pub fn set_start_with_windows(&self, v: bool) { self.estado.borrow_mut().start_with_windows = v.into(); }
    pub fn get_start_with_windows(&self) -> bool { self.estado.borrow().start_with_windows.clone() }
    pub fn set_shortcuts_visible(&self, v: bool) { self.estado.borrow_mut().shortcuts_visible = v.into(); }
    pub fn get_shortcuts_visible(&self) -> bool { self.estado.borrow().shortcuts_visible.clone() }
    pub fn set_mini_player(&self, v: bool) { self.estado.borrow_mut().mini_player = v.into(); }
    pub fn get_mini_player(&self) -> bool { self.estado.borrow().mini_player.clone() }
    pub fn set_has_background(&self, v: bool) { self.estado.borrow_mut().has_background = v.into(); }
    pub fn get_has_background(&self) -> bool { self.estado.borrow().has_background.clone() }
    pub fn set_background_name(&self, v: String) { self.estado.borrow_mut().background_name = v.into(); }
    pub fn get_background_name(&self) -> String { self.estado.borrow().background_name.clone() }
    pub fn set_background_fit(&self, v: i32) { self.estado.borrow_mut().background_fit = v.into(); }
    pub fn get_background_fit(&self) -> i32 { self.estado.borrow().background_fit.clone() }
    pub fn set_background_opacity(&self, v: f32) { self.estado.borrow_mut().background_opacity = v.into(); }
    pub fn get_background_opacity(&self) -> f32 { self.estado.borrow().background_opacity.clone() }
    pub fn set_background_tint(&self, v: f32) { self.estado.borrow_mut().background_tint = v.into(); }
    pub fn get_background_tint(&self) -> f32 { self.estado.borrow().background_tint.clone() }
    pub fn set_background_blur(&self, v: f32) { self.estado.borrow_mut().background_blur = v.into(); }
    pub fn get_background_blur(&self) -> f32 { self.estado.borrow().background_blur.clone() }
    pub fn set_glass_blur(&self, v: f32) { self.estado.borrow_mut().glass_blur = v.into(); }
    pub fn get_glass_blur(&self) -> f32 { self.estado.borrow().glass_blur.clone() }
    pub fn set_font_scale(&self, v: f32) { self.estado.borrow_mut().font_scale = v.into(); }
    pub fn get_font_scale(&self) -> f32 { self.estado.borrow().font_scale.clone() }
    pub fn set_window_opacity(&self, v: f32) { self.estado.borrow_mut().window_opacity = v.into(); }
    pub fn get_window_opacity(&self) -> f32 { self.estado.borrow().window_opacity.clone() }
    pub fn set_reduce_motion(&self, v: bool) { self.estado.borrow_mut().reduce_motion = v.into(); }
    pub fn get_reduce_motion(&self) -> bool { self.estado.borrow().reduce_motion.clone() }
    pub fn set_hot_reload(&self, v: bool) { self.estado.borrow_mut().hot_reload = v.into(); }
    pub fn get_hot_reload(&self) -> bool { self.estado.borrow().hot_reload.clone() }
    pub fn set_update_phase(&self, v: i32) { self.estado.borrow_mut().update_phase = v.into(); }
    pub fn get_update_phase(&self) -> i32 { self.estado.borrow().update_phase.clone() }
    pub fn set_update_title(&self, v: String) { self.estado.borrow_mut().update_title = v.into(); }
    pub fn get_update_title(&self) -> String { self.estado.borrow().update_title.clone() }
    pub fn set_update_detail(&self, v: String) { self.estado.borrow_mut().update_detail = v.into(); }
    pub fn get_update_detail(&self) -> String { self.estado.borrow().update_detail.clone() }
    pub fn set_update_progress(&self, v: f32) { self.estado.borrow_mut().update_progress = v.into(); }
    pub fn get_update_progress(&self) -> f32 { self.estado.borrow().update_progress.clone() }
    pub fn set_library_filtering(&self, v: bool) { self.estado.borrow_mut().library_filtering = v.into(); }
    pub fn get_library_filtering(&self) -> bool { self.estado.borrow().library_filtering.clone() }
    pub fn set_home_greeting(&self, v: String) { self.estado.borrow_mut().home_greeting = v.into(); }
    pub fn get_home_greeting(&self) -> String { self.estado.borrow().home_greeting.clone() }
    pub fn set_library_kind(&self, v: i32) { self.estado.borrow_mut().library_kind = v.into(); }
    pub fn get_library_kind(&self) -> i32 { self.estado.borrow().library_kind.clone() }
    pub fn set_library_sort(&self, v: i32) { self.estado.borrow_mut().library_sort = v.into(); }
    pub fn get_library_sort(&self) -> i32 { self.estado.borrow().library_sort.clone() }
    pub fn set_offline(&self, v: bool) { self.estado.borrow_mut().offline = v.into(); }
    pub fn get_offline(&self) -> bool { self.estado.borrow().offline.clone() }
    pub fn set_editable_playlist_open(&self, v: bool) { self.estado.borrow_mut().editable_playlist_open = v.into(); }
    pub fn get_editable_playlist_open(&self) -> bool { self.estado.borrow().editable_playlist_open.clone() }
    pub fn set_live_accent(&self, v: Cor) { self.estado.borrow_mut().live_accent = v.into(); }
    pub fn get_live_accent(&self) -> Cor { self.estado.borrow().live_accent.clone() }
    pub fn set_artwork_color(&self, v: Cor) { self.estado.borrow_mut().artwork_color = v.into(); }
    pub fn get_artwork_color(&self) -> Cor { self.estado.borrow().artwork_color.clone() }
    pub fn set_fundo(&self, v: Imagem) { self.estado.borrow_mut().fundo = v.into(); }
    pub fn get_fundo(&self) -> Imagem { self.estado.borrow().fundo.clone() }
    pub fn set_acoes_playlists(&self, v: Vec<CardItem>) { self.estado.borrow_mut().acoes_playlists = v.into(); }
    pub fn get_acoes_playlists(&self) -> Vec<CardItem> { self.estado.borrow().acoes_playlists.clone() }
}

/// Acoes que a tela pode pedir (os antigos callbacks do Slint).
pub const ACOES: &[&str] = &[
    "set-sleep", // (i32)
    "clear-recent-searches", // ()
    "set-search-kind", // (i32)
    "toggle-hidden", // (String)
    "toggle-detail-save", // ()
    "navigate", // (i32)
    "search", // (String)
    "play-track", // (String)
    "choose-background", // ()
    "clear-background", // ()
    "set-background-fit", // (i32)
    "set-background-opacity", // (f32)
    "set-background-tint", // (f32)
    "set-background-blur", // (f32)
    "set-glass-blur", // (f32)
    "set-font-scale", // (f32)
    "set-window-opacity", // (f32)
    "set-reduce-motion", // (bool)
    "set-hot-reload", // (bool)
    "check-update", // ()
    "download-update", // ()
    "install-update", // ()
    "open-releases-page", // ()
    "toggle-play", // ()
    "stop-playback", // ()
    "next-track", // ()
    "previous-track", // ()
    "seek", // (f32)
    "seek-by", // (f32)
    "zoom", // (i32)
    "set-volume", // (f32)
    "toggle-mute", // ()
    "nudge-volume", // (f32)
    "toggle-shuffle", // ()
    "cycle-repeat", // ()
    "toggle-favorite", // (String)
    "queue-play-next", // (String)
    "queue-enqueue", // (String)
    "queue-remove", // (i32)
    "queue-play-manual", // (i32)
    "queue-move", // (i32, i32)
    "queue-clear", // ()
    "select-theme", // (String)
    "import-theme", // ()
    "export-theme", // (String)
    "duplicate-theme", // (String)
    "reset-theme", // ()
    "open-theme-folder", // ()
    "reload-theme", // ()
    "toggle-sidebar", // ()
    "filter-playlists", // (String)
    "filter-library", // (String)
    "set-library-kind", // (i32)
    "set-library-sort", // (i32)
    "toggle-pin-playlist", // (String)
    "detail-back", // ()
    "close-queue", // ()
    "detail-play", // ()
    "detail-filter", // (String)
    "detail-sort-by", // (i32)
    "detail-activate", // (String)
    "detail-load-more", // ()
    "set-close-to-tray", // (bool)
    "set-track-notifications", // (bool)
    "set-autoplay", // (bool)
    "set-bitrate", // (i32)
    "set-normalize", // (bool)
    "set-mono", // (bool)
    "clear-cache", // ()
    "set-output-device", // (String)
    "set-start-with-windows", // (bool)
    "login", // ()
    "cancel-login", // ()
    "copy-auth-url", // ()
    "logout", // ()
    "dismiss-status", // ()
    "undo-last", // ()
    "retry-last", // ()
    "dismiss-undo", // ()
    "move-window", // (f32, f32)
    "toggle-mini-player", // ()
];
