//! System media keys: ⏯ ⏮ ⏭ on the Mac keyboard, headphone buttons, "Control Center"
//! on macOS and the media overlay on Windows. Through the same channel the system shows what's
//! playing: title, artist, cover, position.
//!
//! macOS routes media keys to the app that last told it what's playing
//! (MPNowPlayingInfoCenter). So the current track is always reported, not just for
//! looks: without it the keys go to Music.app.

use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
    SeekDirection,
};
use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

use super::{action::Action, app::PlayerApp, covers::cover_bytes};
use crate::core::playback::PlayerState;

/// How far the position may drift from what the system knows before it's corrected.
/// This catches seeks — with the slider, the "previous" button to the track start, track repeat
const POSITION_DRIFT: Duration = Duration::from_millis(1500);
/// Seek step for the ⏪ ⏩ buttons (they exist on Windows)
const SEEK_STEP: Duration = Duration::from_secs(10);
/// Cover file in the temp folder: the system accepts a cover only as a link
const COVER_FILE: &str = "lmp-now-playing-cover";

pub(crate) struct SystemMedia {
    /// `None` — the system didn't let us connect: the app works without media keys too
    controls: Option<MediaControls>,
    /// Key presses: come from the system, handled in `logic`
    events: Receiver<MediaControlEvent>,
    /// What has already been reported to the system; `None` — nothing yet
    shown: Option<Shown>,
}

struct Shown {
    track: Option<PathBuf>,
    state: PlayerState,
    /// Position at the moment of reporting — `at`. After that the system advances it itself while playing
    position: Duration,
    at: Instant,
}

impl Shown {
    /// Where the system thinks the position is now
    fn expected_position(&self, now: Instant) -> Duration {
        if self.state == PlayerState::Playing {
            self.position + now.duration_since(self.at)
        } else {
            self.position
        }
    }
}

impl SystemMedia {
    /// Connects to system media keys. A press wakes the app (`request_repaint`):
    /// the keys work even when the window is hidden
    pub(crate) fn new(cc: &eframe::CreationContext) -> Self {
        let (sender, events) = mpsc::channel();
        let ctx = cc.egui_ctx.clone();
        let controls = MediaControls::new(PlatformConfig {
            display_name: crate::APP_NAME,
            dbus_name: "lmp",
            hwnd: window_handle(cc),
        })
        .and_then(|mut controls| {
            controls.attach(move |event| {
                let _ = sender.send(event);
                ctx.request_repaint();
            })?;
            Ok(controls)
        });
        let controls = match controls {
            Ok(controls) => Some(controls),
            Err(e) => {
                eprintln!("Could not connect to the system media controls: {e}");
                None
            }
        };
        Self {
            controls,
            events,
            shown: None,
        }
    }
}

/// Window for the Windows media overlay: it's bound to a window. Not needed on macOS
fn window_handle(cc: &eframe::CreationContext) -> Option<*mut std::ffi::c_void> {
    #[cfg(target_os = "windows")]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        match cc.window_handle().ok()?.as_raw() {
            RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as *mut std::ffi::c_void),
            _ => None,
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = cc;
        None
    }
}

impl PlayerApp {
    /// System media key presses — into actions. Called in `logic`
    pub(crate) fn poll_system_media(&mut self) {
        while let Ok(event) = self.system_media.events.try_recv() {
            if let Some(action) = self.media_action(event) {
                self.dispatch(action);
            }
        }
    }

    fn media_action(&self, event: MediaControlEvent) -> Option<Action> {
        let seek_by = |direction, step: Duration| {
            let position = Duration::from_millis(self.core.controller.position_ms());
            let target = match direction {
                SeekDirection::Forward => position + step,
                SeekDirection::Backward => position.saturating_sub(step),
            };
            Action::Seek(target.as_secs_f32())
        };
        Some(match event {
            MediaControlEvent::Toggle => Action::TogglePlay,
            MediaControlEvent::Play => Action::Play,
            // Stop is treated as pause: the track stays selected and resumes from the same place
            MediaControlEvent::Pause | MediaControlEvent::Stop => Action::Pause,
            MediaControlEvent::Next => Action::Next,
            MediaControlEvent::Previous => Action::Previous,
            MediaControlEvent::SetPosition(MediaPosition(position)) => {
                Action::Seek(position.as_secs_f32())
            }
            MediaControlEvent::Seek(direction) => seek_by(direction, SEEK_STEP),
            MediaControlEvent::SeekBy(direction, step) => seek_by(direction, step),
            MediaControlEvent::SetVolume(level) => Action::SetVolume(level as f32),
            MediaControlEvent::OpenUri(_) | MediaControlEvent::Raise | MediaControlEvent::Quit => {
                return None;
            }
        })
    }

    /// Tells the system what's playing if it changed: another track — title and cover,
    /// pause, play or seek — state and position. Called in `logic` after actions
    pub(crate) fn sync_system_media(&mut self) {
        if self.system_media.controls.is_none() {
            return;
        }
        let player = self.core.controller.player();
        let track = player.current_track.clone();
        let state = player.state;
        let position = Duration::from_millis(self.core.controller.position_ms());
        let now = Instant::now();
        let path = track.as_ref().map(|track| track.path.clone());

        let (track_changed, playback_changed) = match &self.system_media.shown {
            None => (true, true),
            Some(shown) => {
                let expected = shown.expected_position(now);
                (
                    shown.track != path,
                    shown.state != state || expected.abs_diff(position) > POSITION_DRIFT,
                )
            }
        };
        if !track_changed && !playback_changed {
            return;
        }

        // The cover is looked up while `self` is free: the candidates need the library groups
        let cover = match &track {
            Some(track) if track_changed => {
                let candidates = self.cover_candidates(track);
                cover_bytes(&candidates).and_then(|bytes| write_cover_file(&bytes))
            }
            _ => None,
        };
        let Some(controls) = &mut self.system_media.controls else {
            return;
        };
        // Errors aren't shown: without the system panel the player works as usual
        if track_changed {
            let metadata = match &track {
                Some(track) => MediaMetadata {
                    title: Some(&track.title),
                    artist: Some(&track.artist),
                    album: Some(&track.album),
                    duration: Some(Duration::from_secs(track.duration_secs)),
                    cover_url: cover.as_deref(),
                },
                None => MediaMetadata::default(),
            };
            if let Err(e) = controls.set_metadata(metadata) {
                eprintln!("Could not update the system media controls: {e}");
            }
        }
        // New track info resets the position — so it's sent after it too
        let progress = Some(MediaPosition(position));
        let playback = match (&track, state) {
            (None, _) => MediaPlayback::Stopped,
            (Some(_), PlayerState::Playing) => MediaPlayback::Playing { progress },
            // A stopped track (e.g. after launch) is paused for the system:
            // that way it's visible in "Control Center" and starts with the ⏯ button
            (Some(_), PlayerState::Paused | PlayerState::Stopped) => {
                MediaPlayback::Paused { progress }
            }
        };
        if let Err(e) = controls.set_playback(playback) {
            eprintln!("Could not update the system media controls: {e}");
        }
        self.system_media.shown = Some(Shown {
            track: path,
            state,
            position,
            at: now,
        });
    }
}

/// Puts the cover into a temp file and returns a link to it
fn write_cover_file(bytes: &[u8]) -> Option<String> {
    let path = std::env::temp_dir().join(COVER_FILE);
    std::fs::write(&path, bytes).ok()?;
    file_url(&path)
}

/// A `file://` link to a file. On macOS the link is parsed as a URL, so a path with characters
/// that would need encoding won't do — there'd be no cover then. The macOS temp folder
/// (`/var/folders/…`) contains no such characters. Windows takes the path after
/// `file://` as is
fn file_url(path: &Path) -> Option<String> {
    let path = path.to_str()?;
    let plain = |c: char| c.is_ascii_alphanumeric() || "/._-".contains(c);
    if cfg!(target_os = "macos") && !path.chars().all(plain) {
        return None;
    }
    Some(format!("file://{path}"))
}
