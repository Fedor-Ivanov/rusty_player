//! Lyrics for a track: local (`.lrc` file, tags) or from LRCLIB. LRCLIB answers are stored
//! in the data folder (`lyrics/`, a file per track) — each track is queried once. Requests run
//! in the background: until the answer arrives, what's available locally is shown.

use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
    time::{Duration, Instant},
};

use super::{
    Lyrics, load_lyrics,
    lrclib::{self, Remote, TrackSignature},
    parse_lyrics,
};
use crate::core::{
    library::Track,
    storage::{now_secs, stable_hash, write_atomically},
};

const DIR_NAME: &str = "lyrics";
const FORMAT_VERSION: u32 = 1;
/// "Not found" is remembered for this long: then ask again — lyrics may have been added
const NOT_FOUND_TTL: Duration = Duration::from_secs(14 * 24 * 60 * 60);
/// Network or server failed (LRCLIB often answers "busy") — the request is retried
/// with these pauses
const RETRY_DELAYS: [Duration; 3] = [
    Duration::from_secs(2),
    Duration::from_secs(5),
    Duration::from_secs(10),
];
/// Failed even after retries — ask again no sooner than after this long
const FAILED_RETRY_AFTER: Duration = Duration::from_secs(5 * 60);

/// What to show for a track
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrackLyrics {
    /// The best of the found lyrics: synced wins over plain
    pub lyrics: Option<Lyrics>,
    /// LRCLIB is still answering — lyrics may appear or be replaced with synced ones
    pub searching: bool,
    /// LRCLIB knows the track is instrumental
    pub instrumental: bool,
    /// LRCLIB didn't answer recently (network, server busy) — ask later
    pub unavailable: bool,
    /// LRCLIB can't be asked: online search is disabled in settings
    pub offline: bool,
}

/// Cached LRCLIB answer
#[derive(Serialize, Deserialize)]
struct Cached {
    version: u32,
    /// When it was asked — Unix seconds
    checked_at: u64,
    remote: Remote,
}

pub struct LyricsFinder {
    /// Cache folder; `None` — the data folder is unavailable, answers live until exit
    dir: Option<PathBuf>,
    /// Answers known in this run: read from the cache and received
    known: HashMap<TrackSignature, Remote>,
    /// Requests in flight
    pending: HashSet<TrackSignature>,
    /// Failed requests (network, server busy) and when: ask again after `FAILED_RETRY_AFTER`
    failed: HashMap<TrackSignature, Instant>,
    sender: Sender<(TrackSignature, Result<Remote, String>)>,
    results: Receiver<(TrackSignature, Result<Remote, String>)>,
    /// Wakes the app when an answer arrives: it re-reads the lyrics
    wake: Arc<dyn Fn() + Send + Sync>,
    /// Whether LRCLIB may be asked — online search in settings. Without it — local lyrics
    /// and already downloaded ones
    online: bool,
}

impl LyricsFinder {
    pub fn new(data_dir: Option<PathBuf>, wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        let (sender, results) = mpsc::channel();
        Self {
            dir: data_dir.map(|dir| dir.join(DIR_NAME)),
            known: HashMap::new(),
            pending: HashSet::new(),
            failed: HashMap::new(),
            sender,
            results,
            wake,
            online: false,
        }
    }

    pub fn set_online(&mut self, online: bool) {
        self.online = online;
    }

    /// Lyrics for a track. A local synced one is used right away. Otherwise ask LRCLIB
    /// (from cache or with a background request): its synced lyrics win over local plain ones
    pub fn find(&mut self, track: &Track) -> TrackLyrics {
        let local = load_lyrics(&track.path);
        if local.as_ref().is_some_and(|lyrics| lyrics.synced) {
            return TrackLyrics {
                lyrics: local,
                ..TrackLyrics::default()
            };
        }
        let Some(signature) = signature(track) else {
            return TrackLyrics {
                lyrics: local,
                ..TrackLyrics::default()
            };
        };
        match self.remote(&signature) {
            Some(Remote::Found {
                synced,
                plain,
                instrumental,
            }) => {
                let synced = synced.as_deref().and_then(parse_lyrics);
                let plain = plain.as_deref().and_then(parse_lyrics);
                TrackLyrics {
                    lyrics: synced.or(local).or(plain),
                    instrumental,
                    ..TrackLyrics::default()
                }
            }
            Some(Remote::NotFound) => TrackLyrics {
                lyrics: local,
                ..TrackLyrics::default()
            },
            None if !self.online => TrackLyrics {
                lyrics: local,
                offline: true,
                ..TrackLyrics::default()
            },
            None => {
                let searching = self.request(signature);
                TrackLyrics {
                    lyrics: local,
                    searching,
                    unavailable: !searching,
                    ..TrackLyrics::default()
                }
            }
        }
    }

    /// Takes the arrived answers and caches them. `true` — something arrived: the track's
    /// lyrics should be looked up again
    pub fn poll(&mut self) -> bool {
        let mut arrived = false;
        while let Ok((signature, result)) = self.results.try_recv() {
            self.pending.remove(&signature);
            arrived = true;
            match result {
                Ok(remote) => {
                    self.save(&signature, &remote);
                    self.known.insert(signature, remote);
                }
                Err(e) => {
                    eprintln!("Could not get lyrics for {}: {e}", signature.title);
                    self.failed.insert(signature, Instant::now());
                }
            }
        }
        arrived
    }

    /// LRCLIB answer if already known: in this run or from the cache, until stale
    fn remote(&mut self, signature: &TrackSignature) -> Option<Remote> {
        if let Some(remote) = self.known.get(signature) {
            return Some(remote.clone());
        }
        let file = self.dir.as_ref()?.join(file_name(signature));
        let cached: Cached = serde_json::from_str(&fs::read_to_string(file).ok()?).ok()?;
        let age = now_secs().saturating_sub(cached.checked_at);
        if cached.remote == Remote::NotFound && age > NOT_FOUND_TTL.as_secs() {
            return None;
        }
        self.known.insert(signature.clone(), cached.remote.clone());
        Some(cached.remote)
    }

    /// Asks LRCLIB in the background if not asked yet. `true` — waiting for the answer
    fn request(&mut self, signature: TrackSignature) -> bool {
        if let Some(failed_at) = self.failed.get(&signature) {
            if failed_at.elapsed() < FAILED_RETRY_AFTER {
                return false;
            }
            self.failed.remove(&signature);
        }
        if self.pending.insert(signature.clone()) {
            let sender = self.sender.clone();
            let wake = Arc::clone(&self.wake);
            std::thread::spawn(move || {
                let mut result = lrclib::fetch(&signature);
                for delay in RETRY_DELAYS {
                    if result.is_ok() {
                        break;
                    }
                    std::thread::sleep(delay);
                    result = lrclib::fetch(&signature);
                }
                let _ = sender.send((signature, result));
                wake();
            });
        }
        true
    }

    fn save(&self, signature: &TrackSignature, remote: &Remote) {
        let Some(dir) = &self.dir else {
            return;
        };
        let cached = Cached {
            version: FORMAT_VERSION,
            checked_at: now_secs(),
            remote: remote.clone(),
        };
        // The cache is not user data: if it wasn't saved — we'll ask again some other time
        if let Err(e) = write_atomically(&dir.join(file_name(signature)), &cached) {
            eprintln!("Could not save the lyrics: {e}");
        }
    }
}

/// Track signature for LRCLIB. Without an artist or title there's no point searching
fn signature(track: &Track) -> Option<TrackSignature> {
    let artist = track.artist.trim();
    let title = track.title.trim();
    if artist.is_empty() || title.is_empty() {
        return None;
    }
    Some(TrackSignature {
        artist: artist.to_owned(),
        title: title.to_owned(),
        album: track.album.trim().to_owned(),
        duration_secs: track.duration_secs,
    })
}

/// Cache file name — a case-insensitive hash of the signature
fn file_name(signature: &TrackSignature) -> String {
    let key = format!(
        "{}\u{1f}{}\u{1f}{}\u{1f}{}",
        signature.artist.to_lowercase(),
        signature.title.to_lowercase(),
        signature.album.to_lowercase(),
        signature.duration_secs
    );
    format!("{:016x}.json", stable_hash(&key))
}

#[cfg(test)]
mod tests {
    use super::{LyricsFinder, Remote, TrackSignature, file_name};
    use std::{fs, sync::Arc};

    fn signature() -> TrackSignature {
        TrackSignature {
            artist: "Ghost B.C.".into(),
            title: "Jigolo Har Megiddo".into(),
            album: "Infestissumam".into(),
            duration_secs: 238,
        }
    }

    #[test]
    fn file_name_is_stable_and_case_insensitive() {
        let lower = TrackSignature {
            artist: "ghost b.c.".into(),
            ..signature()
        };
        assert_eq!(file_name(&signature()), file_name(&lower));
        // The hash must not change between versions — otherwise the cache is lost
        assert_eq!(file_name(&signature()), "33735ca278336967.json");
    }

    #[test]
    fn answers_survive_restart() {
        let dir = std::env::temp_dir().join(format!("lmp_lyrics_{}", fastrand::u64(..)));
        let remote = Remote::Found {
            synced: Some("[00:01.00]Line".into()),
            plain: Some("Line".into()),
            instrumental: false,
        };
        let finder = LyricsFinder::new(Some(dir.clone()), Arc::new(|| {}));
        finder.save(&signature(), &remote);

        let mut restarted = LyricsFinder::new(Some(dir.clone()), Arc::new(|| {}));
        assert_eq!(restarted.remote(&signature()), Some(remote));
        fs::remove_dir_all(dir).unwrap();
    }
}
