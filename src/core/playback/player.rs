//! Playback control: current track, play/pause, transitions, link to the audio thread.

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    time::Duration,
};

use super::{
    audio::{self, AudioCmd, AudioEvent},
    queue::{RepeatMode, TrackList},
};
use crate::core::library::Track;

#[derive(Serialize, Debug, Clone, Copy, PartialEq)]
pub enum PlayerState {
    Playing,
    Paused,
    Stopped,
}

#[derive(Serialize, Debug, Clone)]
pub struct Player {
    pub current_track: Option<Track>,
    pub state: PlayerState,
    pub track_list: TrackList,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            current_track: None,
            state: PlayerState::Stopped,
            track_list: TrackList::default(),
        }
    }
}

/// Volume: slider position and mute. Remembered between launches
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Volume {
    /// Slider position: 0.0..=1.0
    pub level: f32,
    pub muted: bool,
}

impl Default for Volume {
    fn default() -> Self {
        Self {
            level: 1.0,
            muted: false,
        }
    }
}

impl Volume {
    /// Audible volume: 0.0 if muted or the slider is at zero
    pub fn audible(self) -> f32 {
        if self.muted { 0.0 } else { self.level }
    }
}

/// Volume to unmute to if the slider was at zero
const UNMUTE_LEVEL: f32 = 0.5;

/// Player state + link to the audio thread: commands go there, events (track finished) come back
pub struct PlayerController {
    player: Player,
    audio: Sender<AudioCmd>,
    events: Receiver<AudioEvent>,
    position_ms: Arc<AtomicU64>,
    volume: Volume,
    /// Grows on every queue change (tracks, current track, shuffle, repeat):
    /// it shows the queue needs saving
    queue_revision: u64,
}

impl PlayerController {
    /// `wake` is called from the audio thread when an event appears for `poll_events`
    /// (a track finished): it should trigger `poll_events` without waiting for user actions
    pub fn new(wake: impl Fn() + Send + 'static) -> Self {
        let position_ms = Arc::new(AtomicU64::new(0));
        let (audio, events) = audio::spawn_audio_thread(position_ms.clone(), wake);

        Self {
            player: Player::default(),
            audio,
            events,
            position_ms,
            volume: Volume::default(),
            queue_revision: 0,
        }
    }

    pub fn player(&self) -> &Player {
        &self.player
    }

    pub fn position_ms(&self) -> u64 {
        self.position_ms.load(Ordering::Relaxed)
    }

    pub fn queue_revision(&self) -> u64 {
        self.queue_revision
    }

    /// Links the queue to saved playlist `playlist` (or unlinks — `None`),
    /// see `TrackList::playlist`
    pub fn link_playlist(&mut self, playlist: Option<u64>) {
        if self.player.track_list.playlist != playlist {
            self.player.track_list.playlist = playlist;
            self.queue_revision += 1;
        }
    }

    pub fn volume(&self) -> Volume {
        self.volume
    }

    /// Sets the whole volume state — e.g. the one saved from the previous run
    pub fn set_volume(&mut self, volume: Volume) {
        self.volume = Volume {
            level: volume.level.clamp(0.0, 1.0),
            muted: volume.muted,
        };
        // Hearing perceives loudness roughly logarithmically: with the slider squared
        // the middle sounds like the middle, not almost like the maximum
        let gain = self.volume.audible().powi(2);
        // The audio thread lives as long as the controller: sending can't fail
        let _ = self.audio.send(AudioCmd::SetVolume(gain));
    }

    /// Volume slider: moving it also unmutes if muted
    pub fn set_volume_level(&mut self, level: f32) {
        self.set_volume(Volume {
            level,
            muted: false,
        });
    }

    /// Mute button. Unmutes if the sound is inaudible — muted or the slider at zero
    /// (then the slider moves to the middle); otherwise mutes, remembering the slider position
    pub fn toggle_mute(&mut self) {
        let volume = self.volume;
        self.set_volume(if volume.audible() == 0.0 {
            Volume {
                level: if volume.level == 0.0 {
                    UNMUTE_LEVEL
                } else {
                    volume.level
                },
                muted: false,
            }
        } else {
            Volume {
                muted: true,
                ..volume
            }
        });
    }

    /// Handles audio thread events. Call at the start of every frame.
    /// Returns `true` if the track played to the end.
    pub fn poll_events(&mut self) -> bool {
        let mut ended = false;

        while let Ok(event) = self.events.try_recv() {
            match event {
                AudioEvent::TrackEnded => {
                    self.player.state = PlayerState::Stopped;
                    ended = true;
                }
            }
        }

        ended
    }

    pub fn seek(&self, position_ms: u64) -> Result<(), String> {
        // Right away, so the UI doesn't show the old position until the audio thread handles the command
        self.position_ms.store(position_ms, Ordering::Relaxed);

        self.audio
            .send(AudioCmd::Seek(Duration::from_millis(position_ms)))
            .map_err(|e| e.to_string())
    }

    /// Makes the track current. Playback doesn't start — that's what `play` is for.
    fn set_current_track(&mut self, track: Track) -> Result<(), String> {
        // If something was already playing, stop it: another track is selected
        self.audio.send(AudioCmd::Stop).map_err(|e| e.to_string())?;

        self.position_ms.store(0, Ordering::Relaxed);

        self.player.current_track = Some(track);
        self.player.state = PlayerState::Stopped;

        Ok(())
    }

    /// Restores the queue of the previous run: the same tracks, shuffle and repeat modes.
    /// The current track is selected but not playing — as if it had just been selected
    pub fn restore_track_list(&mut self, mut list: TrackList) {
        list.current_index = list
            .current_index
            .filter(|&index| index < list.tracks.len());
        self.player.current_track = list.current_index.map(|index| list.tracks[index].clone());
        self.player.state = PlayerState::Stopped;
        // The shuffled order isn't saved — shuffle again
        if list.shuffle {
            list.reshuffle();
        }
        self.player.track_list = list;
    }

    /// Appends tracks to the list. The current track and playback don't change.
    pub fn append_tracks(&mut self, tracks: impl IntoIterator<Item = Track>) {
        let list = &mut self.player.track_list;
        let first_new = list.tracks.len();
        list.tracks.extend(tracks);

        // New tracks go in random order to the end of the shuffled queue
        let mut added: Vec<usize> = (first_new..list.tracks.len()).collect();
        fastrand::shuffle(&mut added);
        list.shuffle_order.extend(added);
        self.queue_revision += 1;
    }

    /// Appends tracks to the queue. If nothing is selected yet, the first added one
    /// becomes current; otherwise the current track is left alone
    pub fn add_tracks(&mut self, tracks: Vec<Track>) -> Result<(), String> {
        if tracks.is_empty() {
            return Ok(());
        }
        let first_new = self.player.track_list.tracks.len();
        let nothing_selected = self.player.current_track.is_none();
        self.append_tracks(tracks);
        if nothing_selected {
            self.select(first_new)?;
        }
        Ok(())
    }

    /// Replaces the whole queue with `tracks` and starts playback from the first of them,
    /// or, with shuffle on, from a random one
    pub fn play_all(&mut self, tracks: Vec<Track>) -> Result<(), String> {
        let list = &mut self.player.track_list;
        list.tracks = tracks;
        list.current_index = None;
        // The queue was replaced with other content — it's no longer linked to the previous playlist
        list.playlist = None;
        list.reshuffle();
        self.queue_revision += 1;
        // Without a current track, "next" is the first in playback order
        let Some(first) = list.neighbor(1, true) else {
            self.player.current_track = None;
            self.player.state = PlayerState::Stopped;
            self.position_ms.store(0, Ordering::Relaxed);
            return self.audio.send(AudioCmd::Stop).map_err(|e| e.to_string());
        };
        self.select(first)?;
        self.play()
    }

    /// Removes tracks from the list. If the current one is among them, playback stops
    /// and there's no current track anymore — the player is as before a track was selected
    pub fn remove_tracks(&mut self, indices: &BTreeSet<usize>) -> Result<(), String> {
        let removes_current = self
            .player
            .track_list
            .current_index
            .is_some_and(|current| indices.contains(&current));
        self.player.track_list.remove(indices);
        self.queue_revision += 1;

        if removes_current {
            self.player.current_track = None;
            self.player.state = PlayerState::Stopped;
            self.position_ms.store(0, Ordering::Relaxed);
            self.audio.send(AudioCmd::Stop).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Clears the queue: playback stops, there's no current track anymore
    pub fn clear(&mut self) -> Result<(), String> {
        let all = (0..self.player.track_list.tracks.len()).collect();
        self.link_playlist(None);
        self.remove_tracks(&all)
    }

    /// Toggles shuffle
    pub fn toggle_shuffle(&mut self) {
        let list = &mut self.player.track_list;
        list.shuffle = !list.shuffle;
        if list.shuffle {
            list.reshuffle();
        }
        self.queue_revision += 1;
    }

    /// Turns on a repeat mode; pressing an already active one turns it off
    pub fn toggle_repeat(&mut self, mode: RepeatMode) {
        let list = &mut self.player.track_list;
        list.repeat = if list.repeat == mode {
            RepeatMode::Off
        } else {
            mode
        };
        self.queue_revision += 1;
    }

    /// Makes a track from the list current by its position.
    pub fn select(&mut self, index: usize) -> Result<(), String> {
        let track = self
            .player
            .track_list
            .tracks
            .get(index)
            .cloned()
            .ok_or("There is no track with this number in the list")?;
        self.set_current_track(track)?;
        self.player.track_list.current_index = Some(index);
        self.queue_revision += 1;
        Ok(())
    }

    /// Called when a track finishes. With repeat-one — starts it again,
    /// otherwise — the next in order (respecting shuffle). If the list ended and repeat
    /// all is off, makes the first track current but doesn't start it.
    pub fn play_next(&mut self) -> Result<(), String> {
        // The current track was removed from the list: "finished" could arrive after stopping —
        // don't start anything on our own
        if self.player.current_track.is_none() {
            return Ok(());
        }
        let list = &self.player.track_list;
        if list.repeat == RepeatMode::One
            && let Some(current) = list.current_index
        {
            self.select(current)?;
            return self.play();
        }

        match list.neighbor(1, list.repeat == RepeatMode::All) {
            Some(next) => {
                self.select(next)?;
                self.play()
            }
            None => match list.play_order().first() {
                Some(&first) => self.select(first),
                None => Ok(()),
            },
        }
    }

    /// "Next" button: the next track (respecting shuffle), after the last — the first.
    /// If the track was playing, the new one starts too.
    pub fn skip_next(&mut self) -> Result<(), String> {
        match self.player.track_list.neighbor(1, true) {
            Some(next) => self.switch_to(next),
            None => Ok(()),
        }
    }

    /// "Previous" button: if the track has been playing for more than a few seconds, rewinds to its
    /// start (like most players), otherwise — the previous track, before the first — the last.
    pub fn skip_prev(&mut self) -> Result<(), String> {
        const RESTART_THRESHOLD_MS: u64 = 3000;

        if self.player.track_list.current_index.is_some()
            && self.position_ms() > RESTART_THRESHOLD_MS
        {
            return self.seek(0);
        }
        match self.player.track_list.neighbor(-1, true) {
            Some(prev) => self.switch_to(prev),
            None => Ok(()),
        }
    }

    /// Switches to a track, keeping the state: if it was playing, the new one plays too
    fn switch_to(&mut self, index: usize) -> Result<(), String> {
        let was_playing = self.player.state == PlayerState::Playing;
        self.select(index)?;
        if was_playing { self.play() } else { Ok(()) }
    }

    /// Starts or resumes the current track. From a stop, plays from `position_ms` (after a seek
    /// or a restored position) and waits until the audio thread opens the file: a broken file
    /// returns an error here instead of failing silently
    pub fn play(&mut self) -> Result<(), String> {
        match self.player.state {
            PlayerState::Playing => return Ok(()),

            PlayerState::Paused => {
                self.audio
                    .send(AudioCmd::Resume)
                    .map_err(|e| e.to_string())?;

                self.player.state = PlayerState::Playing;

                return Ok(());
            }

            PlayerState::Stopped => {}
        }

        let path = self
            .player
            .current_track
            .as_ref()
            .map(|t| t.path.clone())
            .ok_or("No track selected")?;

        let start_at = Duration::from_millis(self.position_ms());

        let (reply_tx, reply_rx) = mpsc::channel();

        self.audio
            .send(AudioCmd::Play {
                path,
                start_at,
                reply: reply_tx,
            })
            .map_err(|e| e.to_string())?;

        reply_rx.recv().map_err(|e| e.to_string())??;

        // `TrackEnded` from the previous track that arrived before the new one started is stale
        while self.events.try_recv().is_ok() {}

        self.player.state = PlayerState::Playing;

        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), String> {
        if self.player.state != PlayerState::Playing {
            return Ok(());
        }

        self.audio
            .send(AudioCmd::Pause)
            .map_err(|e| e.to_string())?;

        self.player.state = PlayerState::Paused;

        Ok(())
    }
}
