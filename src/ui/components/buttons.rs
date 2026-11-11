//! Icon buttons with an exact hit area and smooth hover.

use eframe::egui::{self, Align2, CursorIcon, FontId, Pos2, Rect, Response, Sense, Ui, vec2};

use crate::ui::style::theme::{
    DESTRUCTIVE_BUTTON_FILL, DESTRUCTIVE_BUTTON_HOVER, HIGHLIGHT_STRONG, HOVER_FADE_SECS,
    PRIMARY_BUTTON_FILL, PRIMARY_BUTTON_HOVER, PRIMARY_BUTTON_TEXT, SECONDARY_BUTTON_HOVER, TEXT,
    TRACK_ACTIVE_FADE_SECS,
};

const PILL_HEIGHT: f32 = 32.0;
const PILL_PADDING: f32 = 18.0;
const PILL_TEXT_SIZE: f32 = 14.0;

/// Shape of the area where the button reacts to the cursor
pub enum HitShape {
    Rect,
    /// A circle inscribed in the button rectangle
    Circle,
}

pub struct IconButton {
    pub response: Response,
    pub clicked: bool,
    /// Smooth hover for drawing: 0.0 — not hovered, 1.0 — hovered
    pub hover: f32,
}

/// Icon button: click, hand cursor and smooth hover — only inside `shape`
pub fn icon_button(
    ui: &mut Ui,
    rect: Rect,
    shape: HitShape,
    id_salt: &str,
    enabled: bool,
) -> IconButton {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let response = ui.interact(rect, ui.scope_id().with(id_salt), sense);

    // egui hit-tests by rectangle, we refine the circle ourselves
    let inside = |pos: Pos2| match shape {
        HitShape::Rect => true,
        HitShape::Circle => pos.distance(rect.center()) <= rect.width() / 2.0,
    };
    let hovered = enabled && response.hover_pos().is_some_and(inside);
    let clicked = response.clicked() && response.interact_pointer_pos().is_some_and(inside);

    if hovered {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let hover = ui.ctx().animate_bool_with_time_and_easing(
        response.id.with("hover"),
        hovered,
        HOVER_FADE_SECS,
        egui::emath::easing::cubic_out,
    );

    IconButton {
        response,
        clicked,
        hover,
    }
}

/// Toggle button in the list panel. Returns the click and smooth (0.0..=1.0)
/// on and hover values for drawing the icon.
pub fn toggle_button(
    ui: &mut Ui,
    rect: Rect,
    id_salt: &str,
    on: bool,
    tooltip: &str,
) -> (bool, f32, f32) {
    let button = icon_button(ui, rect, HitShape::Circle, id_salt, true);
    let active = ui.ctx().animate_bool_with_time_and_easing(
        button.response.id.with("active"),
        on,
        TRACK_ACTIVE_FADE_SECS,
        egui::emath::easing::cubic_in_out,
    );
    let clicked = button.clicked;
    button.response.on_hover_text(tooltip);
    (clicked, active, button.hover)
}

/// "Pill" button style
#[derive(Clone, Copy)]
pub enum ButtonStyle {
    /// Primary action: light fill and dark text
    Primary,
    /// Translucent
    Secondary,
    /// An action that deletes something: red
    Destructive,
}

/// A "pill" button with text. Smoothly changes brightness on hover
pub fn pill_button(ui: &mut Ui, label: &str, style: ButtonStyle) -> Response {
    let font = FontId::proportional(PILL_TEXT_SIZE);
    let text_width = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), TEXT)
        .size()
        .x;
    let (rect, response) = ui.allocate_exact_size(
        vec2(text_width + 2.0 * PILL_PADDING, PILL_HEIGHT),
        Sense::click(),
    );
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    let hover = ui.ctx().animate_bool_with_time_and_easing(
        response.id.with("hover"),
        response.hovered(),
        HOVER_FADE_SECS,
        egui::emath::easing::cubic_out,
    );
    let (fill, text) = match style {
        // A light button darkens slightly on hover
        ButtonStyle::Primary => (
            PRIMARY_BUTTON_FILL.lerp_to_gamma(PRIMARY_BUTTON_HOVER, hover),
            PRIMARY_BUTTON_TEXT,
        ),
        // Translucent — lightens on hover
        ButtonStyle::Secondary => (
            HIGHLIGHT_STRONG.lerp_to_gamma(SECONDARY_BUTTON_HOVER, hover),
            TEXT,
        ),
        ButtonStyle::Destructive => (
            DESTRUCTIVE_BUTTON_FILL.lerp_to_gamma(DESTRUCTIVE_BUTTON_HOVER, hover),
            TEXT,
        ),
    };
    ui.painter().rect_filled(rect, PILL_HEIGHT / 2.0, fill);
    ui.painter()
        .text(rect.center(), Align2::CENTER_CENTER, label, font, text);
    response
}
