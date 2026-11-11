//! Search field in the library view header.

use eframe::egui::{
    self, Align, Align2, CornerRadius, FontId, Id, Key, Layout, Margin, Rect, RichText, Sense,
    Stroke, TextEdit, Ui, UiBuilder, pos2, vec2,
};

use super::buttons::{HitShape, icon_button};
use crate::ui::{
    i18n::t,
    style::icons::{paint_clear, paint_search},
    style::theme::{HIGHLIGHT, HIGHLIGHT_STRONG, HOVER_FADE_SECS, TEXT, TEXT_FAINT, TEXT_MUTED},
};

/// Field width; in a narrow area — no more than `SEARCH_MAX_SHARE` of the row width
const SEARCH_WIDTH: f32 = 220.0;
const SEARCH_MAX_SHARE: f32 = 0.5;
/// Field height — slightly more than the title row
const SEARCH_HEIGHT: f32 = 26.0;
/// Minimum gap between the title and the field
const SEARCH_GAP: f32 = 12.0;
const SEARCH_TEXT_SIZE: f32 = 13.0;
const ICON_SIZE: f32 = 14.0;
/// Icon padding from the field edges
const ICON_INSET: f32 = 8.0;
const CLEAR_SIZE: f32 = 14.0;

/// A title row with a search field at the right edge — like `justify-content: space-between`.
/// `title` draws the title in the remaining width on the left (`block_title`, `block_title_with_note`).
/// The row height is the same as a title without search: the field extends beyond the row equally above and below,
/// and nothing shifts when switching to "breadcrumbs".
/// Returns `true` if the query changed
pub fn title_with_search(
    ui: &mut Ui,
    id_salt: &str,
    query: &mut String,
    title: impl FnOnce(&mut Ui),
) -> bool {
    let available = ui.available_rect_before_wrap();
    let field_width = SEARCH_WIDTH.min(available.width() * SEARCH_MAX_SHARE);
    let title_area = Rect::from_min_max(
        available.min,
        pos2(
            available.right() - field_width - SEARCH_GAP,
            available.bottom(),
        ),
    );
    let row = ui
        .scope_builder(UiBuilder::new().max_rect(title_area), title)
        .response
        .rect;
    let field = Rect::from_center_size(
        pos2(available.right() - field_width / 2.0, row.center().y),
        vec2(field_width, SEARCH_HEIGHT),
    );
    search_field(ui, field, id_salt, query)
}

/// Search field in `rect`: a magnifier on the left, a "Search" hint (in the UI language), a clear cross on the right while
/// the query is not empty. Esc clears the query. Returns `true` if the query changed
fn search_field(ui: &mut Ui, rect: Rect, id_salt: &str, query: &mut String) -> bool {
    let text_id = Id::unique(("search", id_salt));
    let radius = CornerRadius::same((rect.height() / 2.0) as u8);

    // A click on the field outside the text (on the magnifier, at the edges) also focuses the field.
    // The background is added before the text — clicks on the text go to the text
    let background = ui.interact(rect, text_id.with("background"), Sense::click());
    if background.clicked() {
        ui.memory_mut(|memory| memory.request_focus(text_id));
    }
    let focused = ui.memory(|memory| memory.has_focus(text_id));
    let hover = ui.ctx().animate_bool_with_time_and_easing(
        text_id.with("hover"),
        background.hovered() || focused,
        HOVER_FADE_SECS,
        egui::emath::easing::cubic_out,
    );
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        radius,
        HIGHLIGHT.lerp_to_gamma(HIGHLIGHT_STRONG, hover),
    );
    if focused {
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, HIGHLIGHT_STRONG),
            egui::StrokeKind::Inside,
        );
    }

    let icon = Rect::from_center_size(
        pos2(rect.left() + ICON_INSET + ICON_SIZE / 2.0, rect.center().y),
        vec2(ICON_SIZE, ICON_SIZE),
    );
    let icon_color = if focused { TEXT_MUTED } else { TEXT_FAINT };
    paint_search(ui, icon, icon_color);

    let clear = Rect::from_center_size(
        pos2(
            rect.right() - ICON_INSET - CLEAR_SIZE / 2.0,
            rect.center().y,
        ),
        vec2(CLEAR_SIZE, CLEAR_SIZE),
    );
    let text_rect = Rect::from_min_max(
        pos2(icon.right() + 6.0, rect.top()),
        pos2(clear.left() - 4.0, rect.bottom()),
    );

    // A separate child Ui: the field takes no space in the title layout
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(text_rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    let response = child.add_sized(
        text_rect.size(),
        TextEdit::singleline(query)
            .id(text_id)
            .frame(egui::Frame::NONE)
            .margin(Margin::ZERO)
            .font(FontId::proportional(SEARCH_TEXT_SIZE))
            .text_color(TEXT)
            .hint_text(RichText::new(t().search).color(TEXT_FAINT))
            .align(Align2::LEFT_CENTER),
    );
    let mut changed = response.changed();

    // Esc removes focus from the field (TextEdit does that itself) — and clears the query, like in macOS
    if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Escape)) && !query.is_empty() {
        query.clear();
        changed = true;
    }

    if !query.is_empty() {
        let button = icon_button(
            ui,
            clear,
            HitShape::Circle,
            &format!("clear {id_salt}"),
            true,
        );
        paint_clear(ui, clear, button.hover);
        if button.clicked {
            query.clear();
            changed = true;
        }
    }
    changed
}
