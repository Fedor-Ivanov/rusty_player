//! The queue between launches: stored as JSON in the app data folder next to the library.
//! Saved right after each change, loaded at startup.

use serde::{Deserialize, Serialize};
use std::{fs, io::ErrorKind, path::PathBuf};

use super::queue::TrackList;
use crate::core::storage::write_atomically;

const FILE_NAME: &str = "playlist.json";
const FORMAT_VERSION: u32 = 1;

#[derive(Serialize)]
struct StoredRef<'a> {
    version: u32,
    playlist: &'a TrackList,
}

#[derive(Deserialize)]
struct Stored {
    version: u32,
    playlist: TrackList,
}

pub struct QueueStore {
    /// Where to save; `None` — the data folder is unavailable, the queue lives only until exit
    file: Option<PathBuf>,
}

impl QueueStore {
    pub fn new(dir: Option<PathBuf>) -> Self {
        Self {
            file: dir.map(|dir| dir.join(FILE_NAME)),
        }
    }

    /// The previous run's queue. `Ok(None)` — nothing saved (first launch or a file
    /// of an unknown version): start empty
    pub fn load(&self) -> Result<Option<TrackList>, String> {
        let Some(file) = &self.file else {
            return Ok(None);
        };
        let text = match fs::read_to_string(file) {
            Ok(text) => text,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("Could not read the saved playlist: {e}")),
        };
        let stored: Stored = serde_json::from_str(&text)
            .map_err(|e| format!("The saved playlist is damaged: {e}"))?;
        if stored.version != FORMAT_VERSION {
            return Ok(None);
        }
        Ok(Some(stored.playlist))
    }

    pub fn save(&self, playlist: &TrackList) -> Result<(), String> {
        let Some(file) = &self.file else {
            return Ok(());
        };
        let stored = StoredRef {
            version: FORMAT_VERSION,
            playlist,
        };
        write_atomically(file, &stored).map_err(|e| format!("Could not save the playlist: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::QueueStore;
    use crate::{
        core::library::Track,
        core::playback::{RepeatMode, queue::TrackList},
    };
    use std::{fs, path::PathBuf};

    #[test]
    fn saves_and_loads_playlist() {
        let dir = std::env::temp_dir().join(format!("lmp_playlist_{}", fastrand::u64(..)));
        let list = TrackList {
            tracks: vec![Track {
                path: PathBuf::from("/music/a.mp3"),
                title: "A".into(),
                artist: "Artist".into(),
                album_artist: String::new(),
                duration_secs: 60,
                year: None,
                track_number: None,
                disc_number: None,
                album: String::new(),
            }],
            current_index: Some(0),
            shuffle: true,
            repeat: RepeatMode::All,
            ..Default::default()
        };

        let store = QueueStore::new(Some(dir.clone()));
        assert!(store.load().unwrap().is_none());
        store.save(&list).unwrap();

        let loaded = QueueStore::new(Some(dir.clone())).load().unwrap().unwrap();
        assert_eq!(loaded.tracks.len(), 1);
        assert_eq!(loaded.tracks[0].title, "A");
        assert_eq!(loaded.current_index, Some(0));
        assert!(loaded.shuffle);
        assert_eq!(loaded.repeat, RepeatMode::All);

        fs::remove_dir_all(dir).unwrap();
    }
}
