//! Block titles and lines at panel borders.

use eframe::egui::{
    self, Align, Color32, CursorIcon, FontId, Galley, Rangef, Rect, Response, RichText, Sense,
    Stroke, Ui, pos2, vec2,
};
use std::sync::Arc;

use crate::ui::style::theme::{HOVER_FADE_SECS, RESIZE_LINE, TEXT, TEXT_FAINT, TEXT_MUTED};

const BLOCK_TITLE_SIZE: f32 = 16.0;
/// A note next to the title
const NOTE_SIZE: f32 = 13.0;
const RESIZE_LINE_WIDTH: f32 = 1.0;

/// Block (panel) title aligned by `align`: `Align::Min` — left,
/// `Align::Center` — center, `Align::Max` — right.
/// Returns the full-width row rectangle — buttons can be placed next to the title
/// using it.
pub fn block_title(ui: &mut Ui, text: &str, align: Align, color: Color32) -> Rect {
    let row = title_row(ui);
    let galley = title_galley(ui, text, color, row.width());
    let x = row.left() + (row.width() - galley.size().x).max(0.0) * align.to_factor();
    paint_in_row(ui, row, x, galley, color);
    row
}

/// A title button: looks exactly like a left-aligned `block_title` ("Library", "Queue"),
/// but can be clicked — the cursor over the text becomes a hand
pub fn title_button(ui: &mut Ui, text: &str) -> Response {
    let row = title_row(ui);
    let galley = title_galley(ui, text, TEXT, row.width());
    let rect = Rect::from_min_size(row.min, vec2(galley.size().x, row.height()));
    let response = ui
        .interact(
            rect,
            ui.scope_id().with(("title_button", text)),
            Sense::click(),
        )
        .on_hover_cursor(CursorIcon::PointingHand);
    paint_in_row(ui, row, row.left(), galley, TEXT);
    response
}

/// A left-aligned block title with `note` next to it — in small muted text on the same line
/// (e.g. how many tracks are in the list). The row is the same as `block_title`'s
pub fn block_title_with_note(ui: &mut Ui, text: &str, note: &str) -> Rect {
    // Roughly the width of a space: "Queue · 12 tracks" reads as one phrase
    const GAP: f32 = 5.0;
    let row = title_row(ui);
    let title = title_galley(ui, text, TEXT, row.width());
    let title_width = title.size().x;
    paint_in_row(ui, row, row.left(), title, TEXT);

    let note = egui::WidgetText::from(RichText::new(note).size(NOTE_SIZE).color(TEXT_FAINT))
        .into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            (row.width() - title_width - GAP).max(0.0),
            FontId::proportional(NOTE_SIZE),
        );
    paint_in_row(ui, row, row.left() + title_width + GAP, note, TEXT_FAINT);
    row
}

/// Side panel resize line — along the full height of the panel border, always visible
pub fn paint_resize_line(ui: &Ui, edge_x: f32, y_range: Rangef) {
    ui.painter()
        .vline(edge_x, y_range, Stroke::new(RESIZE_LINE_WIDTH, RESIZE_LINE));
}

/// A "breadcrumbs" title: `parents › … › current`. Each of `parents` is a link
/// to its level, smoothly lightening on hover. Returns the index of the clicked
/// link. The row has the same height and the text sits on the same line as `block_title`'s:
/// nothing shifts when switching between a plain title and breadcrumbs
pub fn breadcrumb_title(ui: &mut Ui, parents: &[&str], current: &str) -> Option<usize> {
    const GAP: f32 = 6.0;
    let row = title_row(ui);
    let mut clicked = None;
    let mut x = row.left();

    for (index, parent) in parents.iter().enumerate() {
        // Link: its space is allocated before drawing — the color depends on hover
        let width = title_galley(ui, parent, TEXT, f32::INFINITY).size().x;
        let rect = Rect::from_min_size(pos2(x, row.top()), vec2(width, row.height()));
        let response = ui
            .interact(
                rect,
                ui.scope_id().with(("breadcrumb", index)),
                Sense::click(),
            )
            .on_hover_cursor(CursorIcon::PointingHand);
        if response.clicked() {
            clicked = Some(index);
        }
        let hover = ui.ctx().animate_bool_with_time_and_easing(
            response.id.with("hover"),
            response.hovered(),
            HOVER_FADE_SECS,
            egui::emath::easing::cubic_out,
        );
        let color = TEXT_MUTED.lerp_to_gamma(TEXT, hover);
        paint_in_row(
            ui,
            row,
            x,
            title_galley(ui, parent, color, f32::INFINITY),
            color,
        );
        x += width + GAP;

        let separator = title_galley(ui, "›", TEXT_FAINT, f32::INFINITY);
        let separator_width = separator.size().x;
        paint_in_row(ui, row, x, separator, TEXT_FAINT);
        x += separator_width + GAP;
    }

    // The current level — truncated with "…" if it doesn't fit
    let current = title_galley(ui, current, TEXT, (row.right() - x).max(0.0));
    paint_in_row(ui, row, x, current, TEXT);

    clicked
}

/// Full-width title row. The height is the title font's row height, the same for
/// all title kinds
fn title_row(ui: &mut Ui) -> Rect {
    let height = title_galley(ui, "Ag", TEXT, f32::INFINITY).size().y;
    ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover())
        .0
}

/// Title text truncated with "…" to `max_width`
fn title_galley(ui: &Ui, text: &str, color: Color32, max_width: f32) -> Arc<Galley> {
    egui::WidgetText::from(RichText::new(text).size(BLOCK_TITLE_SIZE).color(color)).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        max_width,
        FontId::proportional(BLOCK_TITLE_SIZE),
    )
}

/// Text in the title row from left edge `x`, vertically centered in the row
fn paint_in_row(ui: &Ui, row: Rect, x: f32, galley: Arc<Galley>, color: Color32) {
    let y = row.center().y - galley.size().y / 2.0;
    ui.painter().galley(pos2(x, y), galley, color);
}

/// A line of text looking like `text_link`, but not clickable: same size and color,
/// no wider than the available space — long text is truncated with "…". For places where the same
/// line is sometimes a link and sometimes not — so the look doesn't change
pub fn text_line(ui: &mut Ui, text: &str, size: f32, color: Color32) -> Response {
    let galley = egui::WidgetText::from(RichText::new(text).size(size).color(color)).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        ui.available_width(),
        FontId::proportional(size),
    );
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
    ui.painter().galley(rect.min, galley, color);
    response
}

/// A text link: color `color` at rest, smoothly lightens on hover, the cursor is a hand.
/// No wider than the available space: long text is truncated with "…"
pub fn text_link(ui: &mut Ui, text: &str, size: f32, color: Color32) -> Response {
    let galley = |ui: &Ui, color: Color32, max_width: f32| {
        egui::WidgetText::from(RichText::new(text).size(size).color(color)).into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            max_width,
            FontId::proportional(size),
        )
    };
    // Space is allocated before drawing — the color depends on hover
    let link_size = galley(ui, color, ui.available_width()).size();
    let (rect, response) = ui.allocate_exact_size(link_size, Sense::click());
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    let hover = ui.ctx().animate_bool_with_time_and_easing(
        response.id.with("hover"),
        response.hovered(),
        HOVER_FADE_SECS,
        egui::emath::easing::cubic_out,
    );
    let color = color.lerp_to_gamma(TEXT, hover);
    ui.painter()
        .galley(rect.min, galley(ui, color, rect.width()), color);
    response
}
