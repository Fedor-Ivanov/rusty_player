use eframe::egui::Color32;

use crate::core::settings::{Glass, GlassTint};

// Glassmorphism: dark glass — the system blur under a transparent window
// and a graphite tint, one for the whole window.
// White with alpha is specified premultiplied: (a, a, a, a).

/// Primary text and icons
pub const TEXT: Color32 = Color32::from_rgb(0xF5, 0xF5, 0xF7);
/// Secondary text: artist, time, inactive tracks
pub const TEXT_MUTED: Color32 = Color32::from_rgba_premultiplied(180, 180, 180, 180);
/// Hints and placeholders
pub const TEXT_FAINT: Color32 = Color32::from_rgba_premultiplied(110, 110, 110, 110);

// Icons are drawn from overlapping shapes, so their colors are opaque:
// with alpha, overlaps would show as denser spots
pub const ICON: Color32 = TEXT;
/// An icon at rest, lightens to ICON on hover
pub const ICON_SOFT: Color32 = Color32::from_rgb(0xC8, 0xC8, 0xCE);
/// A switched-off toggle
pub const ICON_OFF: Color32 = Color32::from_rgb(0x8A, 0x8A, 0x92);
/// An unavailable button
pub const ICON_DISABLED: Color32 = Color32::from_rgb(0x5C, 0x5C, 0x63);

/// Tint over the system blur — the only glass fill, one for the whole window.
/// The color follows the tint from settings; the higher the alpha, the darker it is and the less is visible
/// behind the window. Transparency 0.0 — alpha 255, 1.0 — `MIN_TINT_ALPHA`
pub fn window_tint(glass: Glass) -> Color32 {
    let [r, g, b] = tint_rgb(glass.tint);
    let alpha = 255.0 - glass.transparency.clamp(0.0, 1.0) * (255.0 - MIN_TINT_ALPHA);
    Color32::from_rgba_unmultiplied(r, g, b, alpha.round() as u8)
}

/// The most transparent tint: beyond it, light text on a light background behind the window
/// becomes unreadable
const MIN_TINT_ALPHA: f32 = 105.0;

/// A tint circle in settings: the same color, but lighter — the tint itself is dark,
/// and circles of different tints on dark glass would be almost indistinguishable
pub fn tint_swatch(tint: GlassTint) -> Color32 {
    let [r, g, b] = tint_rgb(tint).map(|channel| (channel as f32 * 2.6).min(255.0) as u8);
    Color32::from_rgb(r, g, b)
}

/// Glass tint colors are dark: light text sits on the tint
fn tint_rgb(tint: GlassTint) -> [u8; 3] {
    match tint {
        GlassTint::Graphite => [24, 24, 26],
        GlassTint::Blue => [16, 28, 56],
        GlassTint::Violet => [36, 22, 56],
        GlassTint::Green => [16, 40, 30],
        GlassTint::Rose => [54, 20, 34],
        GlassTint::Amber => [52, 36, 14],
    }
}

/// Panel resize line at the panel border: muted white (~18%),
/// so it separates sections without catching the eye
pub const RESIZE_LINE: Color32 = Color32::from_rgba_premultiplied(45, 45, 45, 45);

/// Hover highlight
pub const HIGHLIGHT: Color32 = Color32::from_rgba_premultiplied(18, 18, 18, 18);
/// Active element highlight, round button background
pub const HIGHLIGHT_STRONG: Color32 = Color32::from_rgba_premultiplied(40, 40, 40, 40);

/// Selection frame thickness: a selected row, the marquee frame, a tile
/// with an open context menu. The frame color is HIGHLIGHT_STRONG
pub const SELECTION_STROKE_WIDTH: f32 = 1.5;

/// Mouse marquee frame — matching the selected rows: a stroke of the same color
/// as the selection frame and the playing track highlight, the fill — like the hover highlight
pub const MARQUEE_FILL: Color32 = HIGHLIGHT;
pub const MARQUEE_STROKE: Color32 = HIGHLIGHT_STRONG;

/// Primary action button ("Play"): light, with dark text — more prominent than the rest
pub const PRIMARY_BUTTON_FILL: Color32 = TEXT;
pub const PRIMARY_BUTTON_TEXT: Color32 = Color32::from_rgb(0x16, 0x16, 0x18);
pub const PRIMARY_BUTTON_HOVER: Color32 = Color32::from_rgb(0xD6, 0xD6, 0xDA);
/// Delete button: red, darker on hover
pub const DESTRUCTIVE_BUTTON_FILL: Color32 = Color32::from_rgb(0xD9, 0x3F, 0x3F);
pub const DESTRUCTIVE_BUTTON_HOVER: Color32 = Color32::from_rgb(0xB8, 0x32, 0x32);
/// Window dimming under a modal dialog
pub const MODAL_BACKDROP: Color32 = Color32::from_black_alpha(120);

// Current track details backdrop — its blurred cover: cover opacity (the window glass
// stays visible under it) and darkening on top for text readability — stronger at the top,
// where the text is, weaker at the bottom
pub const TRACK_BACKDROP_OPACITY: f32 = 0.55;
pub const TRACK_BACKDROP_SHADE_TOP: Color32 = Color32::from_black_alpha(150);
pub const TRACK_BACKDROP_SHADE_BOTTOM: Color32 = Color32::from_black_alpha(70);
/// Current track cover and backdrop transition: the new one fades in over the old one
pub const COVER_CROSSFADE_SECS: f32 = 0.25;
/// Secondary button at rest — HIGHLIGHT_STRONG, on hover — lighter
pub const SECONDARY_BUTTON_HOVER: Color32 = Color32::from_rgba_premultiplied(66, 66, 66, 66);

/// The unfilled part of the slider
pub const SLIDER_TRACK: Color32 = Color32::from_rgba_premultiplied(55, 55, 55, 55);

/// Error banner border
pub const RED: Color32 = Color32::from_rgb(0xFF, 0x6B, 0x6B);
/// Background of banners at the top of the window (error, adding progress): almost opaque —
/// they lie over the content and must be readable
pub const BANNER_FILL: Color32 = Color32::from_rgba_unmultiplied_const(28, 28, 30, 245);
/// Progress banner border — neutral, unlike the red one of the error
pub const BANNER_BORDER: Color32 = Color32::from_rgba_premultiplied(45, 45, 45, 45);

// Duration of smooth transitions: hover and active element change
pub const HOVER_FADE_SECS: f32 = 0.15;
pub const TRACK_ACTIVE_FADE_SECS: f32 = 0.2;
