//! Album cover: drawing for the whole UI — grid tiles and the current track cover
//! with a smooth transition (`CoverFade`), a backdrop from the blurred cover. The covers themselves come from the cache
//! (`ui::covers`).

use eframe::egui::{
    self, Color32, CornerRadius, Mesh, Painter, Rect, Shape, Stroke, TextureHandle, Ui,
    epaint::Vertex, pos2, vec2,
};

use crate::ui::{
    covers::Cover,
    style::theme::{
        COVER_CROSSFADE_SECS, HIGHLIGHT, TEXT_FAINT, TRACK_BACKDROP_OPACITY,
        TRACK_BACKDROP_SHADE_BOTTOM, TRACK_BACKDROP_SHADE_TOP,
    },
};

/// Cover in square `rect`: when ready — center-cropped to a square (like CSS
/// `object-fit: cover`); while being read — an empty square (a record would flicker);
/// if the tags have no cover — a vinyl record placeholder
pub fn paint_cover(ui: &Ui, rect: Rect, cover: Cover<'_>, corner_radius: u8) {
    match cover {
        Cover::Ready(texture, appear) => {
            // Just became ready — fades in over the "loading" underlay
            if appear < 1.0 {
                ui.painter()
                    .rect_filled(rect, CornerRadius::same(corner_radius), HIGHLIGHT);
            }
            paint_square_image(ui, texture, rect, corner_radius, appear);
        }
        Cover::Missing => paint_placeholder(ui.painter(), rect, corner_radius),
        Cover::Loading => {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(corner_radius), HIGHLIGHT);
        }
    }
}

/// An image cropped to a square, with opacity `opacity`
fn paint_square_image(
    ui: &Ui,
    texture: &TextureHandle,
    rect: Rect,
    corner_radius: u8,
    opacity: f32,
) {
    egui::Image::new((texture.id(), rect.size()))
        .uv(cover_uv(texture, rect))
        .corner_radius(corner_radius)
        .tint(Color32::WHITE.gamma_multiply(opacity))
        .paint_at(ui, rect);
}

/// What's visible in place of the cover
#[derive(Clone)]
enum Shown {
    Image(TextureHandle),
    /// No cover in tags — a placeholder (and for the backdrop — just glass)
    Missing,
}

impl Shown {
    fn same(&self, other: &Shown) -> bool {
        match (self, other) {
            (Shown::Image(a), Shown::Image(b)) => a.id() == b.id(),
            (Shown::Missing, Shown::Missing) => true,
            _ => false,
        }
    }

    fn texture(&self) -> Option<&TextureHandle> {
        match self {
            Shown::Image(texture) => Some(texture),
            Shown::Missing => None,
        }
    }
}

/// Smooth cover transition — for the current track, where the image changes with the track.
/// While the new cover is being read, the old one is visible: when switching tracks the image doesn't vanish
/// for a single frame. Once ready — fades in over the old one over `COVER_CROSSFADE_SECS`; tracks
/// of the same album share the cover, and the transition isn't visible at all. Keeps the textures itself: the old one
/// won't vanish even if the cover cache has already evicted it
#[derive(Default)]
pub struct CoverFade {
    current: Option<Shown>,
    previous: Option<Shown>,
    /// When the transition started — egui time in seconds
    changed_at: f64,
}

impl CoverFade {
    /// Takes into account the cover cache's answer for this frame and returns how far the new cover
    /// has faded in: 0.0..=1.0. "Still loading" changes nothing
    pub fn update(&mut self, ui: &Ui, cover: Cover<'_>) -> f32 {
        let now = ui.input(|input| input.time);
        let next = match cover {
            // It fades in track changes itself: it doesn't need the cache's own fade-in
            Cover::Ready(texture, _) => Some(Shown::Image(texture.clone())),
            Cover::Missing => Some(Shown::Missing),
            Cover::Loading => None,
        };
        if let Some(next) = next
            && !self
                .current
                .as_ref()
                .is_some_and(|current| current.same(&next))
        {
            self.previous = self.current.replace(next);
            self.changed_at = now;
        }

        let t = ((now - self.changed_at) as f32 / COVER_CROSSFADE_SECS).clamp(0.0, 1.0);
        if t < 1.0 {
            ui.ctx().request_repaint();
        } else {
            self.previous = None;
        }
        // Ease in and out
        t * t * (3.0 - 2.0 * t)
    }

    /// Cover in square `rect`: the old one, with the new one fading in over it (`progress` — from
    /// `update`). While nothing has been read yet — an empty square, like `paint_cover`
    pub fn paint(&self, ui: &Ui, rect: Rect, progress: f32, corner_radius: u8) {
        match &self.previous {
            Some(previous) => paint_shown(ui, rect, previous, 1.0, corner_radius),
            None => {
                ui.painter()
                    .rect_filled(rect, CornerRadius::same(corner_radius), HIGHLIGHT);
            }
        }
        if let Some(current) = &self.current {
            paint_shown(ui, rect, current, progress, corner_radius);
        }
    }

    /// A backdrop from the blurred cover (`CoverSize::Backdrop`) over all of `rect`: center-cropped
    /// to `rect`'s proportions, translucent — the window glass shows through — and darkened
    /// with a gradient so light text on top is readable on any cover. From `fade_from` to
    /// the bottom of `rect` it smoothly dissolves into the glass. When the cover changes, the old backdrop dissolves,
    /// the new one fades in (`progress` — from `update`). Returns a shape rather than drawing: the backdrop
    /// is placed under what's already drawn. No cover — `None`, just the glass is visible
    pub fn backdrop(&self, rect: Rect, fade_from: f32, progress: f32) -> Option<Shape> {
        let rows = backdrop_rows(rect, fade_from);
        let previous = self.previous.as_ref().and_then(Shown::texture);
        let current = self.current.as_ref().and_then(Shown::texture);
        // The backdrop is translucent: if we just fade out the old one and fade in the new one, in the middle
        // of the transition two translucent layers let more glass through, and the backdrop "blinks".
        // So the new layer fades in exactly enough that together with the fading
        // old one the backdrop density doesn't change — only the color changes
        let previous_visibility = 1.0 - progress;
        let current_visibility = if previous.is_some() {
            let below = TRACK_BACKDROP_OPACITY * previous_visibility;
            (1.0 - (1.0 - TRACK_BACKDROP_OPACITY) / (1.0 - below)) / TRACK_BACKDROP_OPACITY
        } else {
            progress
        };
        let layers = [
            (previous, previous_visibility),
            (current, current_visibility),
        ];
        let mut shapes = Vec::new();
        // One darkening for both layers: visible as much as any backdrop is visible
        let mut shade = 0.0;
        for (texture, visibility) in layers {
            if let Some(texture) = texture {
                shapes.push(backdrop_image(rect, &rows, texture, visibility).into());
                shade += visibility;
            }
        }
        if shapes.is_empty() {
            return None;
        }
        shapes.push(backdrop_shade(rect, fade_from, &rows, shade.min(1.0)).into());
        Some(Shape::Vec(shapes))
    }
}

/// Cover with opacity `opacity`: the image or a vinyl record placeholder
fn paint_shown(ui: &Ui, rect: Rect, shown: &Shown, opacity: f32, corner_radius: u8) {
    match shown {
        Shown::Image(texture) => {
            egui::Image::new((texture.id(), rect.size()))
                .uv(cover_uv(texture, rect))
                .corner_radius(corner_radius)
                .tint(Color32::WHITE.gamma_multiply(opacity))
                .paint_at(ui, rect);
        }
        Shown::Missing => {
            let mut painter = ui.painter().clone();
            painter.multiply_opacity(opacity);
            paint_placeholder(&painter, rect, corner_radius);
        }
    }
}

/// Backdrop vertex rows from top to bottom: height and share of full visibility. Up to `fade_from` the backdrop is
/// fully visible, then down to the bottom of `rect` it dissolves along a smooth curve: opacity between
/// rows changes linearly, there are enough rows for the curve to look smooth
fn backdrop_rows(rect: Rect, fade_from: f32) -> Vec<(f32, f32)> {
    const FADE_STEPS: usize = 12;
    let fade_from = fade_from.clamp(rect.top(), rect.bottom());
    std::iter::once((rect.top(), 1.0))
        .chain((0..=FADE_STEPS).map(|step| {
            let t = step as f32 / FADE_STEPS as f32;
            let smooth = t * t * (3.0 - 2.0 * t);
            (egui::lerp(fade_from..=rect.bottom(), t), 1.0 - smooth)
        }))
        .collect()
}

/// Blurred backdrop cover with visibility `visibility`
fn backdrop_image(
    rect: Rect,
    rows: &[(f32, f32)],
    texture: &TextureHandle,
    visibility: f32,
) -> Mesh {
    let uv = cover_uv(texture, rect);
    let mut mesh = Mesh::with_texture(texture.id());
    for &(y, alpha) in rows {
        let v = egui::lerp(uv.y_range(), (y - rect.top()) / rect.height());
        let tint = Color32::WHITE.gamma_multiply(TRACK_BACKDROP_OPACITY * visibility * alpha);
        for (x, u) in [(rect.left(), uv.left()), (rect.right(), uv.right())] {
            mesh.vertices.push(Vertex {
                pos: pos2(x, y),
                uv: pos2(u, v),
                color: tint,
            });
        }
    }
    connect_rows(&mut mesh);
    mesh
}

/// Backdrop darkening for text readability: from the top to `fade_from` — a gradient from
/// strong to weak, then it dissolves together with the backdrop
fn backdrop_shade(rect: Rect, fade_from: f32, rows: &[(f32, f32)], visibility: f32) -> Mesh {
    let mut mesh = Mesh::default();
    for &(y, alpha) in rows {
        let t = ((y - rect.top()) / (fade_from - rect.top()).max(1.0)).min(1.0);
        let color = TRACK_BACKDROP_SHADE_TOP
            .lerp_to_gamma(TRACK_BACKDROP_SHADE_BOTTOM, t)
            .gamma_multiply(visibility * alpha);
        mesh.colored_vertex(pos2(rect.left(), y), color);
        mesh.colored_vertex(pos2(rect.right(), y), color);
    }
    connect_rows(&mut mesh);
    mesh
}

/// Vertices come in pairs (left, right) per row — two triangles between adjacent rows
fn connect_rows(mesh: &mut Mesh) {
    let rows = mesh.vertices.len() as u32 / 2;
    for row in 0..rows.saturating_sub(1) {
        let i = row * 2;
        mesh.add_triangle(i, i + 1, i + 3);
        mesh.add_triangle(i, i + 3, i + 2);
    }
}

/// The part of the texture that fills `rect` without distortion: center crop to the proportions
/// of `rect` (like CSS `object-fit: cover`)
fn cover_uv(texture: &TextureHandle, rect: Rect) -> Rect {
    // How many times the image is wider than the area (by proportions)
    let ratio = texture.aspect_ratio() / (rect.width() / rect.height().max(1.0));
    let uv_size = if ratio > 1.0 {
        vec2(1.0 / ratio, 1.0)
    } else {
        vec2(1.0, ratio)
    };
    Rect::from_center_size(pos2(0.5, 0.5), uv_size)
}

/// Cover placeholder: a square with a vinyl record
fn paint_placeholder(painter: &Painter, rect: Rect, corner_radius: u8) {
    painter.rect_filled(rect, CornerRadius::same(corner_radius), HIGHLIGHT);

    // Sizes are fractions of the square's side
    let center = rect.center();
    let r = |fraction: f32| rect.width().min(rect.height()) * fraction;
    let disc = Color32::from_rgb(0x12, 0x12, 0x14);
    let groove = Stroke::new(1.0, TEXT_FAINT.gamma_multiply(0.3));
    painter.circle_filled(center, r(0.42), disc);
    painter.circle_stroke(center, r(0.42), groove);
    for fraction in [0.37, 0.32, 0.27, 0.22] {
        painter.circle_stroke(center, r(fraction), groove);
    }
    painter.circle_filled(center, r(0.14), TEXT_FAINT);
    painter.circle_filled(center, r(0.025).max(2.0), disc);
}
