//! Writing data files: the library, queue, playlists and settings are stored as JSON files in the app
//! data folder. Each module decides what and when to save, but the writing happens here.

use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread::JoinHandle,
};

/// Writes `value` to JSON file `file`: into a temp file, then renames it over the old one.
/// If writing is interrupted, the old file stays intact rather than half-written
pub fn write_atomically(file: &Path, value: &impl Serialize) -> Result<(), String> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string(value).map_err(|e| e.to_string())?;
    let temp = file.with_extension("json.tmp");
    fs::write(&temp, json).map_err(|e| e.to_string())?;
    fs::rename(&temp, file).map_err(|e| e.to_string())
}

/// String hash for cache file names (lyrics, artist images) — FNV-1a.
/// Not `DefaultHasher`: it may compute differently in another Rust version, and the cache would be lost
pub fn stable_hash(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ byte as u64).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// Now — in Unix seconds: when the service was asked, for the cached answer's lifetime
pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// Writing a file on a background thread — for large files (the library), so the UI
/// doesn't wait for the write. Writes in order; if changes come faster than writing keeps up,
/// intermediate states are skipped — the latest is written. On drop, waits for
/// everything sent to be written
pub struct BackgroundWriter<T> {
    jobs: Option<Sender<Job<T>>>,
    errors: Receiver<String>,
    thread: Option<JoinHandle<()>>,
}

enum Job<T> {
    Write(T),
    /// Reply when everything sent earlier has been written
    Flush(Sender<()>),
}

impl<T: Serialize + Send + 'static> BackgroundWriter<T> {
    /// `what` — what is being written, for the error text: "Could not save the {what}"
    pub fn new(file: PathBuf, what: &'static str) -> Self {
        let (jobs, inbox) = mpsc::channel::<Job<T>>();
        let (report, errors) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            let write = |value: &T| {
                if let Err(e) = write_atomically(&file, value) {
                    let _ = report.send(format!("Could not save the {what}: {e}"));
                }
            };
            // Wait for a task, then take everything that has accumulated: only the last write
            // is needed, and a wait request is fulfilled after writing what came before it
            while let Ok(job) = inbox.recv() {
                let mut latest = None;
                let mut flushes = Vec::new();
                for job in std::iter::once(job).chain(inbox.try_iter()) {
                    match job {
                        Job::Write(value) => latest = Some(value),
                        Job::Flush(done) => {
                            if let Some(value) = latest.take() {
                                write(&value);
                            }
                            flushes.push(done);
                        }
                    }
                }
                if let Some(value) = latest {
                    write(&value);
                }
                for done in flushes {
                    let _ = done.send(());
                }
            }
        });
        Self {
            jobs: Some(jobs),
            errors,
            thread: Some(thread),
        }
    }

    /// Sends `value` to be written and returns immediately
    pub fn write(&self, value: T) {
        if let Some(jobs) = &self.jobs {
            let _ = jobs.send(Job::Write(value));
        }
    }

    /// Waits until everything sent is written: before exit and in tests
    pub fn flush(&self) {
        let (done, wait) = mpsc::channel();
        if let Some(jobs) = &self.jobs
            && jobs.send(Job::Flush(done)).is_ok()
        {
            let _ = wait.recv();
        }
    }

    /// Write error, if any — to show the user
    pub fn take_error(&self) -> Option<String> {
        self.errors.try_iter().last()
    }
}

impl<T> Drop for BackgroundWriter<T> {
    fn drop(&mut self) {
        // Without a sender the thread finishes writing the rest and exits
        self.jobs = None;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BackgroundWriter;
    use std::fs;

    #[test]
    fn writes_latest_value_in_background() {
        let dir = std::env::temp_dir().join(format!("lmp_writer_{}", fastrand::u64(..)));
        let file = dir.join("data.json");
        let writer = BackgroundWriter::new(file.clone(), "data");
        for value in 0..100 {
            writer.write(value);
        }
        writer.flush();
        assert_eq!(fs::read_to_string(&file).unwrap(), "99");
        assert!(writer.take_error().is_none());

        // On drop, what was sent gets written
        writer.write(7);
        drop(writer);
        assert_eq!(fs::read_to_string(&file).unwrap(), "7");
        fs::remove_dir_all(dir).unwrap();
    }
}
