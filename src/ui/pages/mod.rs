//! Central area pages: what's open is decided by `Route` (see `navigation`).
//! Each page is a separate file with `impl PlayerApp`; here — page selection, the title
//! with "breadcrumbs" and search, and what's shared between pages.

mod album;
mod albums;
mod artists;
mod now_playing;
mod playlist;
mod playlists;
mod search;
mod shared;
mod songs;

pub(crate) use search::LibrarySearch;

use eframe::egui::{self, Align, Align2, FontId, Ui};

use crate::ui::{
    action::Action,
    app::PlayerApp,
    components::{self},
    i18n::{fill, t},
    navigation::{Route, Section},
    style::theme::{TEXT, TEXT_FAINT},
};

/// Image rounding in the grid
pub(crate) const TILE_RADIUS: u8 = 8;

/// Artist name in their profile header and album title on the album page
pub(crate) const ARTIST_NAME_SIZE: f32 = 30.0;

/// Text centered in the remaining space
pub(crate) fn placeholder(ui: &Ui, text: &str) {
    ui.painter().text(
        ui.available_rect_before_wrap().center(),
        Align2::CENTER_CENTER,
        text,
        FontId::proportional(14.0),
        TEXT_FAINT,
    );
}

impl PlayerApp {
    /// The open library page (`self.route`) across the whole central area
    pub(crate) fn render_library_view(&mut self, ui: &mut Ui) {
        egui::Frame::new()
            .inner_margin(16.0)
            .show(ui, |ui| self.render_breadcrumbs(ui));
        match self.route.clone() {
            Route::Album(album) | Route::ArtistAlbum { album, .. } => {
                self.render_album_page(ui, &album)
            }
            Route::Artist(artist) => self.render_artist_profile(ui, &artist),
            Route::Artists => self.render_artists(ui),
            Route::Albums => self.render_albums(ui),
            Route::Songs => self.render_songs(ui),
            Route::Playlists => self.render_playlists(ui),
            Route::Playlist(id) => self.render_playlist_page(ui, id),
            Route::NowPlaying => {}
        }
    }

    /// Page title. Inside an artist or album — "breadcrumbs", each leading
    /// to its level: "Albums › Album", "Artists › Artist › Album"
    pub(crate) fn render_breadcrumbs(&mut self, ui: &mut Ui) {
        let section = self.route.section();
        let title = section.title();
        let target = match self.route.clone() {
            Route::Album(album) => {
                components::breadcrumb_title(ui, &[title], &album.title).map(|_| Route::Albums)
            }
            Route::ArtistAlbum { artist, album } => {
                match components::breadcrumb_title(ui, &[title, &artist], &album.title) {
                    Some(0) => Some(Route::Artists),
                    Some(_) => Some(Route::Artist(artist)),
                    None => None,
                }
            }
            Route::Artist(artist) => {
                components::breadcrumb_title(ui, &[title], &artist).map(|_| Route::Artists)
            }
            Route::Playlist(id) => {
                let name = self
                    .core
                    .playlists
                    .get(id)
                    .map(|playlist| playlist.name.clone())
                    .unwrap_or_default();
                components::breadcrumb_title(ui, &[title], &name).map(|_| Route::Playlists)
            }
            // The section's top level — with a search field on the right. In an empty library there's nothing to search
            Route::Artists | Route::Albums | Route::Songs
                if !self.core.library.tracks().is_empty() =>
            {
                self.render_title_with_search(ui, section);
                None
            }
            Route::Playlists if !self.core.playlists.all().is_empty() => {
                self.render_title_with_search(ui, section);
                None
            }
            _ => {
                components::block_title(ui, title, Align::LEFT, TEXT);
                None
            }
        };
        if let Some(route) = target {
            self.dispatch(Action::Navigate(route));
        }
    }

    /// View title on the left and search field on the right. In Songs, next to the title — the track count,
    /// like "Queue · N tracks"; during search — how many found: "· 3 of 120 tracks"
    pub(crate) fn render_title_with_search(&mut self, ui: &mut Ui, section: Section) {
        let note = (section == Section::Songs).then(|| {
            self.refresh_found_songs();
            let total = self.core.library.tracks().len();
            let tracks = t().count(total, &t().track_count);
            match &self.search.found_songs {
                Some(found) => format!(
                    "· {}",
                    fill(
                        t().found_of,
                        &[
                            ("found", &found.indices.len().to_string()),
                            ("tracks", &tracks)
                        ]
                    )
                ),
                None => format!("· {tracks}"),
            }
        });
        let query = &mut self.search.query;
        let changed = components::title_with_search(ui, section.title(), query, |ui| match &note {
            Some(note) => {
                components::block_title_with_note(ui, section.title(), note);
            }
            None => {
                components::block_title(ui, section.title(), Align::LEFT, TEXT);
            }
        });
        // Grid selections reset by themselves — the query is part of their scope. The Songs list
        // shows other tracks — its selection points to nothing
        if changed && section == Section::Songs {
            self.library_view.clear_selection();
        }
    }
}
