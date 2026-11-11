//! Shared by library pages: context menus for albums, artists and tracks,
//! artist track selections.

use eframe::egui;

use crate::{
    core::library::{Track, groups::AlbumGroup},
    core::playlists::PlaylistId,
    ui::style::icons::paint_overlay_play,
    ui::{
        action::Action,
        app::PlayerApp,
        components::{self},
        dialogs::remove::GroupKind,
        i18n::{fill, t},
        navigation::{AlbumKey, Route},
    },
};

/// Play button over a cover in the grid: size and offset from the cover corner
const OVERLAY_PLAY_SIZE: f32 = 32.0;
const OVERLAY_PLAY_INSET: f32 = 8.0;

/// Play button in the top left corner of the cover of tile `tile` — fades in on tile
/// hover. Returns `true` if it was clicked
pub(crate) fn overlay_play_button(
    ui: &mut egui::Ui,
    image_rect: egui::Rect,
    tile: &components::Tile,
    id_salt: &str,
) -> bool {
    if tile.hover == 0.0 {
        return false;
    }
    let play_rect = egui::Rect::from_min_size(
        image_rect.min + egui::vec2(OVERLAY_PLAY_INSET, OVERLAY_PLAY_INSET),
        egui::vec2(OVERLAY_PLAY_SIZE, OVERLAY_PLAY_SIZE),
    );
    let play = components::icon_button(ui, play_rect, components::HitShape::Circle, id_salt, true);
    paint_overlay_play(ui, play_rect, tile.hover, play.hover);
    play.clicked
}

/// Where to add tracks from the "Add to playlist ›" submenu
#[derive(Clone, Copy)]
pub(crate) enum PlaylistChoice {
    /// To a new playlist — ask for the name first
    New,
    Existing(PlaylistId),
}

/// Saved playlists for the "Add to playlist ›" submenu: id and title
pub(crate) type PlaylistEntries = [(PlaylistId, String)];

/// The "Add to playlist ›" submenu: "New playlist…" and saved playlists.
/// Returns the choice
pub(crate) fn playlist_submenu(
    ui: &mut egui::Ui,
    playlists: &PlaylistEntries,
) -> Option<PlaylistChoice> {
    let mut chosen = None;
    components::submenu(ui, t().add_to_playlist, |ui| {
        if components::menu_item(ui, t().new_playlist).clicked() {
            chosen = Some(PlaylistChoice::New);
        }
        if !playlists.is_empty() {
            ui.separator();
        }
        for (id, name) in playlists {
            if components::menu_item(ui, name).clicked() {
                chosen = Some(PlaylistChoice::Existing(*id));
            }
        }
    });
    chosen
}

/// What to do with an album or artist — from the context menu or via the button on the cover
#[derive(Clone, Copy)]
pub(crate) enum GroupAction {
    /// Replace the queue with their tracks and play
    Play,
    AddToQueue,
    /// Their tracks — into a saved playlist
    AddToPlaylist(PlaylistChoice),
    /// Remove their tracks from the library (doesn't touch the files)
    Remove,
}

/// Which tiles the action applies to
pub(crate) enum Target {
    /// One tile: the play button on the cover
    One(usize),
    /// Selected ones: context menu, Delete key
    Selection,
}

/// Album or artist context menu. Acts on all selected tiles —
/// their count is in the labels. Returns the chosen action
pub(crate) fn group_context_menu(
    tile: &components::Tile,
    kind: GroupKind,
    playlists: &PlaylistEntries,
) -> Option<GroupAction> {
    let mut action = None;
    tile.response.context_menu(|ui| {
        components::menu_style(ui);
        let t = t();
        let items = match (tile.selected, kind) {
            (1, _) => [
                (t.play.to_owned(), GroupAction::Play),
                (t.add_to_queue.to_owned(), GroupAction::AddToQueue),
                (t.remove_from_library.to_owned(), GroupAction::Remove),
            ],
            (count, kind) => {
                let word = match kind {
                    GroupKind::Album => &t.album_count,
                    GroupKind::Artist => &t.artist_count,
                    GroupKind::Songs => &t.track_count,
                };
                let items = t.count(count, word);
                let items = [("items", items.as_str())];
                [
                    (fill(t.play_items, &items), GroupAction::Play),
                    (fill(t.add_items_to_queue, &items), GroupAction::AddToQueue),
                    (
                        fill(t.remove_items_from_library, &items),
                        GroupAction::Remove,
                    ),
                ]
            }
        };
        let [play, add_to_queue, remove] = items;
        for (label, menu_action) in [play, add_to_queue] {
            if components::menu_item(ui, &label).clicked() {
                action = Some(menu_action);
            }
        }
        if let Some(choice) = playlist_submenu(ui, playlists) {
            action = Some(GroupAction::AddToPlaylist(choice));
        }
        if components::menu_item(ui, &remove.0).clicked() {
            action = Some(remove.1);
        }
    });
    action
}

/// Library track list menu labels: add to playlist and remove from library.
/// For several selected — with their count
pub(crate) fn track_menu_labels(selected: usize) -> (String, String) {
    let t = t();
    match selected {
        0 | 1 => (t.add_to_queue.to_owned(), t.remove_from_library.to_owned()),
        count => {
            let items = t.count(count, &t.track_count);
            let items = [("items", items.as_str())];
            (
                fill(t.add_items_to_queue, &items),
                fill(t.remove_items_from_library, &items),
            )
        }
    }
}

impl PlayerApp {
    /// Where "Go to album" leads for a track: its album page — if the track is in the library
    /// and has a known album (see `Core::album_of`). Otherwise there's no menu item
    pub(crate) fn album_route(&mut self, track: &Track) -> Option<Route> {
        if !self.core.library.paths().contains(&track.path) {
            return None;
        }
        self.core
            .album_of(track)
            .map(|album| Route::Album(AlbumKey::of(album)))
    }

    /// Saved playlists for the "Add to playlist ›" submenu
    pub(crate) fn playlist_entries(&self) -> Vec<(PlaylistId, String)> {
        self.core
            .playlists
            .all()
            .iter()
            .map(|playlist| (playlist.id, playlist.name.clone()))
            .collect()
    }

    /// Albums of artist `artist` (name case-insensitive, as in grouping),
    /// in grid order
    pub(crate) fn artist_albums(&self, artist: &str) -> impl Iterator<Item = &AlbumGroup> {
        let artist = artist.to_lowercase();
        self.core
            .groups
            .albums
            .iter()
            .filter(move |album| album.artist.to_lowercase() == artist)
    }

    /// Library indices of all the artist's tracks: album by album in grid order
    pub(crate) fn artist_track_indices(&self, artist: &str) -> Vec<usize> {
        self.artist_albums(artist)
            .flat_map(|album| album.tracks.iter().copied())
            .collect()
    }
}

/// A menu item for albums or artists `names` — into an action on their library tracks
/// `tracks`, in order
pub(crate) fn group_action(
    action: GroupAction,
    kind: GroupKind,
    names: Vec<String>,
    tracks: Vec<usize>,
) -> Action {
    match action {
        GroupAction::Play => Action::PlayLibraryTracks(tracks),
        GroupAction::AddToQueue => Action::AddToQueue(tracks),
        GroupAction::AddToPlaylist(PlaylistChoice::New) => Action::RequestNewPlaylist(tracks),
        GroupAction::AddToPlaylist(PlaylistChoice::Existing(id)) => {
            Action::AddToPlaylist { id, tracks }
        }
        // Don't remove right away — ask first
        GroupAction::Remove => Action::RequestGroupRemoval {
            kind,
            names,
            tracks,
        },
    }
}

/// Library track list menu item
pub(crate) enum TrackMenuItem {
    AddToQueue,
    AddToPlaylist(PlaylistChoice),
    /// Open the track's album
    GoToAlbum(Route),
    RemoveFromLibrary,
}

/// Library track list menu: add to queue, add to a saved playlist,
/// go to album and remove from library. For several selected — with their count.
/// `album` — where "Go to album" leads (see `PlayerApp::album_route`); `None` — no item:
/// several tracks are selected, the album is unknown, or we're already on its page.
/// Returns the chosen item
pub(crate) fn track_menu(
    ui: &mut egui::Ui,
    selected: usize,
    playlists: &PlaylistEntries,
    album: Option<&Route>,
) -> Option<TrackMenuItem> {
    let (add_label, remove_label) = track_menu_labels(selected);
    let mut chosen = None;
    if components::menu_item(ui, &add_label).clicked() {
        chosen = Some(TrackMenuItem::AddToQueue);
    }
    if let Some(choice) = playlist_submenu(ui, playlists) {
        chosen = Some(TrackMenuItem::AddToPlaylist(choice));
    }
    if let Some(route) = album
        && components::menu_item(ui, t().go_to_album).clicked()
    {
        chosen = Some(TrackMenuItem::GoToAlbum(route.clone()));
    }
    if components::menu_item(ui, &remove_label).clicked() {
        chosen = Some(TrackMenuItem::RemoveFromLibrary);
    }
    chosen
}

impl TrackMenuItem {
    /// A menu item — into an action on the selected library tracks `selected`
    pub(crate) fn into_action(self, selected: Vec<usize>) -> Action {
        match self {
            TrackMenuItem::AddToQueue => Action::AddToQueue(selected),
            TrackMenuItem::AddToPlaylist(PlaylistChoice::New) => {
                Action::RequestNewPlaylist(selected)
            }
            TrackMenuItem::AddToPlaylist(PlaylistChoice::Existing(id)) => Action::AddToPlaylist {
                id,
                tracks: selected,
            },
            TrackMenuItem::GoToAlbum(route) => Action::Navigate(route),
            TrackMenuItem::RemoveFromLibrary => Action::RequestSongsRemoval(selected),
        }
    }
}
