//! Right panel: the queue — what we're listening to now. Tracks get here from the library, playlists or
//! via the "+" button bypassing the library. At the bottom — shuffle and repeat.

use eframe::egui::{self, Align, Align2, FontId, Layout, Rect, Sense, Ui, UiBuilder, pos2, vec2};

use crate::{
    core::playback::RepeatMode,
    ui::{
        action::Action,
        app::{ADD_BUTTON_SIZE, ListFocus, PlayerApp},
        components,
        i18n::{fill, t},
        loading::Destination,
        style::icons::{paint_broom, paint_repeat, paint_save_playlist, paint_shuffle},
        style::theme::{TEXT, TEXT_FAINT},
    },
};

// Gap between icon buttons: repeat in the bottom block and buttons in the header
const ICON_BUTTON_GAP: f32 = 4.0;
/// Buttons in the queue header: clear, save, "+"
const HEADER_BUTTONS: f32 = 3.0;
/// Minimum gap between the title and the buttons
const TITLE_BUTTONS_GAP: f32 = 8.0;

impl PlayerApp {
    pub(crate) fn render_queue(&mut self, ui: &mut Ui) {
        // The whole panel area — remembered before the title so the placeholder is centered in it
        let panel_rect = ui.max_rect();
        egui::Frame::new().inner_margin(16.0).show(ui, |ui| {
            // Next to the title — how many tracks are in the queue; empty — no number:
            // "No tracks" is shown there anyway
            let count = self.core.controller.player().track_list.tracks.len();
            // Three buttons on the right — the title takes the space to their left and is truncated with "…",
            // rather than going under the buttons
            let full = ui.available_rect_before_wrap();
            let buttons_width = HEADER_BUTTONS * ADD_BUTTON_SIZE
                + (HEADER_BUTTONS - 1.0) * ICON_BUTTON_GAP
                + TITLE_BUTTONS_GAP;
            let title_area = Rect::from_min_max(
                full.min,
                pos2(
                    (full.right() - buttons_width).max(full.left()),
                    full.bottom(),
                ),
            );
            let title_row = ui
                .scope_builder(UiBuilder::new().max_rect(title_area), |ui| {
                    if count == 0 {
                        components::block_title(ui, t().queue, Align::LEFT, TEXT)
                    } else {
                        // The linked playlist — next to the title, before the track count
                        let tracks = t().count(count, &t().track_count);
                        let note = match self.queue_playlist_name() {
                            Some(name) => format!("· {name} · {tracks}"),
                            None => format!("· {tracks}"),
                        };
                        components::block_title_with_note(ui, t().queue, &note)
                    }
                })
                .inner;
            // The button row — the same height, but full width
            let row = Rect::from_x_y_ranges(full.x_range(), title_row.y_range());
            // Left to right: "+", save, clear. "+" is placed at the right edge of the given
            // row — we give it a row shortened by the two buttons on the right
            let add_row = row.with_max_x(row.right() - 2.0 * (ADD_BUTTON_SIZE + ICON_BUTTON_GAP));
            self.render_add_button(ui, add_row, Destination::Queue);
            self.render_queue_buttons(ui, row, count > 0);
        });

        if self.core.controller.player().track_list.tracks.is_empty() {
            if self.loaders.is_empty() {
                ui.painter().text(
                    panel_rect.center(),
                    Align2::CENTER_CENTER,
                    t().no_tracks,
                    FontId::proportional(14.0),
                    TEXT_FAINT,
                );
            }
            return;
        }

        // A single selected track from the library — it can be shown there and its album opened.
        // The menu opens after selection, so the selection at the start of the frame is the one
        // it refers to
        let single = match self.queue_view.selection().iter().collect::<Vec<_>>()[..] {
            [&index] => self
                .core
                .controller
                .player()
                .track_list
                .tracks
                .get(index)
                .cloned(),
            _ => None,
        };
        let album = single.as_ref().and_then(|track| self.album_route(track));
        let in_library = single
            .filter(|track| self.core.library.paths().contains(&track.path))
            .map(|track| track.path);

        // How many selected tracks are not from the library (added with the "+" button bypassing it) — for
        // the "Add to library" item. Removing from the library isn't possible here: the queue is about what
        // we're listening to, not about the library. The menu opens after selection, so the selection
        // at the start of the frame is the one it refers to
        let list = &self.core.controller.player().track_list;
        let library_paths = self.core.library.paths();
        let outside_library = self
            .queue_view
            .selection()
            .iter()
            .filter_map(|&index| list.tracks.get(index))
            .filter(|track| !library_paths.contains(&track.path))
            .count();
        // The menu only knows how many rows are selected: which ones — we find out after the list
        let mut remove = false;
        let mut add_to_library = false;
        let mut show_in_library = false;
        let mut go_to_album = false;
        let current = list.current_index;
        let response = self.queue_view.show(
            ui,
            &list.tracks,
            |index| components::RowState {
                current: current == Some(index),
                missing: false,
            },
            self.focus == ListFocus::Queue,
            |ui, selected| {
                if in_library.is_some() && components::menu_item(ui, t().show_in_library).clicked()
                {
                    show_in_library = true;
                }
                if album.is_some() && components::menu_item(ui, t().go_to_album).clicked() {
                    go_to_album = true;
                }
                if outside_library > 0 {
                    let label = t().for_tracks(
                        t().add_to_library,
                        t().add_items_to_library,
                        outside_library,
                    );
                    if components::menu_item(ui, &label).clicked() {
                        add_to_library = true;
                    }
                }
                let label =
                    t().for_tracks(t().remove_from_queue, t().remove_items_from_queue, selected);
                if components::menu_item(ui, &label).clicked() {
                    remove = true;
                }
            },
        );

        if response.interacted {
            self.focus = ListFocus::Queue;
        }
        if show_in_library && let Some(path) = in_library {
            self.dispatch(Action::ShowInLibrary(path));
        } else if go_to_album && let Some(route) = album {
            self.dispatch(Action::Navigate(route));
        } else if add_to_library {
            let selected = self.queue_view.selection().iter().copied().collect();
            self.dispatch(Action::AddQueueTracksToLibrary(selected));
        } else if remove || response.delete {
            let selected = self.queue_view.selection().clone();
            self.dispatch(Action::RemoveFromQueue(selected));
        }
        if let Some(index) = response.activated {
            self.dispatch(Action::PlayQueueTrack(index));
        }
    }

    /// Buttons to the right of "+" in the queue header: save it as a playlist (floppy disk)
    /// and clear it (broom).
    /// An empty queue can't be saved or cleared — the buttons are dimmed
    fn render_queue_buttons(&mut self, ui: &mut Ui, row: Rect, enabled: bool) {
        let size = vec2(ADD_BUTTON_SIZE, ADD_BUTTON_SIZE);
        // Slots from right to left: 0 — at the right edge (clear), 1 — to the left (save),
        // 2 — "+"
        let rect_at = |slot: f32| {
            Rect::from_center_size(
                pos2(
                    row.right()
                        - ADD_BUTTON_SIZE / 2.0
                        - slot * (ADD_BUTTON_SIZE + ICON_BUTTON_GAP),
                    row.center().y,
                ),
                size,
            )
        };

        let save_rect = rect_at(1.0);
        // The queue is linked to a playlist and hasn't changed — nothing to save
        let save_enabled = enabled && !self.core.queue_matches_playlist();
        let save = components::icon_button(
            ui,
            save_rect,
            components::HitShape::Circle,
            "save_queue",
            save_enabled,
        );
        paint_save_playlist(ui, save_rect, save_enabled, save.hover);
        // The queue is linked to a playlist — a click overwrites it; save as new —
        // from the right-click menu. Not linked — a click asks for a new name
        let linked = self.core.queue_playlist().zip(self.queue_playlist_name());
        let save_clicked = save.clicked;
        let mut save_as_new = false;
        let mut save_to_linked = false;
        match &linked {
            Some((_, name)) => {
                let save_to = fill(t().save_queue_to, &[("playlist", name)]);
                save.response.context_menu(|ui| {
                    components::menu_style(ui);
                    if components::menu_item(ui, &save_to).clicked() {
                        save_to_linked = true;
                    }
                    if components::menu_item(ui, t().save_as_new_playlist).clicked() {
                        save_as_new = true;
                    }
                });
                save.response.on_hover_text(save_to);
            }
            None => {
                save.response.on_hover_text(t().save_queue);
            }
        }

        let clear_rect = rect_at(0.0);
        let clear = components::icon_button(
            ui,
            clear_rect,
            components::HitShape::Circle,
            "clear_queue",
            enabled,
        );
        paint_broom(ui, clear_rect, enabled, clear.hover);
        let clear_clicked = clear.clicked;
        clear.response.on_hover_text(t().clear_queue);

        match linked {
            Some((id, _)) if save_clicked || save_to_linked => {
                self.dispatch(Action::SaveQueueToPlaylist(id));
            }
            _ if save_clicked || save_as_new => self.dispatch(Action::RequestSaveQueue),
            _ => {}
        }
        if clear_clicked {
            self.dispatch(Action::ClearQueue);
        }
    }

    /// Title of the playlist the queue is linked to (see `Core::queue_playlist`)
    fn queue_playlist_name(&self) -> Option<String> {
        let id = self.core.queue_playlist()?;
        self.core
            .playlists
            .get(id)
            .map(|playlist| playlist.name.clone())
    }

    /// The queue's bottom block: shuffle on the left, two repeat buttons on the right
    pub(crate) fn render_playback_modes(&mut self, ui: &mut Ui) {
        let list = &self.core.controller.player().track_list;
        let (shuffle, repeat) = (list.shuffle, list.repeat);
        let size = vec2(ADD_BUTTON_SIZE, ADD_BUTTON_SIZE);

        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            let (clicked, active, hover) =
                components::toggle_button(ui, rect, "shuffle", shuffle, t().shuffle);
            paint_shuffle(ui, rect, active, hover);
            if clicked {
                self.dispatch(Action::ToggleShuffle);
            }

            // Repeat buttons are pushed right. Only one of them can be on —
            // RepeatMode itself ensures that
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = ICON_BUTTON_GAP;
                // In right_to_left the rightmost button is added first
                for (mode, id_salt, tooltip) in [
                    (RepeatMode::All, "repeat_all", t().repeat_all),
                    (RepeatMode::One, "repeat_one", t().repeat_one),
                ] {
                    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                    let (clicked, active, hover) =
                        components::toggle_button(ui, rect, id_salt, repeat == mode, tooltip);
                    paint_repeat(ui, rect, mode == RepeatMode::One, active, hover);
                    if clicked {
                        self.dispatch(Action::ToggleRepeat(mode));
                    }
                }
            });
        });
    }
}
