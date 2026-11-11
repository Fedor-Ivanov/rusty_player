//! Saved playlists as a grid: cover (from the first tracks), title and track count.

use eframe::egui::{Align, Ui};
use std::path::PathBuf;

use super::{
    TILE_RADIUS, placeholder,
    search::{contains_query, normalized_query},
    shared::overlay_play_button,
};
use crate::{
    core::playlists::PlaylistId,
    ui::{
        action::Action,
        app::{ListFocus, PlayerApp},
        components::{self, TileLine},
        covers::CoverSize,
        i18n::{fill, t},
        navigation::Route,
    },
};

/// The playlist cover is searched among this many of its first tracks
pub(crate) const PLAYLIST_COVER_TRACKS: usize = 8;

/// Playlist context menu item
enum PlaylistMenuItem {
    Play,
    Rename,
    /// Delete the selected playlists — one or several
    Delete,
}

impl PlayerApp {
    /// All saved playlists as a grid. A click opens a playlist, the button on the cover plays
    /// it; from the menu — play, rename, delete. If several are selected — the menu only
    /// deletes them all
    pub(crate) fn render_playlists(&mut self, ui: &mut Ui) {
        let all = self.core.playlists.all();
        if all.is_empty() {
            placeholder(ui, t().no_playlists);
            return;
        }
        // Search — by title
        let query = normalized_query(&self.search.query);
        let shown: Vec<usize> = (0..all.len())
            .filter(|&index| contains_query(&all[index].name, &query))
            .collect();
        if shown.is_empty() {
            placeholder(ui, t().no_results);
            return;
        }
        // Tile indices belong to the playlists and the query — when they change the selection
        // is reset
        self.playlists_grid
            .set_scope(&format!("{}/{query}", self.core.playlists.revision()));

        let covers = &mut self.covers;
        let mut menu: Option<(PlaylistMenuItem, PlaylistId)> = None;
        let response = self.playlists_grid.show(
            ui,
            shown.len(),
            2,
            Align::Min,
            self.focus == ListFocus::Grid,
            |ui, index, image_rect, tile| {
                let playlist = &all[shown[index]];
                let candidates = cover_candidates(&playlist.tracks);
                let cover = covers.get(&candidates, CoverSize::Thumbnail);
                components::paint_cover(ui, image_rect, cover, TILE_RADIUS);
                // The play button on the cover — like for albums; an empty playlist has nothing to play
                if !playlist.tracks.is_empty()
                    && overlay_play_button(ui, image_rect, tile, &format!("play_playlist {index}"))
                {
                    menu = Some((PlaylistMenuItem::Play, playlist.id));
                }
                if let Some(item) = playlist_context_menu(tile) {
                    menu = Some((item, playlist.id));
                }
                vec![
                    TileLine {
                        text: playlist.name.clone(),
                        primary: true,
                    },
                    TileLine {
                        text: t().count(playlist.tracks.len(), &t().track_count),
                        primary: false,
                    },
                ]
            },
        );
        // Rows next to the screen — in advance: covers will be ready when scrolling
        for index in self.playlists_grid.nearby() {
            covers.prefetch(
                &cover_candidates(&all[shown[index]].tracks),
                CoverSize::Thumbnail,
            );
        }
        if response.interacted {
            self.focus = ListFocus::Grid;
        }

        let id_of = |index: usize| all[shown[index]].id;
        // The right-click menu and Delete act on the selection: a right click selects
        // the tile under the cursor if it wasn't selected
        let selected = || {
            self.playlists_grid
                .selection()
                .iter()
                .map(|&index| id_of(index))
                .collect::<Vec<_>>()
        };
        let action = if let Some(index) = response.opened {
            Some(Action::Navigate(Route::Playlist(id_of(index))))
        } else if let Some((item, id)) = menu {
            Some(match item {
                PlaylistMenuItem::Play => Action::PlayPlaylist(id),
                PlaylistMenuItem::Rename => Action::RequestRenamePlaylist(id),
                PlaylistMenuItem::Delete => Action::RequestPlaylistDeletion(selected()),
            })
        } else if response.delete {
            Some(Action::RequestPlaylistDeletion(selected()))
        } else {
            None
        };
        if let Some(action) = action {
            self.dispatch(action);
        }
    }
}

/// Where to look for the playlist cover: its first tracks
pub(crate) fn cover_candidates(tracks: &[crate::core::library::Track]) -> Vec<PathBuf> {
    tracks
        .iter()
        .take(PLAYLIST_COVER_TRACKS)
        .map(|track| track.path.clone())
        .collect()
}

/// Playlist context menu. One selected — play, rename, delete it;
/// several — only delete them all: several can't be played or renamed at once
fn playlist_context_menu(tile: &components::Tile) -> Option<PlaylistMenuItem> {
    let mut chosen = None;
    tile.response.context_menu(|ui| {
        components::menu_style(ui);
        if tile.selected > 1 {
            let items = t().count(tile.selected, &t().playlist_count);
            let label = fill(t().delete_items, &[("items", &items)]);
            if components::menu_item(ui, &label).clicked() {
                chosen = Some(PlaylistMenuItem::Delete);
            }
            return;
        }
        for (label, item) in [
            (t().play, PlaylistMenuItem::Play),
            (t().rename, PlaylistMenuItem::Rename),
            (t().delete, PlaylistMenuItem::Delete),
        ] {
            if components::menu_item(ui, label).clicked() {
                chosen = Some(item);
            }
        }
    });
    chosen
}
