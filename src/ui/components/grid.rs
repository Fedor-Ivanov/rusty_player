//! A tile grid, like CSS `grid-template-columns: repeat(auto-fill, minmax(…, 1fr))`:
//! the column count adapts to the width, tiles stretch across the whole row.
//! A tile is a square image with a few lines of text under it. Virtualized,
//! like the lists: only visible grid rows are drawn. Selection works like in the track list:
//! ⌘/Shift+click, mouse marquee, ⌘A, Esc, Delete keys.

use eframe::egui::{
    self, Align, Color32, CursorIcon, FontId, Key, Rect, Response, RichText, Sense, Stroke,
    StrokeKind, Ui, pos2, vec2,
};
use std::{collections::BTreeSet, ops::Range};

use super::selection::{Marquee, Selection};
use crate::ui::style::theme::{
    HIGHLIGHT_STRONG, HOVER_FADE_SECS, MARQUEE_FILL, MARQUEE_STROKE, SELECTION_STROKE_WIDTH, TEXT,
    TEXT_MUTED,
};

/// A tile is no narrower than this: there are as many columns as such tiles fit
const TILE_MIN_WIDTH: f32 = 140.0;
const TILE_GAP: f32 = 18.0;
/// How many rows above and below the visible ones count as nearby — their images are prepared in advance
const NEARBY_ROWS: usize = 2;
/// Top padding: the selection frame extends beyond the tile and would otherwise be clipped
/// by the scroll edge on the first row
const TOP_MARGIN: f32 = 8.0;
const SIDE_MARGIN: f32 = 16.0;
/// Gap between the image and the text and between text lines
const TEXT_TOP_GAP: f32 = 8.0;
const LINE_GAP: f32 = 2.0;
const PRIMARY_SIZE: f32 = 14.0;
const SECONDARY_SIZE: f32 = 12.0;
/// Selected tile frame: offset from the tile (so it doesn't lie on the cover's edge)
/// and rounding
const HIGHLIGHT_PADDING: f32 = 6.0;
const HIGHLIGHT_RADIUS: u8 = 14;
/// Auto-scroll speed during marquee selection — same as the track list
const MARQUEE_AUTOSCROLL_SPEED: f32 = 8.0;

/// The tile being drawn now
pub struct Tile {
    /// Smooth hover 0.0..=1.0 — by the cursor position over the tile: stays even
    /// when the cursor is over a button on top of the tile
    pub hover: f32,
    /// Response of the whole tile (image with caption) — for the context menu
    pub response: Response,
    /// How many tiles are selected — the right-click menu acts on all of them
    pub selected: usize,
}

/// A line of text under the tile image
pub struct TileLine {
    pub text: String,
    /// The first line (title) is larger and brighter than the rest
    pub primary: bool,
}

/// What happened to the grid during the frame — the caller decides
pub struct TileGridResponse {
    /// A plain click on a tile (without ⌘/Shift) — open it
    pub opened: Option<usize>,
    /// Delete or Backspace was pressed while the grid is active and something is selected
    pub delete: bool,
    /// The grid was used with the mouse — it becomes active for the keyboard
    pub interacted: bool,
}

/// State of one grid between frames: selection and marquee selection
pub struct TileGridView {
    /// Distinguishes grids: each has its own scroll and background ids
    id: &'static str,
    selection: Selection,
    marquee: Option<Marquee>,
    /// Which content the selection belongs to (see `set_scope`)
    scope: String,
    /// Tiles of visible rows and their neighbors — as of the last `show` (see `nearby`)
    nearby: Range<usize>,
}

impl TileGridView {
    pub fn new(id: &'static str) -> Self {
        Self {
            id,
            selection: Selection::default(),
            marquee: None,
            scope: String::new(),
            nearby: 0..0,
        }
    }

    /// Selection is tile indices. If the grid content changed (another artist,
    /// the library changed), the same indices mean other tiles — the selection is reset.
    /// `scope` describes the content; call every frame before `show`
    pub fn set_scope(&mut self, scope: &str) {
        if self.scope != scope {
            self.selection.clear();
            self.marquee = None;
            self.scope = scope.to_owned();
        }
    }

    pub fn selection(&self) -> &BTreeSet<usize> {
        self.selection.indices()
    }

    /// Tiles on screen and in `NEARBY_ROWS` rows above and below — as of the last `show`. Their
    /// images are worth requesting in advance: they'll be ready when scrolling
    pub fn nearby(&self) -> Range<usize> {
        self.nearby.clone()
    }

    /// Draws a grid of `count` tiles with `line_count` lines of text each. `text_align` —
    /// text alignment under the image: `Align::Min` — left, `Align::Center` —
    /// centered. `keyboard` — whether the grid is active for keys (⌘A, Esc, Delete).
    /// `tile` draws the image of tile `index` in square `image_rect` (it may add its own buttons
    /// and menus on top via [`Tile`]) and returns the text lines under it.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        count: usize,
        line_count: usize,
        text_align: Align,
        keyboard: bool,
        mut tile: impl FnMut(&mut Ui, usize, Rect, &Tile) -> Vec<TileLine>,
    ) -> TileGridResponse {
        let mut response = TileGridResponse {
            opened: None,
            delete: false,
            interacted: false,
        };
        if count == 0 {
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
                self.selection.select_all(count);
            }
            if deselect {
                self.selection.clear();
            }
        }

        let width = (ui.available_width() - 2.0 * SIDE_MARGIN).max(TILE_MIN_WIDTH);
        let columns = (((width + TILE_GAP) / (TILE_MIN_WIDTH + TILE_GAP)).floor() as usize).max(1);
        let tile_width = (width - TILE_GAP * (columns - 1) as f32) / columns as f32;
        // The grid row height is fixed (image + all text lines) — without this the grid
        // can't be virtualized, nor can the tile index under the cursor be computed
        let row_height = tile_width + TEXT_TOP_GAP + text_height(ui, line_count);
        let rows = count.div_ceil(columns);
        let modifiers = ui.input(|i| i.modifiers);
        let mut nearby = 0..0;

        ui.spacing_mut().item_spacing.y = TILE_GAP;
        egui::ScrollArea::vertical()
            .id_salt(self.id)
            .content_margin(egui::Margin {
                left: SIDE_MARGIN as i8,
                right: SIDE_MARGIN as i8,
                top: TOP_MARGIN as i8,
                bottom: SIDE_MARGIN as i8,
            })
            .show_rows(ui, row_height, rows, |ui, visible_rows| {
                let first_row = visible_rows.start.saturating_sub(NEARBY_ROWS);
                let end_row = (visible_rows.end + NEARBY_ROWS).min(rows);
                nearby = first_row * columns..(end_row * columns).min(count);
                // Top and left edge of the whole grid content on screen: show_rows yields
                // only the visible chunk, starting at row visible_rows.start. Tiles under the frame
                // are computed from the grid dimensions, not from drawn widgets — tiles
                // off screen don't exist, but they still need to be selectable by the frame
                let row_stride = row_height + TILE_GAP;
                let content_top = ui.max_rect().top() - visible_rows.start as f32 * row_stride;
                let content_left = ui.max_rect().left();
                let tile_rect = |row: usize, column: usize| {
                    Rect::from_min_size(
                        pos2(
                            content_left + column as f32 * (tile_width + TILE_GAP),
                            content_top + row as f32 * row_stride,
                        ),
                        vec2(tile_width, row_height),
                    )
                };

                // Background under the tiles — for marquee selection and clicks on empty space.
                // Added before the tiles: the tiles end up on top and get clicks,
                // while dragging goes to the background (tiles don't catch it)
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

                for row in visible_rows {
                    ui.allocate_exact_size(vec2(ui.available_width(), row_height), Sense::hover());
                    for column in 0..columns {
                        let index = row * columns + column;
                        if index >= count {
                            break;
                        }
                        let rect = tile_rect(row, column);
                        let image_rect =
                            Rect::from_min_size(rect.min, vec2(tile_width, tile_width));

                        // The whole tile — the image together with the caption. Added before
                        // the content: buttons the tile draws on top of the image
                        // get clicks first
                        let tile_id = ui.scope_id().with((self.id, index));
                        let tile_response = ui
                            .interact(rect, tile_id, Sense::click())
                            .on_hover_cursor(CursorIcon::PointingHand);

                        // A click without modifiers opens the tile; ⌘/Shift+click selects,
                        // as in the list. A right click on an unselected tile selects it —
                        // the menu acts on the selection, like in Finder
                        if tile_response.clicked() {
                            if modifiers.command || modifiers.shift {
                                self.selection.click(index, modifiers);
                            } else {
                                self.selection.clear();
                                response.opened = Some(index);
                            }
                            response.interacted = true;
                        }
                        if tile_response.secondary_clicked() {
                            if !self.selection.contains(index) {
                                self.selection.click(index, egui::Modifiers::NONE);
                            }
                            response.interacted = true;
                        }

                        let hover = ui.ctx().animate_bool_with_time_and_easing(
                            tile_id.with("hover"),
                            tile_response.contains_pointer(),
                            HOVER_FADE_SECS,
                            egui::emath::easing::cubic_out,
                        );
                        // A selected tile and a tile with an open menu get a frame,
                        // like a selected track in the list
                        let highlighted = ui.ctx().animate_bool_with_time_and_easing(
                            tile_id.with("highlighted"),
                            self.selection.contains(index) || tile_response.context_menu_opened(),
                            HOVER_FADE_SECS,
                            egui::emath::easing::cubic_out,
                        );

                        let lines = tile(
                            ui,
                            index,
                            image_rect,
                            &Tile {
                                hover,
                                response: tile_response,
                                selected: self.selection.len().max(1),
                            },
                        );
                        paint_lines(
                            ui,
                            &lines,
                            pos2(rect.left(), image_rect.bottom() + TEXT_TOP_GAP),
                            tile_width,
                            text_align,
                        );
                        if highlighted > 0.0 {
                            ui.painter().rect_stroke(
                                rect.expand(HIGHLIGHT_PADDING),
                                HIGHLIGHT_RADIUS,
                                Stroke::new(
                                    SELECTION_STROKE_WIDTH,
                                    HIGHLIGHT_STRONG.gamma_multiply(highlighted),
                                ),
                                StrokeKind::Inside,
                            );
                        }
                    }
                }

                // The frame — after the tiles, so it lies on top of them
                if let Some(marquee) = &self.marquee
                    && background.dragged()
                    && let Some(pointer) = background.interact_pointer_pos()
                {
                    let start = pos2(marquee.start.x, content_top + marquee.start.y);
                    let area = Rect::from_two_pos(start, pointer);

                    // Tiles touched by the frame: rows — by the grid height, in each —
                    // columns whose rectangle intersects the frame
                    let row_at = |y: f32| {
                        ((((y - content_top) / row_stride).floor()).max(0.0) as usize).min(rows - 1)
                    };
                    let touched: Vec<usize> = (row_at(area.top())..=row_at(area.bottom()))
                        .flat_map(|row| (0..columns).map(move |column| (row, column)))
                        .filter(|&(row, column)| row * columns + column < count)
                        .filter(|&(row, column)| tile_rect(row, column).intersects(area))
                        .map(|(row, column)| row * columns + column)
                        .collect();
                    let anchor = touched.first().copied().unwrap_or(0);
                    self.selection.select_many(&marquee.base, touched, anchor);

                    ui.painter().rect(
                        area,
                        4,
                        MARQUEE_FILL,
                        Stroke::new(SELECTION_STROKE_WIDTH, MARQUEE_STROKE),
                        StrokeKind::Inside,
                    );

                    // The cursor went above or below the grid — scroll there, the faster
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

        self.nearby = nearby;
        response
    }
}

/// Height of the text under the image: the first line is larger, the rest smaller
fn text_height(ui: &Ui, line_count: usize) -> f32 {
    let line_height = |size: f32| {
        ui.painter()
            .layout_no_wrap("Ag".into(), FontId::proportional(size), TEXT)
            .size()
            .y
    };
    (0..line_count)
        .map(|line| {
            line_height(if line == 0 {
                PRIMARY_SIZE
            } else {
                SECONDARY_SIZE
            })
        })
        .sum::<f32>()
        + LINE_GAP * line_count.saturating_sub(1) as f32
}

/// Lines under the image, truncated with "…" to the tile width
fn paint_lines(ui: &Ui, lines: &[TileLine], mut pos: egui::Pos2, width: f32, align: Align) {
    for line in lines {
        let (size, color): (f32, Color32) = if line.primary {
            (PRIMARY_SIZE, TEXT)
        } else {
            (SECONDARY_SIZE, TEXT_MUTED)
        };
        let galley = egui::WidgetText::from(RichText::new(&line.text).size(size).color(color))
            .into_galley(
                ui,
                Some(egui::TextWrapMode::Truncate),
                width,
                FontId::proportional(size),
            );
        let size = galley.size();
        // Free horizontal space — on the left, split evenly on both sides, or on the right
        let x = pos.x + (width - size.x).max(0.0) * align.to_factor();
        ui.painter().galley(pos2(x, pos.y), galley, color);
        pos.y += size.y + LINE_GAP;
    }
}
