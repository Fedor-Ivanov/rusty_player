//! Track list — shared by Songs, album and playlist pages and the queue: virtualization (only visible
//! rows are drawn), click and mouse marquee selection, keys, context menu.

use eframe::egui::{self, Align, Key, Label, Layout, Rect, RichText, Sense, Ui, pos2, vec2};
use std::collections::BTreeSet;

use super::selection::{Marquee, Selection};
use crate::{
    core::library::Track,
    ui::{
        components::format_secs,
        i18n::t,
        style::theme::{
            HIGHLIGHT, HIGHLIGHT_STRONG, HOVER_FADE_SECS, MARQUEE_FILL, MARQUEE_STROKE,
            SELECTION_STROKE_WIDTH, TEXT, TEXT_FAINT, TEXT_MUTED, TRACK_ACTIVE_FADE_SECS,
        },
    },
};

// Rows: the height is fixed — without it the list can't be virtualized,
// since a row's position can't be computed without measuring
const TRACK_ROW_HEIGHT: f32 = 30.0;
const TRACK_ROW_PADDING: f32 = 8.0;
const TRACK_ROW_GAP: f32 = 6.0;
// Auto-scroll speed during marquee selection: points per second for each point
// the cursor has gone past the list edge
const MARQUEE_AUTOSCROLL_SPEED: f32 = 8.0;

/// State of one list between frames: selection and marquee selection
pub struct TrackListView {
    /// Distinguishes lists: each has its own scroll and background ids
    id: &'static str,
    selection: Selection,
    marquee: Option<Marquee>,
    /// The row to scroll to in the next frame (see `reveal`)
    scroll_to: Option<usize>,
}

/// What happened to the list during the frame — the caller decides
pub struct TrackListResponse {
    /// Double click on a track
    pub activated: Option<usize>,
    /// Delete or Backspace was pressed while the list is active
    pub delete: bool,
    /// The list was used with the mouse — it becomes active for the keyboard
    pub interacted: bool,
}

/// What to show in a row
pub struct RowState {
    /// The playing track: highlighted
    pub current: bool,
    /// The file is missing from disk: muted text, a tooltip on hover
    pub missing: bool,
}

impl TrackListView {
    pub fn new(id: &'static str) -> Self {
        Self {
            id,
            selection: Selection::default(),
            marquee: None,
            scroll_to: None,
        }
    }

    /// Selects row `index` — the same as clicking it — and scrolls the list to it
    /// when it's shown: "Show in library" from the queue
    pub fn reveal(&mut self, index: usize) {
        self.selection.click(index, egui::Modifiers::NONE);
        self.scroll_to = Some(index);
    }

    pub fn selection(&self) -> &BTreeSet<usize> {
        self.selection.indices()
    }

    /// Track indices shift after removal — the old selection points to nothing
    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    /// Draws the list. `row_state` — what to show in a row by its index; `keyboard` — whether
    /// the list is active for keys (⌘A, Esc, Delete); `context_menu` draws right-click menu
    /// items and receives the number of selected tracks.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        tracks: &[Track],
        row_state: impl Fn(usize) -> RowState,
        keyboard: bool,
        mut context_menu: impl FnMut(&mut Ui, usize),
    ) -> TrackListResponse {
        let track_count = tracks.len();
        let mut response = TrackListResponse {
            activated: None,
            delete: false,
            interacted: false,
        };
        if track_count == 0 {
            return response;
        }

        // While text is being typed (e.g. in search), Backspace and ⌘A are for the text, not for the list
        if keyboard && !ui.ctx().text_edit_focused() {
            let (delete, select_all, deselect) = ui.input(|i| {
                (
                    i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace),
                    i.modifiers.command && i.key_pressed(Key::A),
                    i.key_pressed(Key::Escape),
                )
            });
            response.delete = delete && !self.selection.is_empty();
            if select_all {
                self.selection.select_all(track_count);
            }
            if deselect {
                self.selection.clear();
            }
        }

        let modifiers = ui.input(|i| i.modifiers);
        // show_rows takes the row spacing from the Ui it's called in
        ui.spacing_mut().item_spacing.y = TRACK_ROW_GAP;
        let mut scroll = egui::ScrollArea::vertical();
        // Show the row from `reveal` — in the middle of the visible part of the list
        if let Some(row) = self.scroll_to.take() {
            let row_stride = TRACK_ROW_HEIGHT + TRACK_ROW_GAP;
            let offset = row as f32 * row_stride - (ui.available_height() - TRACK_ROW_HEIGHT) / 2.0;
            scroll = scroll.vertical_scroll_offset(offset.max(0.0));
        }
        scroll
            .id_salt(self.id)
            // The padding is set on the ScrollArea itself: the scrollbar stays at the panel edge,
            // and the padding scrolls along with the list. A Frame inside show_rows won't do —
            // its padding would break the row position calculation
            .content_margin(egui::Margin {
                left: 16,
                right: 16,
                top: 0,
                bottom: 16,
            })
            .show_rows(ui, TRACK_ROW_HEIGHT, track_count, |ui, rows| {
                // Top of the whole list content (row 0) in screen coordinates: show_rows
                // yields only the visible chunk, starting at row rows.start. The row index
                // under the cursor is computed from the row height, not from drawn widgets —
                // rows off screen don't exist, but they still need to be marquee-selectable
                let row_stride = TRACK_ROW_HEIGHT + TRACK_ROW_GAP;
                let content_top = ui.max_rect().top() - rows.start as f32 * row_stride;
                let row_at = |y: f32| {
                    (((y - content_top) / row_stride).floor().max(0.0) as usize)
                        .min(track_count - 1)
                };

                // Background under the rows — for marquee selection and clicks on empty space.
                // Added before the rows: the rows end up on top and get clicks,
                // while dragging goes to the background (rows don't catch it)
                let visible = ui.clip_rect();
                let background = ui.interact(
                    visible,
                    egui::Id::unique(self.id).with("marquee"),
                    Sense::click_and_drag(),
                );
                if background.clicked() {
                    self.selection.clear();
                    response.interacted = true;
                }
                if background.drag_started()
                    && let Some(start) = background.interact_pointer_pos()
                {
                    self.marquee = Some(Marquee {
                        start: pos2(start.x, start.y - content_top),
                        // With ⌘/Ctrl the area is added to the existing selection
                        base: if modifiers.command {
                            self.selection.indices().clone()
                        } else {
                            BTreeSet::new()
                        },
                    });
                    response.interacted = true;
                }

                for index in rows {
                    let row = track_row(
                        ui,
                        index,
                        &tracks[index],
                        &row_state(index),
                        self.selection.contains(index),
                    );

                    if row.clicked() {
                        self.selection.click(index, modifiers);
                        response.interacted = true;
                    }
                    if row.double_clicked() {
                        response.activated = Some(index);
                    }
                    // A right click on an unselected track selects it — the menu acts
                    // on the selection, like in Finder
                    if row.secondary_clicked() {
                        if !self.selection.contains(index) {
                            self.selection.click(index, egui::Modifiers::NONE);
                        }
                        response.interacted = true;
                    }
                    let selected = self.selection.len();
                    row.context_menu(|ui| {
                        super::menu_style(ui);
                        context_menu(ui, selected);
                    });
                }

                // The frame — after the rows, so it lies on top of them
                if let Some(marquee) = &self.marquee
                    && background.dragged()
                    && let Some(pointer) = background.interact_pointer_pos()
                {
                    let start = pos2(marquee.start.x, content_top + marquee.start.y);
                    let (from, to) = (row_at(start.y), row_at(pointer.y));
                    self.selection
                        .select_range(&marquee.base, from.min(to)..=from.max(to), from);

                    ui.painter().rect(
                        Rect::from_two_pos(start, pointer),
                        4,
                        MARQUEE_FILL,
                        egui::Stroke::new(SELECTION_STROKE_WIDTH, MARQUEE_STROKE),
                        egui::StrokeKind::Inside,
                    );

                    // The cursor went above or below the list — scroll there, the faster
                    // the farther the cursor. A positive delta moves the content down
                    let overshoot = if pointer.y < visible.top() {
                        visible.top() - pointer.y
                    } else if pointer.y > visible.bottom() {
                        visible.bottom() - pointer.y
                    } else {
                        0.0
                    };
                    if overshoot != 0.0 {
                        let dt = ui.input(|i| i.stable_dt);
                        ui.scroll_with_delta(vec2(0.0, overshoot * MARQUEE_AUTOSCROLL_SPEED * dt));
                        ui.ctx().request_repaint();
                    }
                }
                if background.drag_stopped() {
                    self.marquee = None;
                }
            });

        response
    }
}

/// A fixed-height list row: title on the left (truncated with "…"), duration on the right
fn track_row(
    ui: &mut Ui,
    index: usize,
    track: &Track,
    state: &RowState,
    is_selected: bool,
) -> egui::Response {
    let (row_rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), TRACK_ROW_HEIGHT), Sense::hover());
    // A stable row id: egui stores animation state by it. A position-based id
    // won't do — rows change places when scrolling
    let row_id = ui.scope_id().with(("track_row", index));
    // A normal cursor, not a "hand": a click selects rather than plays the track — playing is by double click
    let mut row = ui.interact(row_rect, row_id, Sense::click());
    if state.missing {
        row = row.on_hover_text(t().file_missing);
    }

    // Active, selected and hover states change smoothly: background and text color blend
    let active = ui.ctx().animate_bool_with_time_and_easing(
        row_id.with("active"),
        state.current,
        TRACK_ACTIVE_FADE_SECS,
        egui::emath::easing::cubic_in_out,
    );
    let selected = ui.ctx().animate_bool_with_time_and_easing(
        row_id.with("selected"),
        is_selected,
        HOVER_FADE_SECS,
        egui::emath::easing::cubic_out,
    );
    let hover = ui.ctx().animate_bool_with_time_and_easing(
        row_id.with("hover"),
        row.hovered() && !state.current,
        HOVER_FADE_SECS,
        egui::emath::easing::cubic_out,
    );
    // The background is drawn before the text, so it ends up under it
    let fill = HIGHLIGHT
        .gamma_multiply(hover)
        .lerp_to_gamma(HIGHLIGHT_STRONG, active);
    ui.painter().rect_filled(row_rect, 8, fill);
    // Selection is only a frame of the same color as the playing track highlight: the background stays
    // free, and a selected playing track is still recognizable as playing
    if selected > 0.0 {
        ui.painter().rect_stroke(
            row_rect,
            8,
            egui::Stroke::new(
                SELECTION_STROKE_WIDTH,
                HIGHLIGHT_STRONG.gamma_multiply(selected),
            ),
            egui::StrokeKind::Inside,
        );
    }

    let text_color = if state.missing {
        TEXT_FAINT
    } else {
        TEXT_MUTED.lerp_to_gamma(TEXT, active)
    };
    let content = row_rect.shrink2(vec2(TRACK_ROW_PADDING, 0.0));
    // Like justify-content: space-between: the time is pushed right,
    // the title takes the remaining space on the left
    let mut content_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(content)
            .layout(Layout::right_to_left(Align::Center)),
    );
    let duration = RichText::new(format_secs(track.duration_secs as f32))
        .size(14.0)
        .color(text_color);
    content_ui.add(Label::new(duration).selectable(false));
    content_ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        let title = if track.artist.is_empty() {
            track.title.clone()
        } else {
            format!("{} — {}", track.title, track.artist)
        };
        let title = RichText::new(title).size(14.0).color(text_color);
        ui.add(Label::new(title).selectable(false).truncate());
    });

    row
}
