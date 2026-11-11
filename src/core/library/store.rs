//! The library — tracks the user added "for good". Kept between launches
//! as a list of paths and tags in JSON in the app data folder; the files themselves stay where they are.
//!
//! The library can be huge, so slow work happens in the background: saving (the file is rewritten
//! entirely) and checking which files vanished from disk (one disk query per track).

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashSet},
    fs,
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{self, Receiver},
    },
};

use super::Track;
use crate::core::storage::BackgroundWriter;

const FILE_NAME: &str = "library.json";
/// File format version. 2 — tracks got a year, 3 — track and disc numbers,
/// 4 — album artist: an old-version library has to be re-read from tags once
/// (see `outdated_paths`)
const FORMAT_VERSION: u32 = 4;

#[derive(Deserialize)]
struct Stored {
    version: u32,
    tracks: Vec<Track>,
}

/// Same as `Stored`, for writing: tracks as a shared list, without a copy for every save
#[derive(Serialize)]
struct Snapshot {
    version: u32,
    tracks: Arc<Vec<Track>>,
}

pub struct Library {
    /// Shared with the background writer: it holds a snapshot while writing. A change during writing
    /// copies the list (`Arc::make_mut`), otherwise there's no copy
    tracks: Arc<Vec<Track>>,
    /// Paths of all tracks — so the same file isn't added twice
    paths: HashSet<PathBuf>,
    /// Files that weren't on disk at load time: they were moved or deleted.
    /// Filled by the background check (`check_missing_in_background`); empty until then
    missing: HashSet<PathBuf>,
    /// The background missing-files check's answer, until it arrives
    missing_check: Option<Receiver<HashSet<PathBuf>>>,
    /// Background writing to the library file; `None` — the data folder is unavailable, the library
    /// lives only until exit
    writer: Option<BackgroundWriter<Snapshot>>,
    /// Format version to save with. Until an old library is re-read from tags,
    /// it is saved with the old version — otherwise after a restart it would be considered updated
    version: u32,
    /// Grows on every change: it shows that derived data (groups by
    /// artists and albums) needs to be recomputed
    revision: u64,
}

impl Library {
    /// Loads the library from data folder `dir`. No file — an empty library.
    /// The second value is an error to show the user if something went wrong.
    pub fn load(dir: Option<PathBuf>) -> (Self, Option<String>) {
        let file = dir.map(|dir| dir.join(FILE_NAME));
        let mut library = Self {
            tracks: Arc::new(Vec::new()),
            paths: HashSet::new(),
            missing: HashSet::new(),
            missing_check: None,
            writer: file
                .clone()
                .map(|file| BackgroundWriter::new(file, "library")),
            version: FORMAT_VERSION,
            revision: 0,
        };
        let Some(file) = &file else {
            return (
                library,
                Some("Could not find a folder to store the library: it won't be saved".into()),
            );
        };

        let text = match fs::read_to_string(file) {
            Ok(text) => text,
            // First launch: no library yet
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (library, None),
            Err(e) => return (library, Some(format!("Could not read the library: {e}"))),
        };
        let stored: Stored = match serde_json::from_str(&text) {
            Ok(stored) => stored,
            Err(e) => {
                // Don't overwrite a corrupted file with an empty one on the first change:
                // move it aside so it can be recovered
                let backup = file.with_extension("json.bak");
                let _ = fs::rename(file, &backup);
                return (
                    library,
                    Some(format!(
                        "The library file is damaged and was set aside as {}: {e}",
                        backup.display()
                    )),
                );
            }
        };

        library.paths = stored.tracks.iter().map(|t| t.path.clone()).collect();
        library.tracks = Arc::new(stored.tracks);
        library.version = stored.version;
        (library, None)
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    /// Checks in the background which files are missing from disk: on a huge library this takes seconds,
    /// and startup doesn't wait for it. Until the answer arrives, all tracks are considered present. `wake` wakes
    /// the app when the answer is ready (see `poll_missing`)
    pub fn check_missing_in_background(&mut self, wake: impl Fn() + Send + 'static) {
        let paths: Vec<PathBuf> = self.tracks.iter().map(|t| t.path.clone()).collect();
        let (sender, receiver) = mpsc::channel();
        self.missing_check = Some(receiver);
        std::thread::spawn(move || {
            let missing = paths.into_iter().filter(|path| !path.exists()).collect();
            let _ = sender.send(missing);
            wake();
        });
    }

    /// Takes the background check's answer if it arrived. `true` — the missing-file
    /// marks were updated
    pub fn poll_missing(&mut self) -> bool {
        let Some(receiver) = &self.missing_check else {
            return false;
        };
        let Ok(missing) = receiver.try_recv() else {
            return false;
        };
        self.missing_check = None;
        // Tracks may have been removed from the library during the check — don't mark them
        self.missing = missing
            .into_iter()
            .filter(|path| self.paths.contains(path))
            .collect();
        true
    }

    /// Background writing error, if any — to show the user
    pub fn take_save_error(&self) -> Option<String> {
        self.writer.as_ref().and_then(BackgroundWriter::take_error)
    }

    /// Waits until all changes are saved: before exit
    pub fn flush(&self) {
        if let Some(writer) = &self.writer {
            writer.flush();
        }
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The library was saved by an old version and lacks data from tags — paths of all
    /// its tracks, to re-read them. `None` — the library is up to date
    pub fn outdated_paths(&self) -> Option<Vec<PathBuf>> {
        (self.version < FORMAT_VERSION && !self.tracks.is_empty())
            .then(|| self.tracks.iter().map(|t| t.path.clone()).collect())
    }

    /// Replaces track data with data re-read from tags (by path) and saves the library with
    /// the new version. Tracks missing from `fresh` (file vanished) stay as they were
    pub fn refresh(&mut self, fresh: Vec<Track>) {
        let mut fresh: std::collections::HashMap<PathBuf, Track> =
            fresh.into_iter().map(|t| (t.path.clone(), t)).collect();
        for track in Arc::make_mut(&mut self.tracks) {
            if let Some(updated) = fresh.remove(&track.path) {
                *track = updated;
            }
        }
        self.version = FORMAT_VERSION;
        self.revision += 1;
        self.save();
    }

    /// Paths already in the library: when a folder is added again they are not re-read
    pub fn paths(&self) -> &HashSet<PathBuf> {
        &self.paths
    }

    /// The track file wasn't on disk at load time (per the background check)
    pub fn is_missing(&self, index: usize) -> bool {
        self.tracks
            .get(index)
            .is_some_and(|track| self.missing.contains(&track.path))
    }

    /// Appends tracks, skipping those already present, and saves the library
    pub fn add(&mut self, tracks: Vec<Track>) {
        let new: Vec<Track> = tracks
            .into_iter()
            .filter(|track| self.paths.insert(track.path.clone()))
            .collect();
        for track in &new {
            self.missing.remove(&track.path);
        }
        if new.is_empty() {
            return;
        }
        Arc::make_mut(&mut self.tracks).extend(new);
        self.revision += 1;
        self.save();
    }

    /// Tracks with indices `indices` for playback — without those whose files are missing from disk:
    /// they couldn't be played anyway
    pub fn playable_tracks(&self, indices: &[usize]) -> Vec<Track> {
        indices
            .iter()
            .filter(|&&index| !self.is_missing(index))
            .filter_map(|&index| self.tracks.get(index).cloned())
            .collect()
    }

    /// Paths of tracks with indices `indices`. Removal asks for confirmation by paths, not
    /// by indices: while waiting for the answer, indices may change, but paths won't
    pub fn paths_of(&self, indices: &[usize]) -> HashSet<PathBuf> {
        indices
            .iter()
            .filter_map(|&index| self.tracks.get(index))
            .map(|track| track.path.clone())
            .collect()
    }

    /// Indices of the tracks of files `paths` — in library order. Files not in the library
    /// are not in the answer
    pub fn indices_of(&self, paths: &HashSet<PathBuf>) -> Vec<usize> {
        self.tracks
            .iter()
            .enumerate()
            .filter(|(_, track)| paths.contains(&track.path))
            .map(|(index, _)| index)
            .collect()
    }

    /// Removes the tracks of files `paths` (doesn't touch the files themselves) and saves the library
    pub fn remove_paths(&mut self, paths: &HashSet<PathBuf>) {
        let indices = self.indices_of(paths).into_iter().collect();
        self.remove(&indices);
    }

    /// Removes tracks with indices `indices` (doesn't touch the files themselves) and saves the library
    pub fn remove(&mut self, indices: &BTreeSet<usize>) {
        let mut index = 0;
        let (paths, missing) = (&mut self.paths, &mut self.missing);
        Arc::make_mut(&mut self.tracks).retain(|track| {
            let keep = !indices.contains(&index);
            index += 1;
            if !keep {
                paths.remove(&track.path);
                missing.remove(&track.path);
            }
            keep
        });
        self.revision += 1;
        self.save();
    }

    /// Sends the library to be written in the background. An error comes later — `take_save_error`
    fn save(&self) {
        if let Some(writer) = &self.writer {
            writer.write(Snapshot {
                version: self.version,
                tracks: Arc::clone(&self.tracks),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Library;
    use crate::core::library::Track;
    use std::{collections::BTreeSet, fs, path::Path};

    fn track(path: &Path) -> Track {
        Track {
            path: path.to_path_buf(),
            title: path.file_stem().unwrap().to_string_lossy().into_owned(),
            artist: "Artist".into(),
            album_artist: String::new(),
            duration_secs: 60,
            year: None,
            track_number: None,
            disc_number: None,
            album: String::new(),
        }
    }

    fn titles(library: &Library) -> Vec<&str> {
        library.tracks().iter().map(|t| t.title.as_str()).collect()
    }

    #[test]
    fn saves_loads_dedupes_and_marks_missing() {
        let dir = std::env::temp_dir().join(format!("lmp_library_{}", std::process::id()));
        let music = dir.join("music");
        fs::create_dir_all(&music).unwrap();
        let [a, b, c] = ["a", "b", "c"].map(|name| music.join(format!("{name}.mp3")));
        for path in [&a, &b, &c] {
            fs::write(path, b"").unwrap();
        }

        let (mut library, error) = Library::load(Some(dir.clone()));
        assert!(error.is_none());
        library.add(vec![track(&a), track(&b)]);
        // Adding the same file again is skipped
        library.add(vec![track(&b), track(&c)]);
        assert_eq!(titles(&library), ["a", "b", "c"]);

        library.remove(&BTreeSet::from([0]));
        library.flush();
        fs::remove_file(&c).unwrap();

        // After a restart — the same contents, and the vanished file is marked — once the
        // background check finishes
        let (mut library, error) = Library::load(Some(dir.clone()));
        assert!(error.is_none());
        assert_eq!(titles(&library), ["b", "c"]);
        assert!(!library.is_missing(1));
        library.check_missing_in_background(|| {});
        while !library.poll_missing() {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(!library.is_missing(0));
        assert!(library.is_missing(1));
        assert!(library.paths().contains(&b));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn old_library_is_refreshed_once() {
        let dir = std::env::temp_dir().join(format!("lmp_v1_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.mp3");
        // A format 1 library: the track has no year
        fs::write(
            dir.join("library.json"),
            format!(
                r#"{{"version":1,"tracks":[{{"path":{:?},"title":"a","artist":"","duration_secs":1,"album":""}}]}}"#,
                path
            ),
        )
        .unwrap();

        let (mut library, error) = Library::load(Some(dir.clone()));
        assert!(error.is_none());
        assert_eq!(library.tracks()[0].year, None);
        assert_eq!(library.outdated_paths(), Some(vec![path.clone()]));

        // Adding before the update must not mark the library as updated
        library.add(vec![track(&dir.join("b.mp3"))]);
        library.flush();
        let (mut library, _) = Library::load(Some(dir.clone()));
        assert!(library.outdated_paths().is_some());

        let mut fresh = track(&path);
        fresh.year = Some(1999);
        library.refresh(vec![fresh]);
        library.flush();
        let (library, _) = Library::load(Some(dir.clone()));
        assert_eq!(library.outdated_paths(), None);
        assert_eq!(library.tracks()[0].year, Some(1999));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn damaged_file_is_set_aside() {
        let dir = std::env::temp_dir().join(format!("lmp_damaged_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("library.json"), "not json").unwrap();

        let (library, error) = Library::load(Some(dir.clone()));
        assert!(library.tracks().is_empty());
        assert!(error.unwrap().contains("damaged"));
        assert!(dir.join("library.json.bak").exists());

        fs::remove_dir_all(&dir).unwrap();
    }
}
