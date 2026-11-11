//! Songs: all library tracks as a list (or those found by search).

use eframe::egui::{self, Ui};

use super::{placeholder, shared::track_menu};
use crate::ui::{
    action::Action,
    app::{ListFocus, PlayerApp},
    components::{self, ButtonStyle},
    i18n::t,
};

impl PlayerApp {
    /// All library tracks as a list — or those found by search. A double click adds the track
    /// to the queue and plays it; from the menu — add to a playlist or remove from the library
    pub(crate) fn render_songs(&mut self, ui: &mut Ui) {
        if self.core.library.tracks().is_empty() {
            placeholder(ui, t().library_empty);
            return;
        }
        self.refresh_found_songs();
        if self
            .search
            .found_songs
            .as_ref()
            .is_some_and(|found| found.indices.is_empty())
        {
            placeholder(ui, t().no_results);
            return;
        }

        // "Play" — the shown tracks in list order replace the queue and play
        let mut play_all = false;
        egui::Frame::new()
            .inner_margin(egui::Margin {
                left: 16,
                right: 16,
                top: 0,
                bottom: 16,
            })
            .show(ui, |ui| {
                play_all = components::pill_button(ui, t().play, ButtonStyle::Primary).clicked();
            });
        if play_all {
            self.dispatch(Action::PlayLibraryTracks(self.shown_song_indices()));
        }

        // "Go to album" — only for a single selected track. The menu opens after
        // selection, so the selection at the start of the frame is the one it refers to
        let album = match self.library_view.selection().iter().collect::<Vec<_>>()[..] {
            [&row] => self
                .song_index(row)
                .and_then(|index| self.core.library.tracks().get(index).cloned())
                .and_then(|track| self.album_route(&track)),
            _ => None,
        };

        // List row indices are indices in `found` (during search) or directly in the library.
        // The menu only knows how many rows are selected: which ones — we find out after the list
        let mut menu = None;
        let library = &self.core.library;
        let found = self.search.found_songs.as_ref();
        let tracks = found.map_or(library.tracks(), |found| &found.tracks);
        // Playlists for the "Add to playlist ›" submenu
        let playlists = self.playlist_entries();
        let response = self.library_view.show(
            ui,
            tracks,
            |index| components::RowState {
                current: false,
                missing: library.is_missing(found.map_or(index, |found| found.indices[index])),
            },
            self.focus == ListFocus::Library,
            |ui, selected| {
                if let Some(item) = track_menu(ui, selected, &playlists, album.as_ref()) {
                    menu = Some(item);
                }
            },
        );

        if response.interacted {
            self.focus = ListFocus::Library;
        }
        let selected: Vec<usize> = self
            .library_view
            .selection()
            .iter()
            .filter_map(|&index| self.song_index(index))
            .collect();
        let action = match menu {
            Some(item) => Some(item.into_action(selected)),
            None if response.delete => Some(Action::RequestSongsRemoval(selected)),
            None => response
                .activated
                .and_then(|index| self.song_index(index))
                .map(Action::PlayFromLibrary),
        };
        if let Some(action) = action {
            self.dispatch(action);
        }
    }

    /// Library index of the track in row `row` of the Songs list
    pub(crate) fn song_index(&self, row: usize) -> Option<usize> {
        match &self.search.found_songs {
            Some(found) => found.indices.get(row).copied(),
            None => (row < self.core.library.tracks().len()).then_some(row),
        }
    }

    /// Library indices of all tracks shown in Songs, in order
    pub(crate) fn shown_song_indices(&self) -> Vec<usize> {
        match &self.search.found_songs {
            Some(found) => found.indices.clone(),
            None => (0..self.core.library.tracks().len()).collect(),
        }
    }
}
