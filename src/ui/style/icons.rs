//! Player button icons drawn with shapes: play/pause, prev/next, shuffle, repeat, "+",
//! volume.

use super::theme::{HIGHLIGHT_STRONG, ICON, ICON_DISABLED, ICON_OFF, ICON_SOFT};
use eframe::egui::{self, Color32, Pos2, Rect, Shape, Stroke, Ui, pos2, vec2};

/// Outline of a convex polygon inflated by `radius`: the convex hull
/// of circles around the vertices. Coinciding vertices are allowed.
fn rounded_convex(points: &[Pos2], radius: f32) -> Vec<Pos2> {
    const ARC_SEGMENTS: usize = 16;

    let mut samples: Vec<Pos2> = points
        .iter()
        .flat_map(|&center| {
            (0..ARC_SEGMENTS).map(move |i| {
                let angle = i as f32 / ARC_SEGMENTS as f32 * std::f32::consts::TAU;
                center + radius * egui::Vec2::angled(angle)
            })
        })
        .collect();
    samples.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));

    // Convex hull (Andrew's monotone chain): the lower chain, then the upper
    let cross = |o: Pos2, a: Pos2, b: Pos2| (a - o).x * (b - o).y - (a - o).y * (b - o).x;
    let mut hull: Vec<Pos2> = Vec::with_capacity(samples.len() + 1);
    for point in samples.iter().copied() {
        while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], point) <= 0.0 {
            hull.pop();
        }
        hull.push(point);
    }
    let lower_len = hull.len() + 1;
    for point in samples.iter().rev().skip(1).copied() {
        while hull.len() >= lower_len
            && cross(hull[hull.len() - 2], hull[hull.len() - 1], point) <= 0.0
        {
            hull.pop();
        }
        hull.push(point);
    }
    hull.pop();
    hull
}

// Icons in 24×24 coordinates. Each shape is defined by an inner outline and a radius:
// the final shape is the outline inflated by the radius, so the corners come out rounded.
// The play triangle is split vertically into two halves, each a quadrilateral
// whose vertices pairwise morph into the vertices of a pause bar.
const PLAY_TRIANGLE: [(f32, f32); 3] = [(8.0, 6.5), (17.5, 12.0), (8.0, 17.5)];
const PLAY_HALVES: [[(f32, f32); 4]; 2] = [
    [(8.0, 6.5), (12.75, 9.25), (12.75, 14.75), (8.0, 17.5)],
    [(12.75, 9.25), (17.5, 12.0), (17.5, 12.0), (12.75, 14.75)],
];
const PAUSE_BARS: [[(f32, f32); 4]; 2] = [
    [(7.0, 6.0), (9.0, 6.0), (9.0, 18.0), (7.0, 18.0)],
    [(15.0, 6.0), (17.0, 6.0), (17.0, 18.0), (15.0, 18.0)],
];
const PLAY_CORNER_RADIUS: f32 = 1.0;
/// Opacity of the dark circle of the play button over a cover: at rest and on hover
const OVERLAY_PLAY_ALPHA: u8 = 150;
const OVERLAY_PLAY_HOVER_ALPHA: u8 = 200;
const PAUSE_CORNER_RADIUS: f32 = 1.0;
/// Opacity of the white circle under the play icon: at rest and on hover
const PLAY_BG_OPACITY: f32 = 0.15;
const PLAY_BG_HOVER_OPACITY: f32 = 0.25;

// The "next track" icon: two triangles and a thin bar. "Previous" is its mirror
// image. There's a gap between the shapes so the overlap isn't visible on a translucent icon.
const SKIP_TRIANGLES: [[(f32, f32); 3]; 2] = [
    [(3.5, 7.0), (9.5, 12.0), (3.5, 17.0)],
    [(12.0, 7.0), (18.0, 12.0), (12.0, 17.0)],
];
// The bar is a segment inflated by the rounding radius, so its thickness is 2 × radius
const SKIP_BAR: [(f32, f32); 2] = [(20.5, 7.0), (20.5, 17.0)];
const SKIP_CORNER_RADIUS: f32 = 1.0;
/// Margin around the prev/next icon where it still reacts to the cursor, in screen points
const SKIP_HIT_PADDING: f32 = 4.0;

// The "+" icon: two segments inflated by the rounding radius
const ADD_BARS: [[(f32, f32); 2]; 2] = [[(8.0, 12.0), (16.0, 12.0)], [(12.0, 8.0), (12.0, 16.0)]];
const ADD_CORNER_RADIUS: f32 = 1.0;

// Toggle icons (shuffle, repeat) are drawn with lines of ICON_STROKE thickness
// with triangle arrowheads at the ends
const ICON_STROKE: f32 = 2.0;
const ARROW_CORNER_RADIUS: f32 = 0.5;
// Shuffle: two crossed lines, both with a right arrow
const SHUFFLE_LINES: [[(f32, f32); 4]; 2] = [
    [(4.0, 8.0), (8.0, 8.0), (15.0, 16.0), (17.0, 16.0)],
    [(4.0, 16.0), (8.0, 16.0), (15.0, 8.0), (17.0, 8.0)],
];
const SHUFFLE_ARROWS: [[(f32, f32); 3]; 2] = [
    [(17.0, 13.0), (20.0, 16.0), (17.0, 19.0)],
    [(17.0, 5.0), (20.0, 8.0), (17.0, 11.0)],
];
// Repeat: a loop of two brackets — the upper with a right arrow, the lower with a left arrow
const REPEAT_LINES: [[(f32, f32); 3]; 2] = [
    [(5.0, 12.0), (5.0, 8.0), (16.5, 8.0)],
    [(19.0, 12.0), (19.0, 16.0), (7.5, 16.0)],
];
const REPEAT_ARROWS: [[(f32, f32); 3]; 2] = [
    [(16.5, 5.0), (20.0, 8.0), (16.5, 11.0)],
    [(7.5, 13.0), (4.0, 16.0), (7.5, 19.0)],
];
// "1" inside the loop for repeat-one — thinner, to fit
const REPEAT_ONE_MARK: [(f32, f32); 3] = [(10.75, 11.25), (12.0, 10.0), (12.0, 14.0)];
const REPEAT_ONE_STROKE: f32 = 1.5;

// Volume icon: a speaker of two convex shapes (body and cone), to its right —
// arc waves centered at the cone, or a cross if muted
const SPEAKER_SHAPES: [&[(f32, f32)]; 2] = [
    &[(3.5, 10.0), (6.5, 10.0), (6.5, 14.0), (3.5, 14.0)],
    &[(6.0, 10.0), (10.5, 6.0), (10.5, 18.0), (6.0, 14.0)],
];
const SPEAKER_CORNER_RADIUS: f32 = 1.0;
const VOLUME_WAVE_CENTER: (f32, f32) = (11.0, 12.0);
const VOLUME_WAVE_RADII: [f32; 2] = [3.5, 7.0];
/// Half the angle of a wave arc, in radians (±45°)
const VOLUME_WAVE_HALF_ANGLE: f32 = std::f32::consts::FRAC_PI_4;
const VOLUME_WAVE_STROKE: f32 = 1.8;
/// The volume from which the second wave is visible
const VOLUME_LOUD: f32 = 0.5;
const MUTE_CROSS: [[(f32, f32); 2]; 2] = [[(14.5, 9.5), (19.5, 14.5)], [(14.5, 14.5), (19.5, 9.5)]];

/// The background circle of a round button. Returns the color to draw the icon with.
fn paint_button_bg(ui: &Ui, rect: Rect, disabled: bool, hover: f32) -> Color32 {
    let alpha = if disabled { 0.4 } else { 1.0 };
    let bg_opacity = egui::lerp(PLAY_BG_OPACITY..=PLAY_BG_HOVER_OPACITY, hover);
    ui.painter().circle_filled(
        rect.center(),
        rect.width() / 2.0,
        Color32::WHITE.gamma_multiply(bg_opacity * alpha),
    );
    if disabled { ICON_DISABLED } else { ICON }
}

/// Fills a shape defined in 24×24 icon coordinates, with rounding `radius`
fn fill_icon_shape(ui: &Ui, rect: Rect, points: &[Pos2], radius: f32, color: Color32) {
    let scale = rect.width() / 24.0;
    let points: Vec<Pos2> = points
        .iter()
        .map(|p| rect.min + p.to_vec2() * scale)
        .collect();
    let outline = rounded_convex(&points, radius * scale);
    ui.painter()
        .add(Shape::convex_polygon(outline, color, Stroke::NONE));
}

/// `morph`: 0.0 — play, 1.0 — pause, in between — an intermediate shape.
/// `hover`: 0.0 — not hovered, 1.0 — hovered, in between — the transition.
pub fn paint_play_pause(ui: &Ui, rect: Rect, morph: f32, disabled: bool, hover: f32) {
    let icon_color = paint_button_bg(ui, rect, disabled, hover);

    if morph <= 0.0 {
        // As a whole, not as halves: otherwise their overlap is visible on a translucent icon
        let triangle = PLAY_TRIANGLE.map(|(x, y)| pos2(x, y));
        fill_icon_shape(ui, rect, &triangle, PLAY_CORNER_RADIUS, icon_color);
    } else {
        let radius = egui::lerp(PLAY_CORNER_RADIUS..=PAUSE_CORNER_RADIUS, morph);
        for (from, to) in PLAY_HALVES.iter().zip(&PAUSE_BARS) {
            let points: Vec<Pos2> = from
                .iter()
                .zip(to)
                .map(|(&(ax, ay), &(bx, by))| pos2(ax, ay).lerp(pos2(bx, by), morph))
                .collect();
            fill_icon_shape(ui, rect, &points, radius, icon_color);
        }
    }
}

/// The "next track" (`forward`) or "previous" button. No background circle:
/// on hover the icon smoothly lightens.
/// `hover`: 0.0 — not hovered, 1.0 — hovered, in between — the transition.
pub fn paint_skip(ui: &Ui, rect: Rect, forward: bool, disabled: bool, hover: f32) {
    let icon_color = if disabled {
        ICON_DISABLED
    } else {
        ICON_SOFT.lerp_to_gamma(ICON, hover)
    };

    // "Back" is a horizontal mirror of "forward"
    let point = |(x, y): (f32, f32)| {
        if forward {
            pos2(x, y)
        } else {
            pos2(24.0 - x, y)
        }
    };
    for triangle in SKIP_TRIANGLES {
        fill_icon_shape(
            ui,
            rect,
            &triangle.map(point),
            SKIP_CORNER_RADIUS,
            icon_color,
        );
    }
    fill_icon_shape(
        ui,
        rect,
        &SKIP_BAR.map(point),
        SKIP_CORNER_RADIUS,
        icon_color,
    );
}

/// The area where the prev/next button reacts to the cursor: the icon outline
/// with a small margin. The icon is symmetric, so the area is the same for both buttons.
pub fn skip_hit_rect(rect: Rect) -> Rect {
    let scale = rect.width() / 24.0;
    let points = SKIP_TRIANGLES
        .iter()
        .flatten()
        .chain(&SKIP_BAR)
        .map(|&(x, y)| rect.min + vec2(x, y) * scale);
    Rect::from_points(&points.collect::<Vec<_>>())
        .expand(SKIP_CORNER_RADIUS * scale + SKIP_HIT_PADDING)
}

/// A light circle under an icon in the panel: smoothly fades in on hover
fn paint_hover_circle(ui: &Ui, rect: Rect, hover: f32) {
    ui.painter().circle_filled(
        rect.center(),
        rect.width() / 2.0,
        HIGHLIGHT_STRONG.gamma_multiply(hover),
    );
}

/// A polyline in 24×24 icon coordinates with round caps
fn stroke_icon_path(ui: &Ui, rect: Rect, points: &[(f32, f32)], width: f32, color: Color32) {
    let scale = rect.width() / 24.0;
    let points: Vec<Pos2> = points
        .iter()
        .map(|&(x, y)| rect.min + vec2(x, y) * scale)
        .collect();
    let stroke = Stroke::new(width * scale, color);
    // egui draws lines with flat caps — round them with circles
    for end in [points[0], points[points.len() - 1]] {
        ui.painter().circle_filled(end, stroke.width / 2.0, color);
    }
    ui.painter().add(Shape::line(points, stroke));
}

/// Toggle icon color: off — muted, on — light
fn toggle_color(active: f32) -> Color32 {
    ICON_OFF.lerp_to_gamma(ICON, active)
}

/// The "+" button in the library header: a light plus; on hover a light circle smoothly
/// fades in under it.
/// `hover`: 0.0 — not hovered, 1.0 — hovered, in between — the transition.
pub fn paint_add(ui: &Ui, rect: Rect, hover: f32) {
    paint_hover_circle(ui, rect, hover);
    for bar in ADD_BARS {
        let points = bar.map(|(x, y)| pos2(x, y));
        fill_icon_shape(ui, rect, &points, ADD_CORNER_RADIUS, ICON);
    }
}

/// "Lyrics": a speech bubble with a tail at the bottom left and two lines of text in it
const LYRICS_BUBBLE: [(f32, f32); 12] = [
    (4.5, 7.0),
    (7.0, 4.5),
    (17.0, 4.5),
    (19.5, 7.0),
    (19.5, 13.5),
    (17.0, 16.0),
    (11.5, 16.0),
    (7.5, 19.5),
    (7.5, 16.0),
    (7.0, 16.0),
    (4.5, 13.5),
    (4.5, 7.0),
];
const LYRICS_LINES: [[(f32, f32); 2]; 2] =
    [[(8.5, 8.75), (15.5, 8.75)], [(8.5, 11.75), (13.0, 11.75)]];
const LYRICS_STROKE: f32 = 1.7;

/// Lyrics toggle. `active` and `hover`: 0.0..=1.0, smooth on and hover
/// transitions
pub fn paint_lyrics(ui: &Ui, rect: Rect, active: f32, hover: f32) {
    paint_hover_circle(ui, rect, hover);
    let color = toggle_color(active);
    stroke_icon_path(ui, rect, &LYRICS_BUBBLE, LYRICS_STROKE, color);
    for line in LYRICS_LINES {
        stroke_icon_path(ui, rect, &line, LYRICS_STROKE, color);
    }
}

/// Shuffle toggle.
/// `active` and `hover`: 0.0..=1.0, smooth on and hover transitions.
pub fn paint_shuffle(ui: &Ui, rect: Rect, active: f32, hover: f32) {
    paint_hover_circle(ui, rect, hover);
    let color = toggle_color(active);
    for line in SHUFFLE_LINES {
        stroke_icon_path(ui, rect, &line, ICON_STROKE, color);
    }
    for arrow in SHUFFLE_ARROWS {
        let points = arrow.map(|(x, y)| pos2(x, y));
        fill_icon_shape(ui, rect, &points, ARROW_CORNER_RADIUS, color);
    }
}

/// Repeat toggle: the whole list or, with `one`, a single track (with "1" inside the loop).
/// `active` and `hover`: 0.0..=1.0, smooth on and hover transitions.
pub fn paint_repeat(ui: &Ui, rect: Rect, one: bool, active: f32, hover: f32) {
    paint_hover_circle(ui, rect, hover);
    let color = toggle_color(active);
    for line in REPEAT_LINES {
        stroke_icon_path(ui, rect, &line, ICON_STROKE, color);
    }
    for arrow in REPEAT_ARROWS {
        let points = arrow.map(|(x, y)| pos2(x, y));
        fill_icon_shape(ui, rect, &points, ARROW_CORNER_RADIUS, color);
    }
    if one {
        stroke_icon_path(ui, rect, &REPEAT_ONE_MARK, REPEAT_ONE_STROKE, color);
    }
}

/// Play button over a cover: a dark translucent circle with a white triangle — visible
/// on any cover. `visibility` — fade-in together with tile hover (0.0..=1.0),
/// `hover` — hover over the button itself: the circle darkens
pub fn paint_overlay_play(ui: &Ui, rect: Rect, visibility: f32, hover: f32) {
    let circle = Color32::from_black_alpha(OVERLAY_PLAY_ALPHA)
        .lerp_to_gamma(Color32::from_black_alpha(OVERLAY_PLAY_HOVER_ALPHA), hover);
    ui.painter().circle_filled(
        rect.center(),
        rect.width() / 2.0,
        circle.gamma_multiply(visibility),
    );
    let triangle = PLAY_TRIANGLE.map(|(x, y)| pos2(x, y));
    fill_icon_shape(
        ui,
        rect,
        &triangle,
        PLAY_CORNER_RADIUS,
        ICON.gamma_multiply(visibility),
    );
}

/// Magnifier in the search field: a circle and a handle down to the right — the handle is long and slightly thicker than the circle,
/// with a rounded end. Thin lines — the icon sits next to text
pub fn paint_search(ui: &Ui, rect: Rect, color: Color32) {
    const RING_STROKE: f32 = 1.4;
    const HANDLE_STROKE: f32 = 1.8;
    let radius = rect.width() * 0.27;
    // The circle is shifted up and left so the handle fits in the icon square
    let center = rect.center() - vec2(rect.width() * 0.13, rect.height() * 0.13);
    let painter = ui.painter();
    painter.circle_stroke(center, radius, Stroke::new(RING_STROKE, color));

    let direction = vec2(1.0, 1.0).normalized();
    let handle_end = center + direction * rect.width() * 0.68;
    painter.line_segment(
        [center + direction * radius, handle_end],
        Stroke::new(HANDLE_STROKE, color),
    );
    painter.circle_filled(handle_end, HANDLE_STROKE / 2.0, color);
}

/// Search field clear button: a cross in a circle, the circle lightens on hover
pub fn paint_clear(ui: &Ui, rect: Rect, hover: f32) {
    let painter = ui.painter();
    painter.circle_filled(
        rect.center(),
        rect.width() / 2.0,
        ICON_DISABLED.lerp_to_gamma(ICON_OFF, hover),
    );
    let arm = rect.width() * 0.18;
    let stroke = Stroke::new(1.4, Color32::from_rgb(0x1C, 0x1C, 0x1E));
    for (from, to) in [
        (vec2(-arm, -arm), vec2(arm, arm)),
        (vec2(-arm, arm), vec2(arm, -arm)),
    ] {
        painter.line_segment([rect.center() + from, rect.center() + to], stroke);
    }
}

/// Volume button. `volume` — audible volume 0.0..=1.0: at zero (or when muted)
/// — a cross, quieter than `VOLUME_LOUD` — one wave, louder — two.
/// `hover`: 0.0..=1.0, smooth hover transition
pub fn paint_volume(ui: &Ui, rect: Rect, volume: f32, hover: f32) {
    paint_hover_circle(ui, rect, hover);
    for shape in SPEAKER_SHAPES {
        let points: Vec<Pos2> = shape.iter().map(|&(x, y)| pos2(x, y)).collect();
        fill_icon_shape(ui, rect, &points, SPEAKER_CORNER_RADIUS, ICON);
    }

    if volume == 0.0 {
        for line in MUTE_CROSS {
            stroke_icon_path(ui, rect, &line, VOLUME_WAVE_STROKE, ICON);
        }
        return;
    }
    let waves = if volume < VOLUME_LOUD { 1 } else { 2 };
    for radius in &VOLUME_WAVE_RADII[..waves] {
        const SEGMENTS: usize = 12;
        let (cx, cy) = VOLUME_WAVE_CENTER;
        let arc: Vec<(f32, f32)> = (0..=SEGMENTS)
            .map(|i| {
                let t = i as f32 / SEGMENTS as f32;
                let angle = -VOLUME_WAVE_HALF_ANGLE + 2.0 * VOLUME_WAVE_HALF_ANGLE * t;
                (cx + radius * angle.cos(), cy + radius * angle.sin())
            })
            .collect();
        stroke_icon_path(ui, rect, &arc, VOLUME_WAVE_STROKE, ICON);
    }
}

// Queue button icons — lines of QUEUE_ICON_STROKE thickness on a 24×24 grid
const QUEUE_ICON_STROKE: f32 = 1.7;
/// The queue icon drawing is smaller than the button: the hover circle around it is roomier
const QUEUE_ICON_SCALE: f32 = 0.8;
/// "Save as playlist": a floppy disk — a body with a cut corner, a shutter on top
/// and a label at the bottom
const FLOPPY_LINES: [&[(f32, f32)]; 3] = [
    &[
        (4.5, 4.5),
        (16.0, 4.5),
        (19.5, 8.0),
        (19.5, 19.5),
        (4.5, 19.5),
        (4.5, 4.5),
    ],
    &[(8.0, 4.5), (8.0, 9.0), (15.0, 9.0), (15.0, 4.5)],
    &[(7.5, 19.5), (7.5, 13.5), (16.5, 13.5), (16.5, 19.5)],
];
/// "Clear queue": a broom — the handle at the top right, a band across it and a fanned brush
/// down to the left with two bristle lines
const BROOM_LINES: [&[(f32, f32)]; 4] = [
    // Handle
    &[(19.5, 4.5), (13.0, 11.0)],
    // Brush: the band, the fan edges and the bottom
    &[
        (10.5, 9.5),
        (14.5, 13.5),
        (11.0, 19.5),
        (4.5, 13.0),
        (10.5, 9.5),
    ],
    // Bristles
    &[(11.8, 10.8), (6.7, 15.2)],
    &[(13.2, 12.2), (8.8, 17.3)],
];

/// Icon button color: normal or, if the button is unavailable, muted
fn button_color(enabled: bool) -> Color32 {
    if enabled { ICON } else { ICON_DISABLED }
}

/// The "save queue as playlist" button — a floppy disk. `hover`: 0.0..=1.0, smooth
/// hover transition
pub fn paint_save_playlist(ui: &Ui, rect: Rect, enabled: bool, hover: f32) {
    paint_queue_icon(ui, rect, &FLOPPY_LINES, enabled, hover);
}

/// The "clear queue" button — a broom. `hover`: 0.0..=1.0, smooth hover transition
pub fn paint_broom(ui: &Ui, rect: Rect, enabled: bool, hover: f32) {
    paint_queue_icon(ui, rect, &BROOM_LINES, enabled, hover);
}

/// A queue button icon from lines `lines`: the hover circle fills the button, the drawing is
/// scaled down in the center. The line thickness is the same as for a full-button icon
fn paint_queue_icon(ui: &Ui, rect: Rect, lines: &[&[(f32, f32)]], enabled: bool, hover: f32) {
    paint_hover_circle(ui, rect, hover);
    let icon = Rect::from_center_size(rect.center(), rect.size() * QUEUE_ICON_SCALE);
    for line in lines {
        stroke_icon_path(
            ui,
            icon,
            line,
            QUEUE_ICON_STROKE / QUEUE_ICON_SCALE,
            button_color(enabled),
        );
    }
}
