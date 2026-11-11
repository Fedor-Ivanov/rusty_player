//! Adding tracks: the "+" button with a menu, picking files and a folder, the queue of background
//! adds, progress. Each add knows where the tracks go: to the library or the queue.

use eframe::egui::{self, Rect, Ui, pos2, vec2};
use std::{
    collections::HashSet,
    path::PathBuf,
    time::{Duration, Instant},
};

use super::app::{ADD_BUTTON_SIZE, PlayerApp};
use crate::{
    core::library::{
        loader::{Finished, Loader, Source},
        scan::SUPPORTED_EXTENSIONS,
    },
    ui::{
        action::Action,
        components,
        i18n::{fill, t},
        style::icons::paint_add,
    },
};

// Background adding: how often to check readiness, after how long to show progress
// (a quick add does without it) and the minimum time to keep it so it doesn't flicker
const LOADER_POLL_INTERVAL: Duration = Duration::from_millis(50);
const LOADER_SHOW_DELAY: Duration = Duration::from_millis(400);
const LOADER_MIN_VISIBLE: Duration = Duration::from_millis(500);

/// Where added tracks go
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Destination {
    /// Kept between launches
    Library,
    /// Just to listen now, they don't go into the library
    Queue,
    /// Re-read the tags of tracks already in the library (after a format change)
    LibraryRefresh,
}

/// A queued background add
pub(crate) struct PendingLoad {
    pub(crate) loader: Loader,
    destination: Destination,
}

impl PendingLoad {
    /// Progress banner text with `{done}` and `{total}` — depends on what we're doing
    pub(crate) fn progress_template(&self) -> &'static str {
        match self.destination {
            Destination::Library | Destination::Queue => t().adding_progress,
            Destination::LibraryRefresh => t().updating_library_progress,
        }
    }
}

impl PlayerApp {
    /// The "+" button at the right edge of title row `row` with the "Add files" / "Add folder" menu
    pub(crate) fn render_add_button(&mut self, ui: &mut Ui, row: Rect, destination: Destination) {
        let add_rect = Rect::from_center_size(
            pos2(row.right() - ADD_BUTTON_SIZE / 2.0, row.center().y),
            vec2(ADD_BUTTON_SIZE, ADD_BUTTON_SIZE),
        );
        let id_salt = match destination {
            Destination::Library => "add_to_library",
            Destination::Queue => "add_to_queue",
            Destination::LibraryRefresh => "refresh_library",
        };
        let add =
            components::icon_button(ui, add_rect, components::HitShape::Circle, id_salt, true);
        paint_add(ui, add_rect, add.hover);

        // Menu under "+", aligned to its right edge. Opens and closes
        // on our `clicked` — it counts only clicks inside the circle
        egui::Popup::menu(&add.response)
            .open_memory(add.clicked.then_some(egui::SetOpenCommand::Toggle))
            .align(egui::RectAlign::BOTTOM_END)
            .gap(4.0)
            .show(|ui| {
                components::menu_style(ui);
                if components::menu_item(ui, t().add_files).clicked() {
                    self.dispatch(Action::AddFiles(destination));
                }
                if components::menu_item(ui, t().add_folder).clicked() {
                    self.dispatch(Action::AddFolder(destination));
                }
            });
    }

    /// Picking individual files
    pub(crate) fn add_files(&mut self, destination: Destination) {
        let Some(paths) = rfd::FileDialog::new()
            .add_filter(t().audio_files, SUPPORTED_EXTENSIONS)
            .pick_files()
        else {
            // The user closed the dialog: keep the current track
            return;
        };
        self.start_loading(Source::Files(paths), destination);
    }

    /// Picking a folder: all audio files in it are added, including subfolders
    pub(crate) fn add_folder(&mut self, destination: Destination) {
        let Some(dir) = rfd::FileDialog::new().pick_folder() else {
            return;
        };
        self.start_loading(Source::Folder(dir), destination);
    }

    /// Dropped files and folders: files — as one add (unsupported ones, e.g.
    /// images, are skipped), each folder — as its own, with all audio files inside
    pub(crate) fn add_paths(&mut self, paths: Vec<PathBuf>, destination: Destination) {
        let (folders, files): (Vec<PathBuf>, Vec<PathBuf>) =
            paths.into_iter().partition(|path| path.is_dir());
        let files: Vec<PathBuf> = files
            .into_iter()
            .filter(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| {
                        SUPPORTED_EXTENSIONS
                            .iter()
                            .any(|known| ext.eq_ignore_ascii_case(known))
                    })
            })
            .collect();
        if !files.is_empty() {
            self.start_loading(Source::Files(files), destination);
        }
        for folder in folders {
            self.start_loading(Source::Folder(folder), destination);
        }
    }

    pub(crate) fn start_loading(&mut self, source: Source, destination: Destination) {
        // What's already in the library isn't read a second time: adding the same
        // folder again adds only new files
        let skip = match destination {
            Destination::Library => self.core.library.paths().clone(),
            Destination::Queue | Destination::LibraryRefresh => HashSet::new(),
        };
        self.loaders.push_back(PendingLoad {
            loader: Loader::start(source, skip),
            destination,
        });
    }

    /// Takes ready tracks from the background add. Called every frame
    pub(crate) fn poll_loaders(&mut self, ctx: &egui::Context) {
        let Some(pending) = self.loaders.front_mut() else {
            return;
        };
        pending.loader.update();

        // Progress is shown only if adding takes a while: a few files
        // are added faster, and then the progress doesn't flicker at all
        if self.loader_shown_at.is_none() && pending.loader.elapsed() >= LOADER_SHOW_DELAY {
            self.loader_shown_at = Some(Instant::now());
        }
        // Progress already shown is kept for at least LOADER_MIN_VISIBLE — otherwise it could
        // appear and vanish right away, which looks worse than not showing it at all
        let can_hide = self
            .loader_shown_at
            .is_none_or(|shown_at| shown_at.elapsed() >= LOADER_MIN_VISIBLE);

        if pending.loader.is_finished() && can_hide {
            let pending = self.loaders.pop_front();
            self.loader_shown_at = None;
            if let Some(pending) = pending {
                self.dispatch(Action::TracksLoaded {
                    finished: pending.loader.finish(),
                    destination: pending.destination,
                });
            }
        }

        // While adding is in progress, frames are needed even without user actions:
        // otherwise the progress would update only when the mouse moves
        if !self.loaders.is_empty() {
            ctx.request_repaint_after(LOADER_POLL_INTERVAL);
        }
    }

    /// Puts the added tracks where they belong — all at once
    pub(crate) fn apply_loaded(&mut self, finished: Finished, destination: Destination) {
        // Library refresh: files that couldn't be read just stay
        // as they were (missing ones are marked anyway) — errors aren't shown
        if destination == Destination::LibraryRefresh {
            self.core.library.refresh(finished.tracks);
            return;
        }

        if !finished.tracks.is_empty() {
            match destination {
                Destination::Library => {
                    self.core.library.add(finished.tracks);
                }
                Destination::Queue => {
                    let result = self.core.controller.add_tracks(finished.tracks);
                    self.report_error(result);
                }
                Destination::LibraryRefresh => unreachable!("handled above"),
            }
        }

        // Everything found was already in the library — not an error, just nothing to add
        if finished.total == Some(0) && finished.skipped == 0 {
            self.error = Some(t().no_audio_found.into());
        } else if !finished.errors.is_empty() {
            self.error = Some(summarize_errors(&finished.errors));
        }
    }

    /// Cancels the current add: already read tracks are not added
    pub(crate) fn cancel_loading(&mut self) {
        self.loaders.pop_front();
        self.loader_shown_at = None;
    }
}

/// Adding error text: the first few files and how many more weren't added —
/// with thousands of files the full list wouldn't fit on screen
fn summarize_errors(errors: &[String]) -> String {
    const SHOWN: usize = 3;
    let mut text = errors[..errors.len().min(SHOWN)].join("\n");
    if errors.len() > SHOWN {
        let more = t().count(errors.len() - SHOWN, &t().more_file_count);
        text += "\n";
        text += &fill(t().more_files_failed, &[("more", &more)]);
    }
    text
}
