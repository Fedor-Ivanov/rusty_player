use rodio::{Decoder, DeviceSinkBuilder, Player as AudioPlayer};
use std::{
    fs::File,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, Sender},
    },
    thread,
    time::Duration,
};

pub enum AudioCmd {
    Play {
        path: PathBuf,
        start_at: Duration,
        reply: Sender<Result<(), String>>,
    },
    Pause,
    Resume,
    Stop,
    Seek(Duration),
    /// Volume multiplier: 0.0 — silence, 1.0 — original volume
    SetVolume(f32),
}

#[derive(Debug)]
pub enum AudioEvent {
    TrackEnded,
}

/// Starts the audio thread. It calls `wake` after each event: whoever receives them
/// must wake up and take them, even if the window is hidden and doesn't redraw itself
pub fn spawn_audio_thread(
    position_ms: Arc<AtomicU64>,
    wake: impl Fn() + Send + 'static,
) -> (Sender<AudioCmd>, Receiver<AudioEvent>) {
    let (tx, rx) = mpsc::channel::<AudioCmd>();
    let (event_tx, event_rx) = mpsc::channel::<AudioEvent>();

    thread::spawn(move || {
        // The device must live for the whole playback.
        let device = DeviceSinkBuilder::open_default_sink()
            .map_err(|e| format!("Could not open the audio device: {e}"));

        let mut player: Option<AudioPlayer> = None;
        // Volume survives a track change: each track plays through a new player
        let mut volume = 1.0;

        loop {
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(AudioCmd::Play {
                    path,
                    start_at,
                    reply,
                }) => {
                    if let Some(old) = player.take() {
                        old.stop();
                    }

                    let result = (|| -> Result<AudioPlayer, String> {
                        let device = device.as_ref().map_err(|e| e.clone())?;

                        let file = File::open(&path)
                            .map_err(|e| format!("Could not open the file: {e}"))?;

                        let source = Decoder::try_from(file)
                            .map_err(|e| format!("Could not decode the file: {e}"))?;

                        let new_player = AudioPlayer::connect_new(device.mixer());
                        new_player.set_volume(volume);
                        new_player.append(source);

                        if !start_at.is_zero() {
                            new_player
                                .try_seek(start_at)
                                .map_err(|e| format!("Could not seek the track: {e}"))?;
                        }

                        Ok(new_player)
                    })();

                    match result {
                        Ok(p) => {
                            player = Some(p);
                            position_ms.store(start_at.as_millis() as u64, Ordering::Relaxed);
                            let _ = reply.send(Ok(()));
                        }
                        Err(e) => {
                            position_ms.store(0, Ordering::Relaxed);
                            let _ = reply.send(Err(e));
                        }
                    }
                }

                Ok(AudioCmd::Pause) => {
                    if let Some(p) = &player {
                        p.pause();
                    }
                }

                Ok(AudioCmd::Resume) => {
                    if let Some(p) = &player {
                        p.play();
                    }
                }

                Ok(AudioCmd::Stop) => {
                    if let Some(p) = player.take() {
                        p.stop();
                    }

                    position_ms.store(0, Ordering::Relaxed);
                }

                Ok(AudioCmd::Seek(pos)) => {
                    position_ms.store(pos.as_millis() as u64, Ordering::Relaxed);

                    if let Some(p) = &player
                        && let Err(err) = p.try_seek(pos)
                    {
                        eprintln!("Could not seek the track: {err}");
                    }
                }

                Ok(AudioCmd::SetVolume(value)) => {
                    volume = value;
                    if let Some(p) = &player {
                        p.set_volume(value);
                    }
                }

                Err(RecvTimeoutError::Timeout) => {
                    let ended = player.as_ref().is_some_and(|p| p.empty());

                    if ended {
                        player = None;
                        position_ms.store(0, Ordering::Relaxed);
                        let _ = event_tx.send(AudioEvent::TrackEnded);
                        wake();
                    } else if let Some(p) = &player {
                        position_ms.store(p.get_pos().as_millis() as u64, Ordering::Relaxed);
                    }
                }

                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    (tx, event_rx)
}
