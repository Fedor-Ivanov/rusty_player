//! The "remove from library?" dialog: what's being removed and the confirmation.

use eframe::egui::{self};
use std::{collections::HashSet, path::PathBuf};

use crate::{
    core::library::Track,
    ui::{
        action::Action,
        app::PlayerApp,
        components::{self},
        i18n::{fill, t},
    },
};

/// What's removed from the library: an album, an artist or individual tracks — the removal
/// dialog texts depend on it
#[derive(Clone, Copy)]
pub(crate) enum GroupKind {
    Album,
    Artist,
    /// Tracks selected in a list (Songs or an album page)
    Songs,
}

/// Albums, artists or tracks whose removal from the library awaits confirmation
pub(crate) struct LibraryRemoval {
    pub(crate) kind: GroupKind,
    /// Album titles, artist names or a track title — for the dialog text
    pub(crate) names: Vec<String>,
    /// Files of their tracks
    pub(crate) paths: HashSet<PathBuf>,
}

impl PlayerApp {
    /// Asks to confirm removing library tracks with indices `indices` (the files themselves
    /// are not touched). They are removed after the dialog is answered — see `render_removal_dialog`
    pub(crate) fn request_songs_removal(&mut self, indices: &[usize]) {
        let tracks: Vec<&Track> = indices
            .iter()
            .filter_map(|&index| self.core.library.tracks().get(index))
            .collect();
        let names = tracks.iter().map(|track| track.title.clone()).collect();
        let paths: HashSet<PathBuf> = tracks.iter().map(|track| track.path.clone()).collect();
        if !paths.is_empty() {
            self.pending_removal = Some(LibraryRemoval {
                kind: GroupKind::Songs,
                names,
                paths,
            });
        }
    }

    /// Asks to confirm removing albums or artists `names` — library tracks
    /// with indices `tracks`. Paths are remembered, not indices: while the dialog is open, the library
    /// may change
    pub(crate) fn request_group_removal(
        &mut self,
        kind: GroupKind,
        names: Vec<String>,
        tracks: &[usize],
    ) {
        let paths = self.core.library.paths_of(tracks);
        self.pending_removal = Some(LibraryRemoval { kind, names, paths });
    }

    /// The "really remove from library?" dialog — while waiting for an answer
    pub(crate) fn render_removal_dialog(&mut self, ctx: &egui::Context) {
        let Some(removal) = &self.pending_removal else {
            return;
        };
        let t = t();
        let count = |word| t.count(removal.names.len(), word);
        let tracks = t.count(removal.paths.len(), &t.track_count);
        let (title, message) = match (removal.kind, removal.names.as_slice()) {
            (GroupKind::Album, [album]) => (
                t.remove_album_title.to_owned(),
                fill(t.remove_album_message, &[("album", album)]),
            ),
            (GroupKind::Album, _) => {
                let albums = count(&t.album_count);
                (
                    fill(t.remove_albums_title, &[("albums", &albums)]),
                    fill(
                        t.remove_albums_message,
                        &[("albums", &albums), ("tracks", &tracks)],
                    ),
                )
            }
            (GroupKind::Artist, [artist]) => (
                t.remove_artist_title.to_owned(),
                fill(t.remove_artist_message, &[("artist", artist)]),
            ),
            (GroupKind::Artist, _) => {
                let artists = count(&t.artist_count);
                (
                    fill(t.remove_artists_title, &[("artists", &artists)]),
                    fill(
                        t.remove_artists_message,
                        &[("artists", &artists), ("tracks", &tracks)],
                    ),
                )
            }
            (GroupKind::Songs, [song]) => (
                t.remove_song_title.to_owned(),
                fill(t.remove_song_message, &[("song", song)]),
            ),
            (GroupKind::Songs, _) => {
                let songs = count(&t.song_count);
                (
                    fill(t.remove_songs_title, &[("songs", &songs)]),
                    fill(t.remove_songs_message, &[("songs", &songs)]),
                )
            }
        };
        let answer =
            components::confirm_dialog(ctx, "remove_from_library", &title, &message, t.remove);
        match answer {
            components::Confirm::Pending => {}
            components::Confirm::Cancelled => self.pending_removal = None,
            components::Confirm::Confirmed => {
                if let Some(removal) = self.pending_removal.take() {
                    self.dispatch(Action::RemoveFromLibrary(removal.paths));
                }
            }
        }
    }
}
