//! On/off switch — like in system settings: a capsule with a knob.

use eframe::egui::{CornerRadius, CursorIcon, Sense, Ui, pos2, vec2};

use crate::ui::style::theme::{
    HIGHLIGHT_STRONG, ICON_OFF, PRIMARY_BUTTON_FILL, PRIMARY_BUTTON_TEXT, TRACK_ACTIVE_FADE_SECS,
};

const SIZE: eframe::egui::Vec2 = vec2(36.0, 20.0);
/// Gap between the knob and the capsule edge
const KNOB_INSET: f32 = 2.0;

/// A switch in state `on`. Returns `true` on click — the caller decides whether
/// to toggle. The knob slides smoothly
pub fn switch(ui: &mut Ui, on: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(SIZE, Sense::click());
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    let progress = ui.ctx().animate_bool_with_time_and_easing(
        response.id,
        on,
        TRACK_ACTIVE_FADE_SECS,
        eframe::egui::emath::easing::cubic_in_out,
    );
    let painter = ui.painter();
    // Off — a dark capsule, on — a light one, like the primary button
    let track = HIGHLIGHT_STRONG.lerp_to_gamma(PRIMARY_BUTTON_FILL, progress);
    painter.rect_filled(rect, CornerRadius::same(255), track);
    let radius = rect.height() / 2.0 - KNOB_INSET;
    let left = rect.left() + KNOB_INSET + radius;
    let right = rect.right() - KNOB_INSET - radius;
    let knob = ICON_OFF.lerp_to_gamma(PRIMARY_BUTTON_TEXT, progress);
    painter.circle_filled(
        pos2(left + (right - left) * progress, rect.center().y),
        radius,
        knob,
    );
    response.clicked()
}
