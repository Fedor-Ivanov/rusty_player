//! Player logic without UI: library, playback, settings and saving them
//! to disk — and `Core`, which owns them all. Knows nothing about egui: the UI
//! (`crate::ui`) only reads data from here and calls methods. Tested without a window.

pub mod artist_images;
pub mod library;
pub mod lyrics;
pub mod net;
pub mod playback;
pub mod playlists;
pub mod settings;
pub mod storage;

use std::{path::PathBuf, sync::Arc};

use self::{
    artist_images::{ArtistImage, ArtistImages, ArtistJob},
    library::{
        Library, Track,
        groups::{self, AlbumGroup, ArtistGroup},
    },
    lyrics::LyricsFinder,
    playback::{PlayerController, QueueStore},
    playlists::{PlaylistId, Playlists},
    settings::{Settings, SettingsStore},
};

/// The app core: owns the library, saved playlists, the player with its queue and the settings, loads
/// and saves them. Each part knows only itself and is changed through its own methods;
/// here are the cross-part operations (library tracks → player) and saving
pub struct Core {
    /// Tracks kept between launches
    pub library: Library,
    /// Saved playlists
    pub playlists: Playlists,
    /// Player and playback queue
    pub controller: PlayerController,
    /// Settings. Volume lives in the player, it is copied into settings before saving
    pub settings: Settings,
    /// Library artists and albums — recomputed (`refresh_groups`) only when
    /// the library changed
    pub groups: LibraryGroups,
    /// Lyrics: local and from LRCLIB, with an answer cache
    pub lyrics: LyricsFinder,
    /// Artist images: local and from Deezer, looked up in the background
    pub artist_images: ArtistImages,
    queue_store: QueueStore,
    /// Queue revision already saved to disk
    saved_queue_revision: u64,
    settings_store: SettingsStore,
}

#[derive(Default)]
pub struct LibraryGroups {
    /// Library revision the groups were computed for; `None` — not computed yet
    pub revision: Option<u64>,
    pub artists: Vec<ArtistGroup>,
    pub albums: Vec<AlbumGroup>,
}

impl Core {
    /// Loads what's saved in data folder `dir`: the library, playlists, settings and the queue of the previous
    /// run. `None` — the folder is unavailable: everything lives only until exit. `wake` is called
    /// from background threads: a track finished (see `PlayerController::new`), lyrics arrived.
    /// The second value is the first load error, to show the user
    pub fn load(
        dir: Option<PathBuf>,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> (Self, Option<String>) {
        let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(wake);
        let (mut library, mut error) = Library::load(dir.clone());
        library.check_missing_in_background({
            let wake = Arc::clone(&wake);
            move || wake()
        });
        let (settings_store, settings, settings_error) = SettingsStore::load(dir.clone());
        let (playlists, playlists_error) = Playlists::load(dir.clone());
        for e in [settings_error, playlists_error].into_iter().flatten() {
            error.get_or_insert(e);
        }
        let mut lyrics = LyricsFinder::new(dir.clone(), Arc::clone(&wake));
        let mut artist_images = ArtistImages::new(dir.clone(), Arc::clone(&wake));
        lyrics.set_online(settings.online_lookups());
        artist_images.set_online(settings.online_lookups());
        let queue_store = QueueStore::new(dir);
        let saved_queue = queue_store.load().unwrap_or_else(|e| {
            error.get_or_insert(e);
            None
        });

        let mut controller = PlayerController::new(move || wake());
        if let Some(list) = saved_queue {
            controller.restore_track_list(list);
        }
        controller.set_volume(settings.volume());
        let state = Self {
            library,
            playlists,
            // The restored queue is already on disk
            saved_queue_revision: controller.queue_revision(),
            controller,
            settings,
            groups: LibraryGroups::default(),
            lyrics,
            artist_images,
            queue_store,
            settings_store,
        };
        (state, error)
    }

    /// Takes the results of background work: which library files vanished from disk, ready
    /// artist images. Call
    /// every frame. Returns the background writing error, if any — to show the user
    pub fn poll_background(&mut self) -> Option<String> {
        self.library.poll_missing();
        self.artist_images.poll();
        self.library.take_save_error()
    }

    /// Waits for the background library write — before exit, so the latest changes aren't lost
    pub fn flush(&self) {
        self.library.flush();
    }

    /// Recomputes artists and albums if the library changed
    pub fn refresh_groups(&mut self) {
        let revision = self.library.revision();
        if self.groups.revision != Some(revision) {
            let (artists, albums) = groups::group(self.library.tracks());
            self.groups = LibraryGroups {
                revision: Some(revision),
                artists,
                albums,
            };
            // Artists changed — images of new ones are looked up in the background
            self.prefetch_artist_images();
        }
    }

    /// Enables or disables online search: artist photos and lyrics.
    /// After enabling, the background search continues with those not yet looked up
    pub fn set_online_lookups(&mut self, online: bool) {
        self.settings.update_online_lookups(online);
        self.lyrics.set_online(online);
        self.artist_images.set_online(online);
        if online && self.groups.revision.is_some() {
            self.prefetch_artist_images();
        }
    }

    /// Queues all library artists for background image lookup — known ones
    /// the queue skips
    fn prefetch_artist_images(&mut self) {
        let jobs: Vec<ArtistJob> = self
            .groups
            .artists
            .iter()
            .map(|artist| ArtistJob {
                name: artist.name.clone(),
                music_dirs: music_dirs(&self.library, &artist.tracks),
            })
            .collect();
        self.artist_images.prefetch(jobs);
    }

    /// The track's album in the library — looked up the same way as in grouping: case-insensitive.
    /// A track without album or artist in tags has no album: it's in "Unknown Album" together
    /// with other untagged tracks (see `groups::has_known_album`)
    pub fn album_of(&mut self, track: &Track) -> Option<&AlbumGroup> {
        if !groups::has_known_album(track) {
            return None;
        }
        self.refresh_groups();
        let title = track.album.trim().to_lowercase();
        let artist = groups::album_artist(track).to_lowercase();
        self.groups.albums.iter().find(|album| {
            album.title.to_lowercase() == title && album.artist.to_lowercase() == artist
        })
    }

    /// Image of artist `name` to show: ready, being looked up, or none. Not ready —
    /// the artist goes to the front of the background queue
    pub fn artist_image(&mut self, name: &str) -> ArtistImage {
        let (images, library, groups) = (&mut self.artist_images, &self.library, &self.groups);
        images.image(name, || {
            groups
                .artists
                .iter()
                .find(|artist| artist.name == name)
                .map(|artist| music_dirs(library, &artist.tracks))
                .unwrap_or_default()
        })
    }

    /// Replaces the queue with library tracks `indices` and plays from the first
    pub fn play_library_tracks(&mut self, indices: &[usize]) -> Result<(), String> {
        let tracks = self.library.playable_tracks(indices);
        self.controller.play_all(tracks)
    }

    /// A library track plays immediately — see `play_track`
    pub fn play_from_library(&mut self, index: usize) -> Result<(), String> {
        match self.library.tracks().get(index).cloned() {
            Some(track) => self.play_track(track),
            None => Ok(()),
        }
    }

    /// Track `index` of saved playlist `id` plays immediately — see `play_track`
    pub fn play_from_playlist(&mut self, id: PlaylistId, index: usize) -> Result<(), String> {
        let track = self
            .playlists
            .get(id)
            .and_then(|playlist| playlist.tracks.get(index))
            .cloned();
        match track {
            Some(track) => self.play_track(track),
            None => Ok(()),
        }
    }

    /// The track plays immediately. If it's already in the queue, it plays from there, otherwise it's appended —
    /// playing it again doesn't create copies in the queue
    fn play_track(&mut self, track: Track) -> Result<(), String> {
        let queue = &self.controller.player().track_list.tracks;
        let queue_index = match queue.iter().position(|t| t.path == track.path) {
            Some(existing) => existing,
            None => {
                self.controller.append_tracks([track]);
                self.controller.player().track_list.tracks.len() - 1
            }
        };
        self.controller.select(queue_index)?;
        self.controller.play()
    }

    /// Replaces the queue with the tracks of playlist `id` and plays from the first. The queue gets linked
    /// to the playlist: if it's changed, "save" writes the changes into this playlist
    pub fn play_playlist(&mut self, id: PlaylistId) -> Result<(), String> {
        let tracks = self.playlists.playable_tracks(id);
        let result = self.controller.play_all(tracks);
        self.controller.link_playlist(Some(id));
        result
    }

    /// The playlist the queue is linked to — if it still exists
    pub fn queue_playlist(&self) -> Option<PlaylistId> {
        let id = self.controller.player().track_list.playlist?;
        self.playlists.get(id).map(|playlist| playlist.id)
    }

    /// The queue is linked to a playlist and hasn't changed since the last save: it has the same
    /// tracks in the same order as the playlist. Nothing to save then
    pub fn queue_matches_playlist(&self) -> bool {
        let Some(playlist) = self.queue_playlist().and_then(|id| self.playlists.get(id)) else {
            return false;
        };
        let queue = &self.controller.player().track_list.tracks;
        queue.len() == playlist.tracks.len()
            && queue
                .iter()
                .zip(&playlist.tracks)
                .all(|(a, b)| a.path == b.path)
    }

    /// Overwrites playlist `id` with the queue tracks
    pub fn save_queue_to_playlist(&mut self, id: PlaylistId) -> Result<(), String> {
        let tracks = self.controller.player().track_list.tracks.clone();
        self.playlists.set_tracks(id, tracks)
    }

    /// Tracks `indices` of playlist `id` — to the end of the queue (without missing files)
    pub fn add_playlist_tracks_to_queue(
        &mut self,
        id: PlaylistId,
        indices: &[usize],
    ) -> Result<(), String> {
        let Some(playlist) = self.playlists.get(id) else {
            return Ok(());
        };
        let tracks = indices
            .iter()
            .filter_map(|&index| playlist.tracks.get(index))
            .filter(|track| !self.playlists.is_missing(track))
            .cloned()
            .collect();
        self.controller.add_tracks(tracks)
    }

    /// Tracks `indices` of playlist `id` — into the library. Those already in it are not
    /// duplicated
    pub fn add_playlist_tracks_to_library(&mut self, id: PlaylistId, indices: &[usize]) {
        let Some(playlist) = self.playlists.get(id) else {
            return;
        };
        let tracks = indices
            .iter()
            .filter_map(|&index| playlist.tracks.get(index).cloned())
            .collect();
        self.library.add(tracks);
    }

    /// Queue tracks `indices` — into the library. Those already in it are not duplicated
    pub fn add_queue_tracks_to_library(&mut self, indices: &[usize]) {
        let queue = &self.controller.player().track_list.tracks;
        let tracks = indices
            .iter()
            .filter_map(|&index| queue.get(index).cloned())
            .collect();
        self.library.add(tracks);
    }

    /// Saves the queue as a new playlist `name` and links it to it: the next
    /// "save" overwrites this playlist instead of creating another one
    pub fn save_queue_as_playlist(&mut self, name: &str) -> Result<PlaylistId, String> {
        let tracks = self.controller.player().track_list.tracks.clone();
        let id = self.playlists.create(name, tracks)?;
        self.controller.link_playlist(Some(id));
        Ok(id)
    }

    /// Library tracks `indices` — to the end of playlist `id`. Missing files too: the playlist
    /// keeps what was chosen, and playback won't let the missing ones play
    pub fn add_library_tracks_to_playlist(
        &mut self,
        id: PlaylistId,
        indices: &[usize],
    ) -> Result<(), String> {
        let tracks = self.library_tracks(indices);
        self.playlists.add_tracks(id, tracks)
    }

    /// A new playlist `name` from library tracks `indices`
    pub fn create_playlist_from_library(
        &mut self,
        name: &str,
        indices: &[usize],
    ) -> Result<PlaylistId, String> {
        let tracks = self.library_tracks(indices);
        self.playlists.create(name, tracks)
    }

    fn library_tracks(&self, indices: &[usize]) -> Vec<Track> {
        indices
            .iter()
            .filter_map(|&index| self.library.tracks().get(index).cloned())
            .collect()
    }

    /// Library tracks `indices` — to the end of the queue
    pub fn add_library_tracks_to_queue(&mut self, indices: &[usize]) -> Result<(), String> {
        let tracks = self.library.playable_tracks(indices);
        self.controller.add_tracks(tracks)
    }

    /// Saves the queue if it changed since the last save. However many changes
    /// happen between calls (e.g. "Play" both changes tracks and selects the current one),
    /// the file is written once
    pub fn save_queue_if_changed(&mut self) -> Result<(), String> {
        let revision = self.controller.queue_revision();
        if revision == self.saved_queue_revision {
            return Ok(());
        }
        // The revision is considered saved even on error: otherwise writing would repeat on every
        // call. The next change will try again
        self.saved_queue_revision = revision;
        self.queue_store.save(&self.controller.player().track_list)
    }

    /// Saves settings if they changed — together with the player's current volume
    pub fn save_settings_if_changed(&mut self) -> Result<(), String> {
        self.settings.update_volume(self.controller.volume());
        self.settings_store.save_if_changed(&self.settings)
    }
}

/// Folders to search for the local artist image: folders of the artist's tracks (albums) and folders
/// above them (the artist). The first tracks are enough — the rest are usually in the same folders
fn music_dirs(library: &Library, tracks: &[usize]) -> Vec<PathBuf> {
    const TRACKS_TO_LOOK_AT: usize = 20;
    let mut dirs: Vec<PathBuf> = Vec::new();
    for index in tracks.iter().take(TRACKS_TO_LOOK_AT) {
        let Some(album_dir) = library
            .tracks()
            .get(*index)
            .and_then(|track| track.path.parent())
        else {
            continue;
        };
        for dir in [Some(album_dir), album_dir.parent()].into_iter().flatten() {
            if !dirs.iter().any(|known| known == dir) {
                dirs.push(dir.to_path_buf());
            }
        }
    }
    dirs
}

#[cfg(test)]
mod tests {
    use super::Core;
    use crate::core::library::Track;
    use std::{fs, path::PathBuf};

    /// State in a new empty temp folder
    fn new_core() -> (Core, PathBuf) {
        let dir = std::env::temp_dir().join(format!("lmp_core_{}", fastrand::u64(..)));
        let (core, error) = Core::load(Some(dir.clone()), || {});
        assert!(error.is_none(), "{error:?}");
        (core, dir)
    }

    /// End of test: waits for the background library write (`Drop`) and removes the data folder.
    /// The folder may not exist — the test didn't save anything
    fn finish(core: Core, dir: PathBuf) {
        drop(core);
        let _ = fs::remove_dir_all(dir);
    }

    fn track(name: &str) -> Track {
        Track {
            path: PathBuf::from(format!("/music/{name}.mp3")),
            title: name.into(),
            artist: "Artist".into(),
            album_artist: String::new(),
            duration_secs: 60,
            album: "Album".into(),
            year: None,
            track_number: None,
            disc_number: None,
        }
    }

    fn playlist_titles(core: &Core) -> Vec<String> {
        let list = &core.controller.player().track_list;
        list.tracks.iter().map(|t| t.title.clone()).collect()
    }

    fn current_title(core: &Core) -> Option<String> {
        let player = core.controller.player();
        player.current_track.as_ref().map(|t| t.title.clone())
    }

    #[test]
    fn play_library_tracks_replaces_playlist() {
        let (mut core, dir) = new_core();
        core.library.add(vec![track("a"), track("b"), track("c")]);
        core.add_library_tracks_to_queue(&[0]).unwrap();

        // No files — can't play, but the queue is already replaced and the track selected
        let _ = core.play_library_tracks(&[2, 1]);
        assert_eq!(playlist_titles(&core), ["c", "b"]);
        assert_eq!(current_title(&core).as_deref(), Some("c"));
        finish(core, dir);
    }

    #[test]
    fn adding_to_empty_playlist_selects_first_track() {
        let (mut core, dir) = new_core();
        core.library.add(vec![track("a"), track("b")]);
        core.add_library_tracks_to_queue(&[1, 0]).unwrap();
        assert_eq!(playlist_titles(&core), ["b", "a"]);
        assert_eq!(current_title(&core).as_deref(), Some("b"));

        // The current track is already present — adding it changes nothing
        core.add_library_tracks_to_queue(&[0]).unwrap();
        assert_eq!(playlist_titles(&core), ["b", "a", "a"]);
        assert_eq!(current_title(&core).as_deref(), Some("b"));
        finish(core, dir);
    }

    #[test]
    fn play_from_library_does_not_duplicate() {
        let (mut core, dir) = new_core();
        core.library.add(vec![track("a"), track("b")]);
        let _ = core.play_from_library(1);
        let _ = core.play_from_library(0);
        let _ = core.play_from_library(1);
        assert_eq!(playlist_titles(&core), ["b", "a"]);
        assert_eq!(current_title(&core).as_deref(), Some("b"));
        finish(core, dir);
    }

    #[test]
    fn removing_from_library_keeps_playlist() {
        let (mut core, dir) = new_core();
        core.library.add(vec![track("a"), track("b")]);
        core.add_library_tracks_to_queue(&[0, 1]).unwrap();

        let paths = core.library.paths_of(&[0]);
        core.library.remove_paths(&paths);
        let titles: Vec<_> = core.library.tracks().iter().map(|t| &t.title).collect();
        assert_eq!(titles, ["b"]);
        assert_eq!(playlist_titles(&core), ["a", "b"]);
        finish(core, dir);
    }

    #[test]
    fn playlist_and_settings_survive_restart() {
        let (mut core, dir) = new_core();
        core.library.add(vec![track("a"), track("b")]);
        core.add_library_tracks_to_queue(&[0, 1]).unwrap();
        core.controller.set_volume_level(0.3);
        core.save_queue_if_changed().unwrap();
        core.save_settings_if_changed().unwrap();

        let (restored, _) = Core::load(Some(dir.clone()), || {});
        assert_eq!(playlist_titles(&restored), ["a", "b"]);
        assert_eq!(current_title(&restored).as_deref(), Some("a"));
        assert_eq!(restored.library.tracks().len(), 2);
        assert_eq!(restored.controller.volume().level, 0.3);
        finish(core, dir);
    }

    #[test]
    fn queue_saved_as_playlist_plays_again_after_clear() {
        let (mut core, dir) = new_core();
        core.library.add(vec![track("a"), track("b")]);
        core.add_library_tracks_to_queue(&[1, 0]).unwrap();
        let id = core.save_queue_as_playlist("Mix").unwrap();

        core.controller.clear().unwrap();
        assert!(playlist_titles(&core).is_empty());
        assert!(current_title(&core).is_none());

        let _ = core.play_playlist(id);
        assert_eq!(playlist_titles(&core), ["b", "a"]);
        assert_eq!(current_title(&core).as_deref(), Some("b"));

        // A library track goes into a playlist, and the playlist survives a restart
        core.add_library_tracks_to_playlist(id, &[0]).unwrap();
        let (restored, _) = Core::load(Some(dir.clone()), || {});
        let saved = restored.playlists.get(id).unwrap();
        assert_eq!(saved.name, "Mix");
        let titles: Vec<_> = saved.tracks.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(titles, ["b", "a", "a"]);
        finish(core, dir);
    }

    #[test]
    fn saving_linked_queue_updates_its_playlist() {
        let (mut core, dir) = new_core();
        core.library.add(vec![track("a"), track("b"), track("c")]);
        core.add_library_tracks_to_queue(&[0]).unwrap();
        let id = core.save_queue_as_playlist("Mix").unwrap();
        assert_eq!(core.queue_playlist(), Some(id));
        // Just saved — nothing to save
        assert!(core.queue_matches_playlist());

        // Appended to the queue — saving overwrites the same playlist
        core.add_library_tracks_to_queue(&[1]).unwrap();
        assert!(!core.queue_matches_playlist());
        core.save_queue_to_playlist(id).unwrap();
        assert!(core.queue_matches_playlist());
        assert_eq!(core.playlists.all().len(), 1);
        let titles: Vec<_> = core
            .playlists
            .get(id)
            .unwrap()
            .tracks
            .iter()
            .map(|t| t.title.clone())
            .collect();
        assert_eq!(titles, ["a", "b"]);

        // The link survives a restart
        core.save_queue_if_changed().unwrap();
        let (mut restored, _) = Core::load(Some(dir.clone()), || {});
        assert_eq!(restored.queue_playlist(), Some(id));

        // The queue was replaced with other content — no link anymore; started a playlist — linked again
        let _ = restored.play_library_tracks(&[2]);
        assert_eq!(restored.queue_playlist(), None);
        let _ = restored.play_playlist(id);
        assert_eq!(restored.queue_playlist(), Some(id));
        restored.controller.clear().unwrap();
        assert_eq!(restored.queue_playlist(), None);
        finish(core, dir);
    }

    #[test]
    fn playlist_tracks_outside_library_can_be_added_to_it() {
        let (mut core, dir) = new_core();
        core.library.add(vec![track("a")]);
        // "b" was added to the queue bypassing the library and saved to the playlist together with "a"
        core.add_library_tracks_to_queue(&[0]).unwrap();
        core.controller.add_tracks(vec![track("b")]).unwrap();
        let id = core.save_queue_as_playlist("Mix").unwrap();
        assert_eq!(core.library.tracks().len(), 1);

        core.add_playlist_tracks_to_library(id, &[0, 1]);
        let titles: Vec<_> = core
            .library
            .tracks()
            .iter()
            .map(|t| t.title.as_str())
            .collect();
        assert_eq!(titles, ["a", "b"]);
        finish(core, dir);
    }
}
