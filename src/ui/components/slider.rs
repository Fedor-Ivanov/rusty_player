//! Horizontal slider: seeking in the bottom panel, glass transparency in settings.

use eframe::egui::{CornerRadius, CursorIcon, Rect, Sense, Ui, pos2, vec2};

use crate::ui::style::theme::{SLIDER_TRACK, TEXT};

/// A 0.0..=1.0 value slider of width `width`. Returns the new value while dragged
/// or on click
pub fn slider(
    ui: &mut Ui,
    width: f32,
    value: f32,
    track_height: f32,
    thumb_radius: f32,
) -> Option<f32> {
    let (rect, response) =
        ui.allocate_exact_size(vec2(width, thumb_radius * 2.0), Sense::click_and_drag());
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    // The thumb center moves edge to edge inset by the radius — as it is drawn
    let left = rect.left() + thumb_radius;
    let span = (rect.width() - 2.0 * thumb_radius).max(1.0);
    let changed = if response.dragged() || response.clicked() {
        response
            .interact_pointer_pos()
            .map(|pos| ((pos.x - left) / span).clamp(0.0, 1.0))
    } else {
        None
    };
    paint_slider(
        ui,
        rect,
        changed.unwrap_or(value),
        track_height,
        thumb_radius,
    );
    changed
}

/// Slider in `rect`: a track of height `track_height` across the full width, filled up to
/// `fraction` (0.0..=1.0), and a round thumb of radius `thumb_radius`
pub fn paint_slider(ui: &Ui, rect: Rect, fraction: f32, track_height: f32, thumb_radius: f32) {
    let painter = ui.painter();
    let track = Rect::from_center_size(rect.center(), vec2(rect.width(), track_height));
    painter.rect_filled(track, CornerRadius::same(255), SLIDER_TRACK);

    let thumb_x = track.left() + track.width() * fraction;
    let filled = Rect::from_min_max(track.min, pos2(thumb_x, track.max.y));
    painter.rect_filled(filled, CornerRadius::same(255), TEXT);

    // The thumb doesn't go past the edges. max/min instead of clamp: in a very narrow
    // window the left bound may end up right of the right one, and clamp panics
    let thumb_x = thumb_x
        .min(rect.right() - thumb_radius)
        .max(rect.left() + thumb_radius);
    painter.circle_filled(pos2(thumb_x, rect.center().y), thumb_radius, TEXT);
}
