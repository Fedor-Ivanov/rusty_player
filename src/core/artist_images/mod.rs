//! Artist images. A local one comes first — an `artist.jpg` (`.png`, `.webp`) file next to
//! the artist's music, the way Kodi and Plex place them. No local one — a photo from Deezer.
//!
//! Deezer photos are not saved to disk: their license requires that. The data folder
//! (`artists/`) stores only the link (and the "not found" answer) — so Deezer isn't asked
//! on every launch. The image itself is loaded from the link when it's needed on screen,
//! and lives only in memory (see `ui::covers`).
//!
//! All work happens on a background thread, the UI doesn't wait for it: after the library loads,
//! all artists are queued, and images appear over time. Artists on screen
//! are processed before the rest. Deezer is queried one request at a time, with a pause, and only
//! if online search is enabled in settings (`set_online`). Without it, Deezer photos
//! are not shown at all — loading from a link is also an internet access.

mod deezer;

use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    time::{Duration, Instant},
};

use super::storage::{now_secs, stable_hash, write_atomically};

const DIR_NAME: &str = "artists";
/// 2 — a link to the image is stored instead of the image. Version 1 answers are no good: version 1
/// images are deleted (`remove_stored_pictures`), and the artist is queried again
const FORMAT_VERSION: u32 = 2;
/// A local artist image name — with one of these extensions
const LOCAL_NAME: &str = "artist";
const LOCAL_EXTENSIONS: [&str; 4] = ["jpg", "jpeg", "png", "webp"];
/// "Not found" is remembered for this long: then ask again — a photo may have been added
const NOT_FOUND_TTL: Duration = Duration::from_secs(30 * 24 * 60 * 60);
/// Request failed — ask again no sooner than after this long
const FAILED_RETRY_AFTER: Duration = Duration::from_secs(5 * 60);
/// Pause after a Deezer request: its limit is 50 requests per 5 seconds
const REQUEST_GAP: Duration = Duration::from_millis(200);

/// What to show instead of the placeholder
#[derive(Debug, Clone, PartialEq)]
pub enum ArtistImage {
    /// Local image — a file
    File(PathBuf),
    /// Deezer photo — load from the link, into memory only
    Url(String),
    /// Being looked up in the background
    Searching,
    /// No image — placeholder
    Missing,
}

/// An artist for the background lookup
pub struct ArtistJob {
    pub name: String,
    /// Folders of the artist's tracks: the local image is searched in them
    pub music_dirs: Vec<PathBuf>,
}

/// A found artist image
#[derive(Debug, Clone, PartialEq)]
enum Picture {
    File(PathBuf),
    Url(String),
}

/// Cached Deezer answer — a link to the photo, not the photo itself
#[derive(Serialize, Deserialize)]
struct Cached {
    version: u32,
    /// When it was asked — Unix seconds
    checked_at: u64,
    /// `None` — not found
    url: Option<String>,
}

/// Artist state in this run
enum State {
    /// In the background thread's queue; `urgent` — already moved to the front
    Queued {
        urgent: bool,
    },
    Done(Option<Picture>),
    /// There is neither a local image nor a known link, and Deezer can't be asked — online search is off.
    /// Once enabled — we'll ask (`set_online`)
    Offline,
    Failed(Instant),
}

/// Background thread's answer about an artist
enum Outcome {
    Done(Option<Picture>),
    Offline,
    Failed(String),
}

/// Background thread queue: urgent ones (on screen) first, then the rest
#[derive(Default)]
struct Queue {
    urgent: VecDeque<ArtistJob>,
    background: VecDeque<ArtistJob>,
    closed: bool,
}

type Answer = (String, Outcome);

pub struct ArtistImages {
    /// Keyed by lowercase name
    states: HashMap<String, State>,
    queue: Arc<(Mutex<Queue>, Condvar)>,
    answers: Receiver<Answer>,
    /// Whether Deezer may be asked — shared with the background thread
    online: Arc<AtomicBool>,
}

impl ArtistImages {
    /// `data_dir` — the data folder (`None` — unavailable: local images only). `wake` wakes
    /// the app when an image is ready
    pub fn new(data_dir: Option<PathBuf>, wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        let cache = data_dir.map(|dir| dir.join(DIR_NAME));
        let queue: Arc<(Mutex<Queue>, Condvar)> = Arc::default();
        let (reply, answers) = mpsc::channel();
        let online = Arc::new(AtomicBool::new(false));
        let (worker_queue, worker_online) = (Arc::clone(&queue), Arc::clone(&online));
        std::thread::spawn(move || {
            work(
                &worker_queue,
                cache.as_deref(),
                &worker_online,
                &reply,
                &*wake,
            );
        });
        Self {
            states: HashMap::new(),
            queue,
            answers,
            online,
        }
    }

    /// Allows or forbids asking Deezer. After enabling, artists that
    /// weren't looked up because search was off are considered unknown again: they can be
    /// queued (`prefetch`, `image`)
    pub fn set_online(&mut self, online: bool) {
        self.online.store(online, Ordering::Relaxed);
        if online {
            self.states
                .retain(|_, state| !matches!(state, State::Offline));
        }
    }

    /// Queues in the background all artists nothing is known about yet —
    /// after the library is loaded or changed
    pub fn prefetch(&mut self, artists: impl IntoIterator<Item = ArtistJob>) {
        let jobs: Vec<ArtistJob> = artists
            .into_iter()
            .filter(|job| self.should_queue(&key(&job.name)))
            .collect();
        if jobs.is_empty() {
            return;
        }
        for job in &jobs {
            self.states
                .insert(key(&job.name), State::Queued { urgent: false });
        }
        self.push(jobs, false);
    }

    /// Image of an artist on screen. Not ready yet — the artist goes to the front of the queue.
    /// `music_dirs` is called only if the artist needs to be queued
    pub fn image(&mut self, name: &str, music_dirs: impl FnOnce() -> Vec<PathBuf>) -> ArtistImage {
        let key = key(name);
        if key.is_empty() {
            return ArtistImage::Missing;
        }
        match self.states.get(&key) {
            Some(State::Done(picture)) => {
                return match picture {
                    Some(Picture::File(path)) => ArtistImage::File(path.clone()),
                    // Loading from a link is an internet access: only with permission
                    Some(Picture::Url(url)) if self.online.load(Ordering::Relaxed) => {
                        ArtistImage::Url(url.clone())
                    }
                    _ => ArtistImage::Missing,
                };
            }
            Some(State::Queued { urgent: true }) => return ArtistImage::Searching,
            Some(State::Offline) => return ArtistImage::Missing,
            Some(State::Failed(_)) if !self.should_queue(&key) => return ArtistImage::Missing,
            _ => {}
        }
        // A new one, or one waiting in the general queue — to the front
        self.states.insert(key, State::Queued { urgent: true });
        self.push(
            vec![ArtistJob {
                name: name.to_owned(),
                music_dirs: music_dirs(),
            }],
            true,
        );
        ArtistImage::Searching
    }

    /// Takes the ready images. `true` — something arrived: worth redrawing
    pub fn poll(&mut self) -> bool {
        let mut arrived = false;
        while let Ok((key, outcome)) = self.answers.try_recv() {
            arrived = true;
            let state = match outcome {
                Outcome::Done(image) => State::Done(image),
                Outcome::Offline => State::Offline,
                Outcome::Failed(e) => {
                    eprintln!("Could not get the picture of {key}: {e}");
                    State::Failed(Instant::now())
                }
            };
            self.states.insert(key, state);
        }
        arrived
    }

    /// Unknown, or failed long ago — can be queued
    fn should_queue(&self, key: &str) -> bool {
        match self.states.get(key) {
            None => true,
            Some(State::Failed(at)) => at.elapsed() >= FAILED_RETRY_AFTER,
            Some(_) => false,
        }
    }

    fn push(&self, jobs: Vec<ArtistJob>, urgent: bool) {
        let (queue, ready) = &*self.queue;
        let Ok(mut queue) = queue.lock() else {
            return;
        };
        if urgent {
            queue.urgent.extend(jobs);
        } else {
            queue.background.extend(jobs);
        }
        ready.notify_one();
    }
}

impl Drop for ArtistImages {
    fn drop(&mut self) {
        // The background thread finishes the current artist and exits
        let (queue, ready) = &*self.queue;
        if let Ok(mut queue) = queue.lock() {
            queue.closed = true;
            ready.notify_one();
        }
    }
}

fn key(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Background thread: takes artists from the queue one by one — urgent ones first
fn work(
    queue: &(Mutex<Queue>, Condvar),
    cache: Option<&Path>,
    online: &AtomicBool,
    reply: &Sender<Answer>,
    wake: &(dyn Fn() + Send + Sync),
) {
    if let Some(cache) = cache {
        remove_stored_pictures(cache);
    }
    let (queue, ready) = queue;
    // Already processed: an artist could be queued twice — in the background and urgently
    let mut done: HashSet<String> = HashSet::new();
    loop {
        let job = {
            let Ok(mut queue) = queue.lock() else {
                return;
            };
            loop {
                if queue.closed {
                    return;
                }
                if let Some(job) = queue
                    .urgent
                    .pop_front()
                    .or_else(|| queue.background.pop_front())
                {
                    break job;
                }
                let Ok(next) = ready.wait(queue) else {
                    return;
                };
                queue = next;
            }
        };
        let key = key(&job.name);
        if done.contains(&key) {
            continue;
        }
        let online = online.load(Ordering::Relaxed);
        let (outcome, asked_deezer) = find_image(&key, &job.music_dirs, cache, online);
        // Request failed or wasn't made — can try again next time
        if matches!(outcome, Outcome::Done(_)) {
            done.insert(key.clone());
        }
        if reply.send((key, outcome)).is_err() {
            return;
        }
        wake();
        if asked_deezer {
            std::thread::sleep(REQUEST_GAP);
        }
    }
}

/// Artist image: local, cached, or — if `online` — from Deezer. The second value is
/// whether Deezer was asked: a pause is needed after a request
fn find_image(
    key: &str,
    music_dirs: &[PathBuf],
    cache: Option<&Path>,
    online: bool,
) -> (Outcome, bool) {
    if let Some(local) = find_local(music_dirs) {
        return (Outcome::Done(Some(Picture::File(local))), false);
    }
    let Some(cache) = cache else {
        return (Outcome::Done(None), false);
    };
    if let Some(cached) = read_cached(cache, key) {
        return (Outcome::Done(cached.map(Picture::Url)), false);
    }
    if !online {
        return (Outcome::Offline, false);
    }
    let outcome = match fetch_and_store(cache, key) {
        Ok(url) => Outcome::Done(url.map(Picture::Url)),
        Err(e) => Outcome::Failed(e),
    };
    (outcome, true)
}

/// Local artist image in one of the `dirs` folders
fn find_local(dirs: &[PathBuf]) -> Option<PathBuf> {
    dirs.iter()
        .flat_map(|dir| {
            LOCAL_EXTENSIONS
                .iter()
                .map(move |ext| dir.join(format!("{LOCAL_NAME}.{ext}")))
        })
        .find(|path| path.is_file())
}

/// Artist cache file: the Deezer answer
fn cache_file(dir: &Path, key: &str) -> PathBuf {
    dir.join(format!("{:016x}.json", stable_hash(key)))
}

/// Cached answer: `Some(Some(link))`, `Some(None)` — "not found", until stale;
/// `None` — nothing in cache (or stale, or from an old version)
fn read_cached(dir: &Path, key: &str) -> Option<Option<String>> {
    let text = fs::read_to_string(cache_file(dir, key)).ok()?;
    let cached: Cached = serde_json::from_str(&text).ok()?;
    if cached.version != FORMAT_VERSION {
        return None;
    }
    if cached.url.is_some() {
        return Some(cached.url);
    }
    let age = now_secs().saturating_sub(cached.checked_at);
    (age < NOT_FOUND_TTL.as_secs()).then_some(None)
}

/// Asks Deezer and caches the answer — a link to the photo, not the photo itself
fn fetch_and_store(dir: &Path, key: &str) -> Result<Option<String>, String> {
    let url = deezer::find_picture(key)?;
    let cached = Cached {
        version: FORMAT_VERSION,
        checked_at: now_secs(),
        url: url.clone(),
    };
    write_atomically(&cache_file(dir, key), &cached)?;
    Ok(url)
}

/// Deletes photos saved by version 1: storing Deezer photos on disk is not allowed
fn remove_stored_pictures(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.extension().is_some_and(|ext| ext == "jpg") {
            let _ = fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ArtistImage, ArtistImages, ArtistJob, Cached, FORMAT_VERSION, cache_file};
    use crate::core::storage::{now_secs, write_atomically};
    use std::{fs, sync::Arc, time::Duration};

    /// Waits until the background thread answers about `name`
    fn wait(images: &mut ArtistImages, name: &str) -> ArtistImage {
        for _ in 0..2000 {
            images.poll();
            let image = images.image(name, Vec::new);
            if image != ArtistImage::Searching {
                return image;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        panic!("no answer for {name}");
    }

    #[test]
    fn finds_local_pictures_and_cached_links_in_background() {
        let root = std::env::temp_dir().join(format!("lmp_artists_{}", fastrand::u64(..)));
        let music = root.join("music");
        fs::create_dir_all(&music).unwrap();
        fs::write(music.join("artist.png"), b"png").unwrap();

        // The cache holds a photo link for one artist, "not found" for another, and a photo
        // saved by the old version: it must not be there
        let data = root.join("data");
        let cache = data.join("artists");
        fs::create_dir_all(&cache).unwrap();
        let found = Cached {
            version: FORMAT_VERSION,
            checked_at: now_secs(),
            url: Some("https://cdn/him.jpg".into()),
        };
        write_atomically(&cache_file(&cache, "him"), &found).unwrap();
        let missing = Cached { url: None, ..found };
        write_atomically(&cache_file(&cache, "nobody"), &missing).unwrap();
        let old_picture = cache.join("0123456789abcdef.jpg");
        fs::write(&old_picture, b"jpg").unwrap();

        let mut images = ArtistImages::new(Some(data), Arc::new(|| {}));
        images.prefetch([ArtistJob {
            name: "Artist".into(),
            music_dirs: vec![music.clone()],
        }]);
        assert_eq!(
            wait(&mut images, "Artist"),
            ArtistImage::File(music.join("artist.png"))
        );
        // Online search is off (the default): Deezer photos are not shown
        // and Deezer is not asked
        assert_eq!(wait(&mut images, "HIM"), ArtistImage::Missing);
        assert_eq!(wait(&mut images, "Unknown Artist"), ArtistImage::Missing);
        assert!(!old_picture.exists());

        // Enabled — a known link is shown right away, without a request
        images.set_online(true);
        assert_eq!(
            images.image("HIM", Vec::new),
            ArtistImage::Url("https://cdn/him.jpg".into())
        );
        // "Not found" from the cache — also without a request
        assert_eq!(wait(&mut images, "Nobody"), ArtistImage::Missing);
        drop(images);
        fs::remove_dir_all(root).unwrap();
    }
}
