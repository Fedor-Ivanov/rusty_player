//! User actions on app data — and the only place where they are performed.
//!
//! Pages and panels don't change the library, the queue and saved playlists themselves: they send an `Action`
//! (`dispatch`), and at the end of the frame `apply_actions` performs the accumulated ones in order. This way everything
//! the app can do is visible in one enum, and each action is implemented once —
//! wherever it's invoked from: a menu, a button or the keyboard.
//!
//! The UI's own state (selection, scroll, search text) doesn't belong here —
//! components still change it themselves.

use eframe::egui;
use std::{
    collections::{BTreeSet, HashSet},
    path::PathBuf,
};

use crate::{
    core::{
        library::loader::Finished,
        playback::{PlayerState, RepeatMode},
        playlists::PlaylistId,
        settings::{Glass, Language},
    },
    ui::{
        app::{ListFocus, PlayerApp},
        dialogs::{
            playlist::{NameDialog, NamePurpose},
            remove::GroupKind,
        },
        i18n,
        loading::Destination,
        navigation::Route,
    },
};

pub(crate) enum Action {
    /// Replace the queue with library tracks (by their library indices) and play from the first
    PlayLibraryTracks(Vec<usize>),
    /// Play a library track: if it's not in the queue, it's appended
    PlayFromLibrary(usize),
    /// Library tracks — to the end of the queue
    AddToQueue(Vec<usize>),
    /// Ask whether to remove library tracks (see `dialogs::remove`)
    RequestSongsRemoval(Vec<usize>),
    /// Ask whether to remove albums or artists `names` — their library tracks `tracks`
    RequestGroupRemoval {
        kind: GroupKind,
        names: Vec<String>,
        tracks: Vec<usize>,
    },
    /// Remove the tracks of these files from the library — confirmed in the dialog. The files themselves
    /// stay on disk
    RemoveFromLibrary(HashSet<PathBuf>),
    /// Play the queue track with the index
    PlayQueueTrack(usize),
    /// Show a library track (file) in Songs: open the list, select the track and scroll
    /// to it
    ShowInLibrary(PathBuf),
    /// Remove tracks with the indices from the queue
    RemoveFromQueue(BTreeSet<usize>),
    /// Clear the queue: playback stops
    ClearQueue,
    /// Queue tracks with the indices — into the library (those not from it)
    AddQueueTracksToLibrary(Vec<usize>),
    ToggleShuffle,
    ToggleRepeat(RepeatMode),

    // ---- saved playlists
    /// Open the name dialog to save the queue as a playlist
    RequestSaveQueue,
    SaveQueueAsPlaylist(String),
    /// Overwrite the playlist linked to the queue with its tracks
    SaveQueueToPlaylist(PlaylistId),
    /// Open the name dialog for a new playlist from library tracks
    RequestNewPlaylist(Vec<usize>),
    CreatePlaylist {
        name: String,
        tracks: Vec<usize>,
    },
    /// Library tracks — to the end of the playlist
    AddToPlaylist {
        id: PlaylistId,
        tracks: Vec<usize>,
    },
    /// Replace the queue with the playlist's tracks and play
    PlayPlaylist(PlaylistId),
    /// Play a playlist track: if it's not in the queue, it's appended
    PlayFromPlaylist {
        id: PlaylistId,
        index: usize,
    },
    /// Playlist tracks with the indices — to the end of the queue
    AddPlaylistTracksToQueue {
        id: PlaylistId,
        indices: Vec<usize>,
    },
    /// Playlist tracks with the indices — into the library (those not from it)
    AddPlaylistTracksToLibrary {
        id: PlaylistId,
        indices: Vec<usize>,
    },
    /// Remove tracks with the indices from the playlist
    RemoveFromPlaylist {
        id: PlaylistId,
        indices: BTreeSet<usize>,
    },
    RequestRenamePlaylist(PlaylistId),
    RenamePlaylist {
        id: PlaylistId,
        name: String,
    },
    /// Ask whether to delete playlists (see `dialogs::playlist`)
    RequestPlaylistDeletion(Vec<PlaylistId>),
    DeletePlaylists(Vec<PlaylistId>),

    // ---- player
    /// Playing — pause, otherwise — play
    TogglePlay,
    /// Play if not playing (the system "play" button, not a toggle)
    Play,
    Pause,
    Previous,
    Next,
    /// Seek the current track to a position in seconds
    Seek(f32),
    /// Volume 0.0..=1.0 — the slider position. Unmutes if muted
    SetVolume(f32),
    ToggleMute,
    /// Track finished — an audio thread event, not a user one: the next track in order
    TrackEnded,

    // ---- adding files
    /// Pick files in the system dialog and add them — to the library or to the queue
    AddFiles(Destination),
    /// Pick a folder and add all audio files in it
    AddFolder(Destination),
    /// Add dropped files and folders (folders — with all audio files inside)
    AddPaths {
        paths: Vec<PathBuf>,
        destination: Destination,
    },
    /// Background adding finished: put what was read where it belongs
    TracksLoaded {
        finished: Finished,
        destination: Destination,
    },
    /// Stop the current adding: what was already read is not added
    CancelLoading,

    // ---- settings
    /// UI language — from the next frame the whole UI uses it
    SetLanguage(Language),
    /// Window glass tint and transparency
    SetGlass(Glass),
    /// Enable or disable online search: artist photos, lyrics
    SetOnlineLookups(bool),

    /// Show or hide lyrics on the current track page
    ToggleLyrics,

    // ---- navigation
    /// Open a page (see `navigation`)
    Navigate(Route),
}

impl PlayerApp {
    /// Sends an action: it runs at the end of the frame
    pub(crate) fn dispatch(&mut self, action: Action) {
        self.actions.push(action);
    }

    /// Performs the actions accumulated during the frame. Called at the end of `ui` and at the end of `logic`:
    /// with a hidden window `ui` isn't called, but events (track finished, files added)
    /// happen then too
    pub(crate) fn apply_actions(&mut self, ctx: &egui::Context) {
        if self.actions.is_empty() {
            return;
        }
        for action in std::mem::take(&mut self.actions) {
            self.apply(action);
        }
        // The frame was already drawn before the actions — the next one shows their result right away,
        // without waiting for the mouse to move
        ctx.request_repaint();
    }

    /// Play the current track. No track — does nothing: space or the system button
    /// with an empty queue is no reason for an error
    fn play(&mut self) -> Result<(), String> {
        if self.core.controller.player().current_track.is_none() {
            return Ok(());
        }
        self.core.controller.play()
    }

    fn apply(&mut self, action: Action) {
        match action {
            Action::PlayLibraryTracks(indices) => {
                // The playlist's track indices change entirely — the old selection points to nothing
                self.queue_view.clear_selection();
                let result = self.core.play_library_tracks(&indices);
                self.report_error(result);
            }
            Action::PlayFromLibrary(index) => {
                let result = self.core.play_from_library(index);
                self.report_error(result);
            }
            Action::AddToQueue(indices) => {
                let result = self.core.add_library_tracks_to_queue(&indices);
                self.report_error(result);
            }
            Action::RequestSongsRemoval(indices) => self.request_songs_removal(&indices),
            Action::RequestGroupRemoval {
                kind,
                names,
                tracks,
            } => self.request_group_removal(kind, names, &tracks),
            Action::RemoveFromLibrary(paths) => {
                self.core.library.remove_paths(&paths);
                // Library track indices shifted — the selection in lists points to nothing
                self.library_view.clear_selection();
                self.album_tracks_view.clear_selection();
            }
            Action::PlayQueueTrack(index) => {
                let result = self
                    .core
                    .controller
                    .select(index)
                    .and_then(|()| self.core.controller.play());
                self.report_error(result);
            }
            Action::RemoveFromQueue(indices) => {
                let result = self.core.controller.remove_tracks(&indices);
                self.report_error(result);
                // Indices of the remaining tracks shifted
                self.queue_view.clear_selection();
            }
            Action::ClearQueue => {
                let result = self.core.controller.clear();
                self.report_error(result);
                self.queue_view.clear_selection();
            }

            Action::AddQueueTracksToLibrary(indices) => {
                self.core.add_queue_tracks_to_library(&indices);
            }

            Action::RequestSaveQueue => {
                self.name_dialog = Some(NameDialog::new(NamePurpose::SaveQueue, String::new()));
            }
            Action::SaveQueueAsPlaylist(name) => {
                let result = self.core.save_queue_as_playlist(&name).map(drop);
                self.report_error(result);
            }
            Action::SaveQueueToPlaylist(id) => {
                let result = self.core.save_queue_to_playlist(id);
                self.report_error(result);
            }
            Action::RequestNewPlaylist(tracks) => {
                self.name_dialog = Some(NameDialog::new(
                    NamePurpose::NewPlaylist(tracks),
                    String::new(),
                ));
            }
            Action::CreatePlaylist { name, tracks } => {
                let result = self
                    .core
                    .create_playlist_from_library(&name, &tracks)
                    .map(drop);
                self.report_error(result);
            }
            Action::AddToPlaylist { id, tracks } => {
                let result = self.core.add_library_tracks_to_playlist(id, &tracks);
                self.report_error(result);
            }
            Action::PlayPlaylist(id) => {
                self.queue_view.clear_selection();
                let result = self.core.play_playlist(id);
                self.report_error(result);
            }
            Action::PlayFromPlaylist { id, index } => {
                let result = self.core.play_from_playlist(id, index);
                self.report_error(result);
            }
            Action::AddPlaylistTracksToQueue { id, indices } => {
                let result = self.core.add_playlist_tracks_to_queue(id, &indices);
                self.report_error(result);
            }
            Action::AddPlaylistTracksToLibrary { id, indices } => {
                self.core.add_playlist_tracks_to_library(id, &indices);
            }
            Action::RemoveFromPlaylist { id, indices } => {
                let result = self.core.playlists.remove_tracks(id, &indices);
                self.report_error(result);
                // Indices of the remaining tracks shifted
                self.playlist_tracks_view.clear_selection();
            }
            Action::RequestRenamePlaylist(id) => {
                let name = self
                    .core
                    .playlists
                    .get(id)
                    .map(|playlist| playlist.name.clone())
                    .unwrap_or_default();
                self.name_dialog = Some(NameDialog::new(NamePurpose::Rename(id), name));
            }
            Action::RenamePlaylist { id, name } => {
                let result = self.core.playlists.rename(id, &name);
                self.report_error(result);
            }
            Action::RequestPlaylistDeletion(ids) => self.pending_playlist_deletion = ids,
            Action::DeletePlaylists(ids) => {
                let result = self.core.playlists.delete(&ids);
                self.report_error(result);
                // The deleted playlist's page was open — go to the playlist list
                if let Route::Playlist(id) = self.route
                    && ids.contains(&id)
                {
                    self.navigate(Route::Playlists);
                }
            }

            Action::ToggleShuffle => self.core.controller.toggle_shuffle(),
            Action::ToggleRepeat(mode) => self.core.controller.toggle_repeat(mode),

            Action::TogglePlay => {
                let result = if self.core.controller.player().state == PlayerState::Playing {
                    self.core.controller.pause()
                } else {
                    self.play()
                };
                self.report_error(result);
            }
            Action::Play => {
                let result = self.play();
                self.report_error(result);
            }
            Action::Pause => {
                let result = self.core.controller.pause();
                self.report_error(result);
            }
            Action::Previous => {
                let result = self.core.controller.skip_prev();
                self.report_error(result);
            }
            Action::Next => {
                let result = self.core.controller.skip_next();
                self.report_error(result);
            }
            Action::Seek(secs) => {
                let result = self.core.controller.seek((secs.max(0.0) * 1000.0) as u64);
                self.report_error(result);
            }
            Action::SetVolume(level) => self.core.controller.set_volume_level(level),
            Action::ToggleMute => self.core.controller.toggle_mute(),
            Action::TrackEnded => {
                let result = self.core.controller.play_next();
                self.report_error(result);
            }

            Action::AddFiles(destination) => self.add_files(destination),
            Action::AddFolder(destination) => self.add_folder(destination),
            Action::AddPaths { paths, destination } => self.add_paths(paths, destination),
            Action::TracksLoaded {
                finished,
                destination,
            } => self.apply_loaded(finished, destination),
            Action::CancelLoading => self.cancel_loading(),

            Action::SetLanguage(language) => {
                self.core.settings.update_language(language);
                i18n::set_language(language);
            }

            Action::SetGlass(glass) => self.core.settings.update_glass(glass),
            Action::SetOnlineLookups(online) => {
                self.core.set_online_lookups(online);
                // Lyrics may not have been found because search was off — look again
                self.current_lyrics = None;
            }

            Action::ToggleLyrics => self.lyrics_open = !self.lyrics_open,

            Action::ShowInLibrary(path) => self.show_in_library(&path),

            Action::Navigate(route) => self.navigate(route),
        }
    }
}

impl PlayerApp {
    /// Opens Songs and selects the track of file `path` in it, as if it were clicked.
    /// The search is reset: otherwise the track could be hidden
    fn show_in_library(&mut self, path: &std::path::Path) {
        let Some(index) = self
            .core
            .library
            .tracks()
            .iter()
            .position(|track| track.path == path)
        else {
            return;
        };
        if !self.search.is_empty() {
            self.search = Default::default();
        }
        self.navigate(Route::Songs);
        self.library_view.reveal(index);
        self.focus = ListFocus::Library;
    }
}
