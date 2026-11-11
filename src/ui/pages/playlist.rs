//! Saved playlist page: cover, title, track count, buttons and tracks.

use eframe::egui::{self, Label, RichText, Sense, Ui, vec2};
use std::collections::BTreeSet;

use super::{
    ARTIST_NAME_SIZE, TILE_RADIUS, album::ALBUM_PAGE_COVER_SIZE, placeholder,
    playlists::cover_candidates,
};
use crate::{
    core::playlists::PlaylistId,
    ui::{
        action::Action,
        app::{ListFocus, PlayerApp},
        components::{self, ButtonStyle},
        covers::CoverSize,
        i18n::{fill, t},
        navigation::Route,
        style::theme::{TEXT, TEXT_MUTED},
    },
};

/// A button in the playlist header
enum HeaderButton {
    Play,
    Rename,
    Delete,
}

/// Playlist track menu item
enum TrackItem {
    AddToQueue,
    GoToAlbum,
    AddToLibrary,
    RemoveFromLibrary,
    Remove,
}

impl PlayerApp {
    /// Page of playlist `id`. A double click on a track plays only it (it goes into
    /// the queue); the whole playlist — with the "Play" button. From the track menu — add to the queue
    /// or remove from the playlist
    pub(crate) fn render_playlist_page(&mut self, ui: &mut Ui, id: PlaylistId) {
        let Some(playlist) = self.core.playlists.get(id) else {
            // The playlist was deleted — to the playlist list
            self.dispatch(Action::Navigate(Route::Playlists));
            return;
        };
        let name = playlist.name.clone();
        let track_count = playlist.tracks.len();
        let candidates = cover_candidates(&playlist.tracks);

        // ---- header
        let mut pressed = None;
        egui::Frame::new()
            .inner_margin(egui::Margin {
                left: 16,
                right: 16,
                top: 0,
                bottom: 20,
            })
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    let (cover_rect, _) = ui.allocate_exact_size(
                        vec2(ALBUM_PAGE_COVER_SIZE, ALBUM_PAGE_COVER_SIZE),
                        Sense::hover(),
                    );
                    let cover = self.covers.get(&candidates, CoverSize::Large);
                    components::paint_cover(ui, cover_rect, cover, TILE_RADIUS);
                    ui.add_space(20.0);

                    ui.vertical(|ui| {
                        ui.add(
                            Label::new(RichText::new(&name).size(ARTIST_NAME_SIZE).color(TEXT))
                                .selectable(false)
                                .truncate(),
                        );
                        ui.add(
                            Label::new(
                                RichText::new(t().count(track_count, &t().track_count))
                                    .size(13.0)
                                    .color(TEXT_MUTED),
                            )
                            .selectable(false),
                        );
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            // An empty playlist can't be played
                            ui.add_enabled_ui(track_count > 0, |ui| {
                                if components::pill_button(ui, t().play, ButtonStyle::Primary)
                                    .clicked()
                                {
                                    pressed = Some(HeaderButton::Play);
                                }
                            });
                            if components::pill_button(ui, t().rename, ButtonStyle::Secondary)
                                .clicked()
                            {
                                pressed = Some(HeaderButton::Rename);
                            }
                            if components::pill_button(ui, t().delete, ButtonStyle::Secondary)
                                .clicked()
                            {
                                pressed = Some(HeaderButton::Delete);
                            }
                        });
                    });
                });
            });
        if let Some(button) = pressed {
            self.dispatch(match button {
                HeaderButton::Play => Action::PlayPlaylist(id),
                HeaderButton::Rename => Action::RequestRenamePlaylist(id),
                HeaderButton::Delete => Action::RequestPlaylistDeletion(vec![id]),
            });
            return;
        }

        // ---- tracks
        // A single selected track from the library — can go to its album. The menu opens
        // after selection, so the selection at the start of the frame is the one it refers to
        let single = match self
            .playlist_tracks_view
            .selection()
            .iter()
            .collect::<Vec<_>>()[..]
        {
            [&index] => self
                .core
                .playlists
                .get(id)
                .and_then(|playlist| playlist.tracks.get(index).cloned()),
            _ => None,
        };
        let album = single.and_then(|track| self.album_route(&track));
        let Some(playlist) = self.core.playlists.get(id) else {
            return;
        };
        if playlist.tracks.is_empty() {
            placeholder(ui, t().no_tracks);
            return;
        }
        let current_path = self
            .core
            .controller
            .player()
            .current_track
            .as_ref()
            .map(|track| track.path.clone());
        let playlists = &self.core.playlists;
        // How many selected tracks are not from the library (added with the "+" button bypassing it) and how many
        // are from it — for the "Add to library" and "Remove from library" items. The menu opens
        // after selection, so the selection at the start of the frame is the one it refers to
        let library_paths = self.core.library.paths();
        let selected_count = self.playlist_tracks_view.selection().len();
        let outside_library = self
            .playlist_tracks_view
            .selection()
            .iter()
            .filter_map(|&index| playlist.tracks.get(index))
            .filter(|track| !library_paths.contains(&track.path))
            .count();
        let in_library = selected_count - outside_library;
        // The menu only knows how many rows are selected: which ones — we find out after the list
        let mut menu = None;
        let response = self.playlist_tracks_view.show(
            ui,
            &playlist.tracks,
            |index| components::RowState {
                // The currently playing track is highlighted
                current: current_path.as_ref() == Some(&playlist.tracks[index].path),
                missing: playlists.is_missing(&playlist.tracks[index]),
            },
            self.focus == ListFocus::Library,
            |ui, selected| {
                let (add, remove) = match selected {
                    0 | 1 => (
                        t().add_to_queue.to_owned(),
                        t().remove_from_this_playlist.to_owned(),
                    ),
                    count => {
                        let items = t().count(count, &t().track_count);
                        let items = [("items", items.as_str())];
                        (
                            fill(t().add_items_to_queue, &items),
                            fill(t().remove_items_from_this_playlist, &items),
                        )
                    }
                };
                if components::menu_item(ui, &add).clicked() {
                    menu = Some(TrackItem::AddToQueue);
                }
                if album.is_some() && components::menu_item(ui, t().go_to_album).clicked() {
                    menu = Some(TrackItem::GoToAlbum);
                }
                if outside_library > 0 {
                    let label = t().for_tracks(
                        t().add_to_library,
                        t().add_items_to_library,
                        outside_library,
                    );
                    if components::menu_item(ui, &label).clicked() {
                        menu = Some(TrackItem::AddToLibrary);
                    }
                }
                if in_library > 0 {
                    let label = t().for_tracks(
                        t().remove_from_library,
                        t().remove_items_from_library,
                        in_library,
                    );
                    if components::menu_item(ui, &label).clicked() {
                        menu = Some(TrackItem::RemoveFromLibrary);
                    }
                }
                if components::menu_item(ui, &remove).clicked() {
                    menu = Some(TrackItem::Remove);
                }
            },
        );
        if response.interacted {
            self.focus = ListFocus::Library;
        }

        let selected: BTreeSet<usize> = self.playlist_tracks_view.selection().clone();
        let action = match menu {
            Some(TrackItem::AddToQueue) => Some(Action::AddPlaylistTracksToQueue {
                id,
                indices: selected.into_iter().collect(),
            }),
            Some(TrackItem::GoToAlbum) => album.map(Action::Navigate),
            Some(TrackItem::AddToLibrary) => Some(Action::AddPlaylistTracksToLibrary {
                id,
                indices: selected.into_iter().collect(),
            }),
            // Removing from the library — with confirmation, like in Songs
            Some(TrackItem::RemoveFromLibrary) => {
                let paths = selected
                    .iter()
                    .filter_map(|&index| playlist.tracks.get(index))
                    .map(|track| track.path.clone())
                    .collect();
                Some(Action::RequestSongsRemoval(
                    self.core.library.indices_of(&paths),
                ))
            }
            Some(TrackItem::Remove) => Some(Action::RemoveFromPlaylist {
                id,
                indices: selected,
            }),
            // Removing from the playlist doesn't delete the file: no confirmation
            None if response.delete => Some(Action::RemoveFromPlaylist {
                id,
                indices: selected,
            }),
            None => response
                .activated
                .map(|index| Action::PlayFromPlaylist { id, index }),
        };
        if let Some(action) = action {
            self.dispatch(action);
        }
    }
}
