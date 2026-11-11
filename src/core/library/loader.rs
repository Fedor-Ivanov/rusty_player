//! Background track adding. Reading tags and checking the format of thousands of files takes
//! seconds to minutes — on the UI thread the window would freeze for that time. Here files
//! are processed on several threads; tracks are accumulated and handed over at once when everything is done —
//! so the list has the whole selection at once, and a track from the end of the folder can be found.
//!
//! To cancel adding, just drop the `Loader`: the threads notice the closed channel and stop.

use std::{
    collections::{BTreeMap, HashSet},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc::{self, Receiver, Sender, TryRecvError},
    },
    thread,
    time::{Duration, Instant},
};

use super::{scan::find_audio_files, track::Track};

/// More threads give no gain: disk reads are the bottleneck
const MAX_WORKERS: usize = 8;

/// What is being added
pub enum Source {
    Files(Vec<PathBuf>),
    /// All audio files in a folder, including subfolders
    Folder(PathBuf),
}

enum Message {
    /// How many files will be processed and how many skipped because the receiver already
    /// has them — known after the folder walk
    Total(usize, usize),
    /// Result for the file with index `usize` in the original order
    Loaded(usize, Result<Track, String>),
    /// The folder could not be read
    Failed(String),
}

pub struct Loader {
    messages: Receiver<Message>,
    started: Instant,
    total: Option<usize>,
    skipped: usize,
    /// Index of the next result in order — tracks are collected strictly in the original order
    next: usize,
    /// Results that arrived ahead of their turn: threads run in parallel
    pending: BTreeMap<usize, Result<Track, String>>,
    tracks: Vec<Track>,
    errors: Vec<String>,
    failed: bool,
}

impl Loader {
    /// Starts adding. Files from `skip` are skipped without reading — e.g. those
    /// already in the library
    pub fn start(source: Source, skip: HashSet<PathBuf>) -> Self {
        let (tx, messages) = mpsc::channel();
        thread::spawn(move || run(source, skip, tx));
        Self {
            messages,
            started: Instant::now(),
            total: None,
            skipped: 0,
            next: 0,
            pending: BTreeMap::new(),
            tracks: Vec::new(),
            errors: Vec::new(),
            failed: false,
        }
    }

    /// Takes the results ready by now. Non-blocking. Call every frame
    pub fn update(&mut self) {
        loop {
            match self.messages.try_recv() {
                Ok(Message::Total(total, skipped)) => {
                    self.total = Some(total);
                    self.skipped = skipped;
                }
                Ok(Message::Loaded(index, result)) => {
                    self.pending.insert(index, result);
                }
                Ok(Message::Failed(error)) => {
                    self.errors.push(error);
                    self.failed = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    // All threads have finished; if one crashed, the missing tracks
                    // will never come — don't wait for them forever
                    if self
                        .total
                        .is_none_or(|total| self.next + self.pending.len() < total)
                    {
                        self.failed = true;
                    }
                    break;
                }
            }
        }

        while let Some(result) = self.pending.remove(&self.next) {
            self.next += 1;
            match result {
                Ok(track) => self.tracks.push(track),
                Err(error) => self.errors.push(error),
            }
        }
    }

    pub fn is_finished(&self) -> bool {
        self.failed || self.total.is_some_and(|total| self.next >= total)
    }

    /// How many files are processed and how many in total; `None` while the folder walk is running
    pub fn progress(&self) -> Option<(usize, usize)> {
        self.total
            .map(|total| (self.next + self.pending.len(), total))
    }

    /// Time elapsed since adding started
    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    /// Result: tracks in the original order, per-file errors, and how many files were found
    pub fn finish(self) -> Finished {
        Finished {
            tracks: self.tracks,
            errors: self.errors,
            total: self.total,
            skipped: self.skipped,
        }
    }
}

pub struct Finished {
    pub tracks: Vec<Track>,
    /// Errors for files that could not be added
    pub errors: Vec<String>,
    /// `None` if the folder could not be read
    pub total: Option<usize>,
    /// How many files were skipped because the receiver already has them
    pub skipped: usize,
}

/// Background part of `Loader::start`: walks the folder, then reads the files on several
/// threads. Results arrive in any order with their index — `update` puts them back in order
fn run(source: Source, skip: HashSet<PathBuf>, tx: Sender<Message>) {
    let mut paths = match source {
        Source::Files(paths) => paths,
        Source::Folder(dir) => match find_audio_files(&dir) {
            Ok(paths) => paths,
            Err(error) => {
                let _ = tx.send(Message::Failed(error));
                return;
            }
        },
    };
    let found = paths.len();
    paths.retain(|path| !skip.contains(path));
    let _ = tx.send(Message::Total(paths.len(), found - paths.len()));

    // Threads take files in turn from a shared counter: fast files don't wait for slow ones
    let paths = Arc::new(paths);
    let next = Arc::new(AtomicUsize::new(0));
    let workers = thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(MAX_WORKERS);
    for _ in 0..workers {
        let (paths, next, tx) = (paths.clone(), next.clone(), tx.clone());
        thread::spawn(move || {
            loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                let Some(path) = paths.get(index) else {
                    break;
                };
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let result = Track::from_path(path.clone()).map_err(|e| format!("{name}: {e}"));
                // The receiver is gone — the UI is closed, no point in continuing
                if tx.send(Message::Loaded(index, result)).is_err() {
                    break;
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{Loader, Source};
    use std::{fs, path::Path, thread, time::Duration};

    /// Minimal valid WAV: 16-bit mono, 8 kHz, 0.1 s of silence
    fn write_wav(path: &Path) {
        let samples = 800u32;
        let data_len = samples * 2;
        let mut bytes = Vec::new();
        bytes.extend(b"RIFF");
        bytes.extend((36 + data_len).to_le_bytes());
        bytes.extend(b"WAVEfmt ");
        bytes.extend(16u32.to_le_bytes());
        bytes.extend(1u16.to_le_bytes()); // PCM
        bytes.extend(1u16.to_le_bytes()); // mono
        bytes.extend(8000u32.to_le_bytes());
        bytes.extend(16000u32.to_le_bytes()); // bytes per second
        bytes.extend(2u16.to_le_bytes()); // bytes per sample
        bytes.extend(16u16.to_le_bytes()); // bits per sample
        bytes.extend(b"data");
        bytes.extend(data_len.to_le_bytes());
        bytes.resize(bytes.len() + data_len as usize, 0);
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn loads_folder_in_order_and_reports_errors() {
        let dir = std::env::temp_dir().join(format!("lmp_loader_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let names: Vec<String> = (0..40).map(|i| format!("{i:02}.wav")).collect();
        for name in &names {
            write_wav(&dir.join(name));
        }
        // A broken file with a matching extension — must end up in errors, not in the list
        fs::write(dir.join("99.mp3"), b"not audio").unwrap();

        let skip = [dir.join("05.wav")].into_iter().collect();
        let mut loader = Loader::start(Source::Folder(dir.clone()), skip);
        for _ in 0..500 {
            loader.update();
            if loader.is_finished() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        fs::remove_dir_all(&dir).unwrap();

        assert!(loader.is_finished());
        // 05.wav was in `skip` — it was not read
        assert_eq!(loader.progress(), Some((40, 40)));
        let result = loader.finish();
        let loaded: Vec<String> = result
            .tracks
            .iter()
            .map(|t| t.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        let expected: Vec<&String> = names.iter().filter(|n| *n != "05.wav").collect();
        assert_eq!(loaded.iter().collect::<Vec<_>>(), expected);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].starts_with("99.mp3"));
        assert_eq!(result.total, Some(40));
        assert_eq!(result.skipped, 1);
    }
}
