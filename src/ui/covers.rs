//! Album covers and artist photos — the only image source for the whole
//! UI: grids and the current track. Covers aren't stored in tracks (thousands of covers would take
//! too much memory), so for those shown on screen they are read from tags
//! on background threads, downscaled to the needed size and turned into textures. Only
//! recently shown ones are kept: memory doesn't grow while scrolling a large library. Photos from
//! the internet (Deezer) are loaded from a link straight into memory and never written to disk.
//!
//! During fast scrolling there are more requests than threads can handle: those on screen
//! right now are prepared first, then neighboring rows (`prefetch`), and those scrolled off screen are dropped.

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::{Duration, Instant},
};

use crate::core::library::metadata::{read_cover, read_folder_cover};

/// How many covers to keep: a thumbnail is ~400 KB, up to ~120 MB of video memory in total
const MAX_COVERS: usize = 300;
/// A request not wanted for this many frames is dropped: the image scrolled off screen.
/// While requests are waiting, frames come every 50 ms — that's ~0.5 s
const STALE_FRAMES: u64 = 10;
/// A ready image fades in over this many seconds rather than appearing abruptly
const APPEAR_SECS: f32 = 0.15;

/// Cover size: determines how many pixels to downscale it to
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum CoverSize {
    /// Grid tile
    Thumbnail,
    /// Current track cover
    Large,
    /// Blurred cover — the backdrop of the current track details. Small: when stretched over the whole
    /// area, linear filtering smooths it by itself
    Backdrop,
}

impl CoverSize {
    /// Side in pixels — with headroom for double-density screens
    fn pixels(self) -> u32 {
        match self {
            Self::Thumbnail => 320,
            Self::Large => 640,
            Self::Backdrop => 128,
        }
    }
}

/// Backdrop blur strength: Gaussian sigma in pixels of a `CoverSize::Backdrop` cover
const BACKDROP_BLUR_SIGMA: f32 = 8.0;

/// Where the image comes from: files (cached by the first candidate) or a link
#[derive(Clone, PartialEq, Eq, Hash)]
enum Origin {
    File(PathBuf),
    Url(String),
}

/// A cached image — by origin and size
type Key = (Origin, CoverSize);
/// Threads for covers from files. Link downloads run on their own thread: a slow network
/// doesn't hold up album covers
const WORKERS: usize = 2;

pub enum Cover<'a> {
    /// Still loading
    Loading,
    /// No cover in tags
    Missing,
    /// Ready. The second value is how far it has faded in: 0.0..=1.0, see `APPEAR_SECS`
    Ready(&'a TextureHandle, f32),
}

enum Entry {
    Loading,
    Missing {
        last_used: u64,
    },
    Ready {
        texture: TextureHandle,
        /// Hash of the source image: identical covers of different tracks share one texture by it
        content: u64,
        last_used: u64,
        /// When it became ready — for the fade-in
        ready_at: Instant,
    },
}

/// A read cover: the image and the hash of the source bytes
type Loaded = Option<(ColorImage, u64)>;

/// A request to the background thread
struct Job {
    key: Key,
    /// Files whose tags to search for the cover, in order (empty for a link)
    candidates: Vec<PathBuf>,
    /// The frame in which the image was last wanted
    wanted: u64,
    /// In that frame it was on screen rather than in a neighboring row
    visible: bool,
}

/// Request queue shared by threads. A thread takes the most needed one: wanted most recently,
/// and on a tie — visible before neighboring
#[derive(Default)]
struct Jobs {
    items: Vec<Job>,
    closed: bool,
}

type Queue = Arc<(Mutex<Jobs>, Condvar)>;

pub struct CoverCache {
    /// Requests for covers from files
    files: Queue,
    /// Requests for link downloads
    downloads: Queue,
    results: Receiver<(Key, Loaded)>,
    entries: HashMap<Key, Entry>,
    /// Frame number — for evicting long-unshown thumbnails and abandoned requests
    frame: u64,
}

impl CoverCache {
    pub fn new() -> Self {
        let (result_tx, results) = mpsc::channel();
        let files = Queue::default();
        let downloads = Queue::default();
        let workers = std::iter::repeat_n(&files, WORKERS).chain(std::iter::once(&downloads));
        for queue in workers {
            let (queue, result_tx) = (Arc::clone(queue), result_tx.clone());
            thread::spawn(move || work(&queue, &result_tx));
        }
        Self {
            files,
            downloads,
            results,
            entries: HashMap::new(),
            frame: 0,
        }
    }

    /// Cover of size `size` from the tags of the first of `candidates` that has one: an album
    /// cover is present only in some of its tracks. If it's not there yet — requests a background read.
    /// The cover is cached under the first candidate
    pub fn get(&mut self, candidates: &[PathBuf], size: CoverSize) -> Cover<'_> {
        let Some(first) = candidates.first() else {
            return Cover::Missing;
        };
        let key = (Origin::File(first.clone()), size);
        self.request(&key, candidates, true);
        self.cover(&key)
    }

    /// Image of size `size` from a link: loaded in the background into memory only — never written
    /// to disk. Failed to load (no network) — a placeholder until the image is evicted from the cache
    pub fn get_url(&mut self, url: &str, size: CoverSize) -> Cover<'_> {
        let key = (Origin::Url(url.to_owned()), size);
        self.request(&key, &[], true);
        self.cover(&key)
    }

    /// Requests a cover in advance — for grid rows next to the screen: by the time of scrolling it
    /// will be ready. Visible ones are prepared first
    pub fn prefetch(&mut self, candidates: &[PathBuf], size: CoverSize) {
        if let Some(first) = candidates.first() {
            self.request(&(Origin::File(first.clone()), size), candidates, false);
        }
    }

    /// Requests image `key` or, if already requested, marks that it's still needed
    fn request(&mut self, key: &Key, candidates: &[PathBuf], visible: bool) {
        let frame = self.frame;
        let queue = match key.0 {
            Origin::File(_) => &self.files,
            Origin::Url(_) => &self.downloads,
        };
        let (jobs, ready) = &**queue;
        let Ok(mut jobs) = jobs.lock() else {
            return;
        };
        match self.entries.get(key) {
            None => {
                self.entries.insert(key.clone(), Entry::Loading);
                jobs.items.push(Job {
                    key: key.clone(),
                    candidates: candidates.to_vec(),
                    wanted: frame,
                    visible,
                });
                ready.notify_one();
            }
            // Waiting in the queue — needed in this frame too. If a thread has already taken it, it's not
            // in the queue: it'll be ready soon anyway
            Some(Entry::Loading) => {
                if let Some(job) = jobs.items.iter_mut().find(|job| job.key == *key) {
                    job.visible = visible || (job.wanted == frame && job.visible);
                    job.wanted = frame;
                }
            }
            Some(_) => {}
        }
    }

    /// An image from the cache by key; marks that it was shown in this frame
    fn cover(&mut self, key: &Key) -> Cover<'_> {
        let frame = self.frame;
        match self.entries.get_mut(key) {
            Some(Entry::Ready {
                texture,
                last_used,
                ready_at,
                ..
            }) => {
                *last_used = frame;
                let appear = (ready_at.elapsed().as_secs_f32() / APPEAR_SECS).min(1.0);
                Cover::Ready(texture, appear)
            }
            Some(Entry::Missing { last_used }) => {
                *last_used = frame;
                Cover::Missing
            }
            _ => Cover::Loading,
        }
    }

    /// Takes ready thumbnails, drops abandoned requests and evicts excess
    /// thumbnails. Call once per frame
    pub fn update(&mut self, ctx: &egui::Context) {
        self.frame += 1;
        while let Ok((key, loaded)) = self.results.try_recv() {
            let entry = match loaded {
                Some((image, content)) => Entry::Ready {
                    // The same image already exists (tracks of one album usually share a cover) —
                    // take its texture: switching tracks within an album doesn't change the cover at all
                    texture: self.texture_with(content, key.1).unwrap_or_else(|| {
                        let name = match &key.0 {
                            Origin::File(path) => format!("cover {}", path.display()),
                            Origin::Url(url) => format!("cover {url}"),
                        };
                        ctx.load_texture(name, image, TextureOptions::LINEAR)
                    }),
                    content,
                    last_used: self.frame,
                    ready_at: Instant::now(),
                },
                None => Entry::Missing {
                    last_used: self.frame,
                },
            };
            self.entries.insert(key, entry);
        }
        self.drop_stale_jobs();

        // Waiting for thumbnails or fading in ready ones — frames are needed even without user actions
        if self.entries.values().any(|e| matches!(e, Entry::Loading)) {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        let appearing = self.entries.values().any(|e| {
            matches!(e, Entry::Ready { ready_at, .. } if ready_at.elapsed().as_secs_f32() < APPEAR_SECS)
        });
        if appearing {
            ctx.request_repaint();
        }

        self.evict();
    }

    /// Drops requests not wanted for a long time: the image scrolled off screen. When shown
    /// again — it will be requested again
    fn drop_stale_jobs(&mut self) {
        let frame = self.frame;
        for queue in [&self.files, &self.downloads] {
            let Ok(mut jobs) = queue.0.lock() else {
                continue;
            };
            jobs.items.retain(|job| {
                let wanted = job.wanted + STALE_FRAMES >= frame;
                if !wanted {
                    self.entries.remove(&job.key);
                }
                wanted
            });
        }
    }

    /// An already loaded cover texture with hash `content` and size `size`
    fn texture_with(&self, content: u64, size: CoverSize) -> Option<TextureHandle> {
        self.entries
            .iter()
            .find_map(|((_, entry_size), entry)| match entry {
                Entry::Ready {
                    texture,
                    content: other,
                    ..
                } if *other == content && *entry_size == size => Some(texture.clone()),
                _ => None,
            })
    }

    /// Keeps MAX_COVERS recently shown; doesn't touch requested but not ready ones
    fn evict(&mut self) {
        let loaded = self
            .entries
            .values()
            .filter(|e| !matches!(e, Entry::Loading))
            .count();
        if loaded <= MAX_COVERS {
            return;
        }
        let mut by_age: Vec<(u64, Key)> = self
            .entries
            .iter()
            .filter_map(|(key, entry)| match entry {
                Entry::Ready { last_used, .. } | Entry::Missing { last_used } => {
                    Some((*last_used, key.clone()))
                }
                Entry::Loading => None,
            })
            .collect();
        by_age.sort_unstable_by_key(|(last_used, _)| *last_used);
        for (_, key) in by_age.into_iter().take(loaded - MAX_COVERS) {
            self.entries.remove(&key);
        }
    }
}

/// Background thread: takes the most needed request and prepares the image
fn work(queue: &Queue, results: &Sender<(Key, Loaded)>) {
    let (jobs, ready) = &**queue;
    loop {
        let job = {
            let Ok(mut jobs) = jobs.lock() else {
                return;
            };
            loop {
                if jobs.closed {
                    return;
                }
                let best = jobs
                    .items
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, job)| (job.wanted, job.visible))
                    .map(|(index, _)| index);
                if let Some(index) = best {
                    break jobs.items.swap_remove(index);
                }
                let Ok(next) = ready.wait(jobs) else {
                    return;
                };
                jobs = next;
            }
        };
        let image = match &job.key.0 {
            Origin::File(_) => find_cover(&job.candidates, job.key.1),
            Origin::Url(url) => download(url).and_then(|bytes| decode_cover(&bytes, job.key.1)),
        };
        if results.send((job.key, image)).is_err() {
            return;
        }
    }
}

impl Drop for CoverCache {
    fn drop(&mut self) {
        // The threads finish the current image and exit
        for queue in [&self.files, &self.downloads] {
            if let Ok(mut jobs) = queue.0.lock() {
                jobs.closed = true;
                queue.1.notify_all();
            }
        }
    }
}

/// Cover for files `candidates`: from the tags of the first that has one (usually the very first),
/// and if none has it — an image from their folder (see `read_folder_cover`).
/// Together with the image — the hash of the source bytes
fn find_cover(candidates: &[PathBuf], size: CoverSize) -> Loaded {
    cover_sources(candidates).find_map(|bytes| decode_cover(&bytes, size))
}

/// Cover bytes for files `candidates` as is, without decoding — in the same search
/// order as `find_cover`
pub fn cover_bytes(candidates: &[PathBuf]) -> Option<Vec<u8>> {
    cover_sources(candidates).next()
}

/// Found images in order: from the files' tags, then from their folders. Read lazily —
/// up to the first suitable one
fn cover_sources(candidates: &[PathBuf]) -> impl Iterator<Item = Vec<u8>> + '_ {
    // An image candidate (artist photo) is read as is, the rest are audio files
    let tags = candidates.iter().filter_map(|path| {
        if is_image_file(path) {
            std::fs::read(path).ok()
        } else {
            read_cover(path)
        }
    });
    let folders = std::iter::once(()).flat_map(move |()| {
        // Folders in candidate order, each once: an album's tracks are usually in one.
        // Image folders are not checked: next to an artist photo lie photos of others
        let mut dirs: Vec<&Path> = Vec::new();
        for dir in candidates
            .iter()
            .filter(|path| !is_image_file(path))
            .filter_map(|path| path.parent())
        {
            if !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
        dirs.into_iter().filter_map(read_folder_cover)
    });
    tags.chain(folders)
}

/// An image from a link — as bytes into memory
fn download(url: &str) -> Option<Vec<u8>> {
    let mut response = crate::core::net::agent().get(url).call().ok()?;
    if response.status() != 200 {
        return None;
    }
    response.body_mut().read_to_vec().ok()
}

/// An image file rather than an audio file — by extension
fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            ["jpg", "jpeg", "png", "webp"]
                .iter()
                .any(|known| ext.eq_ignore_ascii_case(known))
        })
}

/// Cover image downscaled so the larger side is no more than
/// `size.pixels()`; for the backdrop — also blurred
fn decode_cover(bytes: &[u8], size: CoverSize) -> Loaded {
    let pixels = size.pixels();
    let mut image = image::load_from_memory(bytes)
        .ok()?
        .thumbnail(pixels, pixels)
        .to_rgba8();
    if size == CoverSize::Backdrop {
        image = image::imageops::blur(&image, BACKDROP_BLUR_SIGMA);
    }
    let size = [image.width() as usize, image.height() as usize];
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    Some((
        ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
        hasher.finish(),
    ))
}
