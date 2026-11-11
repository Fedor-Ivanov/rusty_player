//! Player window: app state (`PlayerApp`), the frame and panel layout.
//! The panels, pages and dialogs themselves are in `panels`, `pages`, `dialogs`: each file there is
//! an `impl PlayerApp` with its own part of the UI.

use eframe::egui::{self, Ui};
use std::{
    collections::VecDeque,
    path::PathBuf,
    time::{Duration, Instant},
};

use super::{
    action::Action,
    components::{self, CoverFade, TileGridView, TrackListView},
    covers::CoverCache,
    dialogs::{playlist::NameDialog, remove::LibraryRemoval},
    file_drop, i18n,
    loading::{Destination, PendingLoad},
    navigation::Route,
    pages::LibrarySearch,
    style::theme::window_tint,
    system_media::SystemMedia,
};
use crate::core::{
    Core,
    library::{loader::Source, metadata::FileMetadata},
    lyrics::TrackLyrics,
    playback::PlayerState,
    playlists::PlaylistId,
};

// Icon buttons in the library header and in the list's bottom block
pub(crate) const ADD_BUTTON_SIZE: f32 = 24.0;
// Side panel width (library and track list): resize limits and initial width
pub(crate) const SIDE_PANEL_MIN_WIDTH: f32 = 180.0;
pub(crate) const SIDE_PANEL_MAX_WIDTH: f32 = 420.0;
pub(crate) const SIDEBAR_DEFAULT_WIDTH: f32 = 240.0;
pub(crate) const TRACK_LIST_DEFAULT_WIDTH: f32 = 300.0;
// Minimum width of the central area (CentralPanel): ensured by the side panels' maximums
pub(crate) const CONTENT_MIN_WIDTH: f32 = 260.0;
// Gap above the current track bar
pub(crate) const NOW_PLAYING_BAR_GAP: f32 = 8.0;

pub struct PlayerApp {
    /// App data: library, saved playlists, player with queue, settings. Changed only
    /// in `action::apply`
    pub(crate) core: Core,
    /// Actions sent during the frame (`dispatch`): performed at its end — see `action`
    pub(crate) actions: Vec<Action>,
    pub(crate) error: Option<String>,
    /// Slider position (in seconds) while the user drags it
    pub(crate) drag_secs: Option<f32>,
    /// Background track adds. Processed in turn: the second one's tracks appear
    /// in the list after the first one's, even if it finishes earlier
    pub(crate) loaders: VecDeque<PendingLoad>,
    /// Since when the current adding's progress is visible; `None` — not visible
    pub(crate) loader_shown_at: Option<Instant>,
    /// Whether the settings window is open
    pub(crate) settings_open: bool,
    pub(crate) library_view: TrackListView,
    pub(crate) queue_view: TrackListView,
    /// Which list receives keys (⌘A, Esc, Delete) — the one used last
    pub(crate) focus: ListFocus,
    /// What's open in the central area. Changed only via `navigate`
    pub(crate) route: Route,
    /// Track list of the open album
    pub(crate) album_tracks_view: TrackListView,
    /// Library grids — each with its own selection and scroll
    pub(crate) artists_grid: TileGridView,
    pub(crate) albums_grid: TileGridView,
    pub(crate) artist_albums_grid: TileGridView,
    /// Search in the open library view
    pub(crate) search: LibrarySearch,
    /// An album or artist whose removal from the library awaits confirmation in the dialog
    pub(crate) pending_removal: Option<LibraryRemoval>,
    /// The playlist name dialog, while open
    pub(crate) name_dialog: Option<NameDialog>,
    /// Playlists whose deletion awaits confirmation; empty — not waiting
    pub(crate) pending_playlist_deletion: Vec<PlaylistId>,
    /// The saved playlists grid and the open playlist's track list
    pub(crate) playlists_grid: TileGridView,
    pub(crate) playlist_tracks_view: TrackListView,
    /// Cover thumbnails for the albums grid
    pub(crate) covers: CoverCache,
    /// Current track cover and backdrop: smooth transition when switching tracks
    pub(crate) cover_fade: CoverFade,
    pub(crate) backdrop_fade: CoverFade,
    /// Current track tags from the file — for details: genre and bitrate aren't stored in the track.
    /// Read once on track change; `None` inside — the file couldn't be read
    pub(crate) current_details: Option<(PathBuf, Option<FileMetadata>)>,
    /// Whether lyrics are shown instead of the cover on the current track page
    pub(crate) lyrics_open: bool,
    /// Current track lyrics — looked up on track change while lyrics are shown, and again
    /// when an LRCLIB answer arrives
    pub(crate) current_lyrics: Option<(PathBuf, TrackLyrics)>,
    /// The line the lyrics were last scrolled to: scroll only when the current
    /// line changed — otherwise the user couldn't scroll the lyrics themselves
    pub(crate) lyrics_line: Option<usize>,
    /// System media keys and reporting the current track to the system
    pub(crate) system_media: SystemMedia,
    /// Where the cursor is while files are dragged over the window (see `file_drop`)
    pub(crate) drop_cursor: file_drop::Cursor,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ListFocus {
    Library,
    Queue,
    /// Artists or albums grid
    Grid,
}

impl PlayerApp {
    /// The egui context is taken from `cc` — so the audio thread can wake the app
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let (core, error) = Core::load(eframe::storage_dir(crate::APP_NAME), {
            // Track finished — wake the app: eframe calls `logic` even with a hidden window
            let ctx = cc.egui_ctx.clone();
            move || ctx.request_repaint()
        });
        i18n::set_language(core.settings.language());
        let outdated = core.library.outdated_paths();
        // No last track (first launch or it was absent at exit) — Now Playing has nothing
        // to show, open albums
        let route = if core.controller.player().current_track.is_none() {
            Route::Albums
        } else {
            Route::NowPlaying
        };
        let mut app = Self {
            core,
            actions: Vec::new(),
            error,
            drag_secs: None,
            loaders: VecDeque::new(),
            loader_shown_at: None,
            settings_open: false,
            library_view: TrackListView::new("library"),
            queue_view: TrackListView::new("playlist"),
            focus: ListFocus::Queue,
            route,
            album_tracks_view: TrackListView::new("album_tracks"),
            artists_grid: TileGridView::new("artists"),
            albums_grid: TileGridView::new("albums"),
            artist_albums_grid: TileGridView::new("artist_albums"),
            search: LibrarySearch::default(),
            pending_removal: None,
            name_dialog: None,
            pending_playlist_deletion: Vec::new(),
            playlists_grid: TileGridView::new("playlists"),
            playlist_tracks_view: TrackListView::new("playlist_tracks"),
            covers: CoverCache::new(),
            cover_fade: CoverFade::default(),
            backdrop_fade: CoverFade::default(),
            current_details: None,
            lyrics_open: false,
            current_lyrics: None,
            lyrics_line: None,
            system_media: SystemMedia::new(cc),
            drop_cursor: file_drop::Cursor::new(cc),
        };
        // The library was saved in an old format — re-read its tags once in the background
        if let Some(paths) = outdated {
            app.start_loading(Source::Files(paths), Destination::LibraryRefresh);
        }
        app
    }

    /// Saves the queue and settings if they changed. Called at the end of `logic`
    /// (changes without a window: next track) and at the end of `ui` (changes from the UI).
    /// While a mouse button is held, the gesture isn't finished (e.g. the volume slider is being dragged) —
    /// settings are written once, when the button is released
    pub(crate) fn save_if_changed(&mut self, ctx: &egui::Context) {
        let result = self.core.save_queue_if_changed();
        self.report_error(result);
        if !ctx.input(|input| input.pointer.any_down()) {
            let result = self.core.save_settings_if_changed();
            self.report_error(result);
        }
    }

    /// Reports an error if `result` is an error: it appears on the banner. `Ok` does
    /// nothing and doesn't clear an already shown error: the banner closes only with the
    /// "OK" button, otherwise the error would disappear by itself on the next successful action
    pub(crate) fn report_error(&mut self, result: Result<(), String>) {
        if let Err(e) = result {
            self.error = Some(e);
        }
    }
}

impl eframe::App for PlayerApp {
    /// Exit: the library is written in the background — wait so the latest changes are saved.
    /// With OpenGL (on Windows, see `window_glass::renderer`) eframe passes its context here
    #[cfg(not(target_os = "windows"))]
    fn on_exit(&mut self) {
        self.core.flush();
    }

    #[cfg(target_os = "windows")]
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.core.flush();
    }

    /// The window is transparent: the system draws a blur under it, so we don't fill anything ourselves
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    /// Everything that must work without the UI too: eframe calls this before every `ui`,
    /// and when the window is hidden (another desktop, minimized, covered by other windows) and `ui`
    /// isn't called at all — on `request_repaint`. This way the next track starts even
    /// when the window isn't visible: the audio thread wakes the app when a track finishes
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Track finished — move to the next one in the queue
        if self.core.controller.poll_events() {
            self.dispatch(Action::TrackEnded);
        }
        // Media keys are here too: they work even when the window isn't visible
        self.poll_system_media();
        // Lyrics arrived from LRCLIB — look up the current track's lyrics again
        if self.core.lyrics.poll() {
            self.current_lyrics = None;
        }
        // Library: the background missing-files check answer, background write errors
        if let Some(e) = self.core.poll_background() {
            self.report_error(Err(e));
        }
        self.poll_loaders(ctx);
        self.apply_actions(ctx);
        self.sync_system_media();
        self.save_if_changed(ctx);
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ui.ctx());
        self.covers.update(ui.ctx());

        // While a track plays, update the time and the slider. This repaints the visible window;
        // moving to the next track doesn't depend on it — see `logic`
        if self.core.controller.player().state == PlayerState::Playing {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }

        // System blur tint — first, under all panels. Panels have no other
        // fills: the glass is the same color across the whole window
        let tint = window_tint(self.core.settings.glass());
        ui.painter().rect_filled(ui.max_rect(), 0, tint);
        // A slot for the Now Playing backdrop — right above the tint, under all panels. The backdrop itself
        // is known only once we reach the central area, but it also extends under the controls
        // panel drawn earlier — so the slot is reserved in advance
        let root_painter = ui.painter().clone();
        let backdrop_slot = root_painter.add(egui::Shape::Noop);

        // CentralPanel has no minimum width of its own — it gets what's left after the side
        // panels. So the minimum is ensured by their maximums: each can take only what
        // remains after the minimums of everything not yet placed. When the window narrows, the side
        // panels shrink first, and the central one doesn't get narrower than CONTENT_MIN_WIDTH
        let sidebar_max_width = (ui.available_width() - SIDE_PANEL_MIN_WIDTH - CONTENT_MIN_WIDTH)
            .clamp(SIDE_PANEL_MIN_WIDTH, SIDE_PANEL_MAX_WIDTH);

        // egui highlights the panel edge with a white line on hover and drag —
        // taking the width from the parent Ui's hovered/active fg_stroke. Instead we have our own
        // line in the UI's tone (paint_resize_line), so for the side panels
        // the width is zeroed. Only the width changes: widget text color inside is unaffected
        let widgets = &mut ui.style_mut().visuals.widgets;
        let edge_strokes = (
            widgets.hovered.fg_stroke.width,
            widgets.active.fg_stroke.width,
        );
        widgets.hovered.fg_stroke.width = 0.0;
        widgets.active.fg_stroke.width = 0.0;

        // Library — at the left edge of the window
        let sidebar = egui::Panel::left("library_sidebar")
            .frame(egui::Frame::new())
            .default_size(SIDEBAR_DEFAULT_WIDTH)
            .size_range(SIDE_PANEL_MIN_WIDTH..=sidebar_max_width)
            .resizable(true)
            .show_separator_line(false)
            .show(ui, |ui| self.render_sidebar(ui));

        // Computed after the side panel: available_width already excludes its width
        let track_list_max_width = (ui.available_width() - CONTENT_MIN_WIDTH)
            .clamp(SIDE_PANEL_MIN_WIDTH, SIDE_PANEL_MAX_WIDTH);

        // Track list — at the right edge of the window
        let track_list = egui::Panel::right("track_list")
            .frame(egui::Frame::new())
            .default_size(TRACK_LIST_DEFAULT_WIDTH)
            .size_range(SIDE_PANEL_MIN_WIDTH..=track_list_max_width)
            .resizable(true)
            .show_separator_line(false)
            .show(ui, |ui| {
                // Panel fits its width to the content, so without this it shrinks
                // back to the minimum right after the edge is dragged
                ui.set_min_width(ui.available_width());

                // The block is pinned to the bottom of the panel. Added before the list: the panel takes
                // its space right away, and the ScrollArea gets only the rest and doesn't go under the block
                egui::Panel::bottom("track_list_bottom")
                    // Without its own frame the panel would fill itself with the default background
                    .frame(egui::Frame::new().inner_margin(16.0))
                    .resizable(false)
                    .show_separator_line(false)
                    .show(ui, |ui| {
                        self.render_playback_modes(ui);
                    });

                self.render_queue(ui);
            });

        let widgets = &mut ui.style_mut().visuals.widgets;
        (
            widgets.hovered.fg_stroke.width,
            widgets.active.fg_stroke.width,
        ) = edge_strokes;

        // Panels must be added before CentralPanel — it takes all remaining space.
        // Added after the side panels, so it stretches only under the central card
        egui::Panel::bottom("audio_controls")
            .frame(egui::Frame::new())
            .show_separator_line(false)
            .show(ui, |ui| {
                // No glass: the controls sit directly on the window background
                egui::Frame::new().inner_margin(16.0).show(ui, |ui| {
                    self.render_controls(ui);
                });
            });

        // Central area without its own background: the tinted system blur is under it
        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show(ui, |ui| {
                // A library view is open and there's a track — a bar with it at the bottom; clicking it
                // returns to the track details. Added before the content: the panel takes
                // its space right away, and the content doesn't go under it.
                // Appears and hides by sliding from the bottom: show_collapsible animates this itself,
                // and the content above the bar moves along with it. The height is set exactly —
                // otherwise the panel doesn't know it in the first frame and jumps
                let mut show_bar = self.route != Route::NowPlaying
                    && self.core.controller.player().current_track.is_some();
                egui::Panel::bottom("now_playing_bar")
                    .frame(egui::Frame::new().inner_margin(egui::Margin {
                        left: 16,
                        right: 16,
                        top: NOW_PLAYING_BAR_GAP as i8,
                        bottom: 0,
                    }))
                    .exact_size(components::NOW_PLAYING_BAR_HEIGHT + NOW_PLAYING_BAR_GAP)
                    .resizable(false)
                    .drag_to_open(false)
                    .show_separator_line(false)
                    .show_collapsible(ui, &mut show_bar, |ui| {
                        if self.render_now_playing_bar(ui) {
                            self.dispatch(Action::Navigate(Route::NowPlaying));
                        }
                    });

                match self.route {
                    Route::NowPlaying => {
                        if let Some(backdrop) = self.track_backdrop(ui) {
                            root_painter.set(backdrop_slot, backdrop);
                        }
                        egui::Frame::new().inner_margin(16.0).show(ui, |ui| {
                            ui.vertical_centered_justified(|ui| {
                                if self.core.controller.player().track_list.tracks.is_empty() {
                                    self.render_add_tracks_block(ui);
                                } else {
                                    self.render_current_track_info(ui);
                                }
                            })
                        });
                    }
                    _ => self.render_library_view(ui),
                }
            });

        self.render_removal_dialog(ui.ctx());
        self.render_settings_dialog(ui.ctx());
        self.render_playlist_name_dialog(ui.ctx());
        self.render_playlist_deletion_dialog(ui.ctx());

        // Banners over the whole window, top center: adding progress (only when
        // poll_loaders decided to show it) and below it an error — until "OK" is pressed
        let mut banner_top = components::BANNER_TOP;
        if self.loader_shown_at.is_some()
            && let Some(pending) = self.loaders.front()
        {
            let (cancel, bottom) = components::loading_banner(
                ui.ctx(),
                pending.progress_template(),
                pending.loader.progress(),
            );
            banner_top = bottom - ui.ctx().content_rect().top() + components::BANNER_GAP;
            if cancel {
                self.dispatch(Action::CancelLoading);
            }
        }
        if let Some(error) = &self.error
            && components::error_banner(ui.ctx(), error, banner_top)
        {
            self.error = None;
        }

        // Resize lines — last, on top of everything, at panel borders — where egui lets the edge be dragged
        let sidebar_rect = sidebar.response.rect;
        components::paint_resize_line(ui, sidebar_rect.right(), sidebar_rect.y_range());
        let track_list_rect = track_list.response.rect;
        components::paint_resize_line(ui, track_list_rect.left(), track_list_rect.y_range());

        // File drag and drop: zones over the whole window while files are over it
        self.render_file_drop(ui.ctx(), track_list_rect);

        // Everything the user did during the frame — after drawing, before saving
        self.apply_actions(ui.ctx());
        self.save_if_changed(ui.ctx());
    }
}
