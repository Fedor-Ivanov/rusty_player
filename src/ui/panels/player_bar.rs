//! Bottom panel: volume, prev / play-pause / next, time and the seek slider.

use eframe::egui::{
    self, Align2, CornerRadius, CursorIcon, FontId, Id, Order, Pos2, Rangef, Rect, Sense, Stroke,
    StrokeKind, Ui, Vec2, pos2, vec2,
};

use crate::{
    core::playback::PlayerState,
    ui::{
        action::Action,
        app::PlayerApp,
        components,
        components::format_secs,
        style::icons::{paint_play_pause, paint_skip, paint_volume, skip_hit_rect},
        style::theme::{
            BANNER_BORDER, BANNER_FILL, HOVER_FADE_SECS, SLIDER_TRACK, TEXT, TEXT_MUTED,
        },
    },
};

const PLAY_BUTTON_SIZE: f32 = 60.0;
const PLAY_MORPH_SECS: f32 = 0.25;
const SKIP_BUTTON_SIZE: f32 = 40.0;
// Gap between the play button and the prev/next buttons
const SKIP_BUTTON_GAP: f32 = 4.0;
const SLIDER_TRACK_HEIGHT: f32 = 5.0;
const SLIDER_THUMB_RADIUS: f32 = 8.0;
const VOLUME_BUTTON_SIZE: f32 = 24.0;
/// Vertical block with the volume slider above the button: size, gap to the button,
/// rounding, top and bottom padding
const VOLUME_POPUP_SIZE: Vec2 = vec2(36.0, 140.0);
const VOLUME_POPUP_GAP: f32 = 6.0;
const VOLUME_POPUP_RADIUS: u8 = 12;
const VOLUME_POPUP_PADDING: f32 = 14.0;
const VOLUME_TRACK_WIDTH: f32 = 4.0;
const VOLUME_THUMB_RADIUS: f32 = 6.0;
/// Wheel scrolling over the volume: how many scroll points for the full scale
const VOLUME_SCROLL_RANGE: f32 = 400.0;

impl PlayerApp {
    pub(crate) fn render_controls(&mut self, ui: &mut Ui) {
        // Row above the time: volume on the right
        let (row, _) = ui.allocate_exact_size(
            vec2(ui.available_width(), VOLUME_BUTTON_SIZE),
            Sense::hover(),
        );
        self.render_volume(ui, row);

        let player = self.core.controller.player();
        let has_track = player.current_track.is_some();
        let playing = player.state == PlayerState::Playing;
        let duration = player
            .current_track
            .as_ref()
            .map_or(0.0, |t| t.duration_secs as f32);

        let position = self
            .drag_secs
            .unwrap_or(self.core.controller.position_ms() as f32 / 1000.0)
            .min(duration);

        // Row: time on the left, button in the center, duration on the right
        let (row, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), PLAY_BUTTON_SIZE), Sense::hover());

        if has_track && duration > 0.0 {
            let font = FontId::proportional(16.0);
            let painter = ui.painter();
            painter.text(
                row.left_center(),
                Align2::LEFT_CENTER,
                format_secs(position),
                font.clone(),
                TEXT_MUTED,
            );
            // On the right — the time remaining. Round up: format_secs drops the fractional
            // part, and otherwise elapsed + remaining would be a second less than the duration
            let remaining = (duration - position).max(0.0).ceil();
            painter.text(
                row.right_center(),
                Align2::RIGHT_CENTER,
                format_secs(remaining),
                font,
                TEXT_MUTED,
            );
        }

        // prev and next sit on either side of play
        let play_rect =
            Rect::from_center_size(row.center(), vec2(PLAY_BUTTON_SIZE, PLAY_BUTTON_SIZE));
        let skip_offset = vec2(
            (PLAY_BUTTON_SIZE + SKIP_BUTTON_SIZE) / 2.0 + SKIP_BUTTON_GAP,
            0.0,
        );
        let skip_size = vec2(SKIP_BUTTON_SIZE, SKIP_BUTTON_SIZE);
        let prev_rect = Rect::from_center_size(row.center() - skip_offset, skip_size);
        let next_rect = Rect::from_center_size(row.center() + skip_offset, skip_size);

        // prev/next react only to the icon outline, not to the whole square around it
        let prev = components::icon_button(
            ui,
            skip_hit_rect(prev_rect),
            components::HitShape::Rect,
            "prev",
            has_track,
        );
        paint_skip(ui, prev_rect, false, !has_track, prev.hover);

        // play has a clear circle, so hover is also counted only inside it
        let play = components::icon_button(
            ui,
            play_rect,
            components::HitShape::Circle,
            "play_pause",
            has_track,
        );
        // 0.0 — the play triangle, 1.0 — the pause bars
        let morph = ui.ctx().animate_bool_with_time_and_easing(
            play.response.id,
            playing,
            PLAY_MORPH_SECS,
            egui::emath::easing::cubic_in_out,
        );
        paint_play_pause(ui, play_rect, morph, !has_track, play.hover);

        let next = components::icon_button(
            ui,
            skip_hit_rect(next_rect),
            components::HitShape::Rect,
            "next",
            has_track,
        );
        paint_skip(ui, next_rect, true, !has_track, next.hover);

        if prev.clicked {
            self.dispatch(Action::Previous);
        }
        if play.clicked {
            self.dispatch(Action::TogglePlay);
        }
        if next.clicked {
            self.dispatch(Action::Next);
        }

        ui.add_space(1.0);
        self.render_progress_bar(ui, position, duration, has_track && duration > 0.0);
    }

    pub(crate) fn render_progress_bar(
        &mut self,
        ui: &mut Ui,
        position: f32,
        duration: f32,
        enabled: bool,
    ) {
        let sense = if enabled {
            Sense::click_and_drag()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), sense);

        let value_at =
            |pos: Pos2| ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0) * duration;

        if enabled {
            let response = response.on_hover_cursor(CursorIcon::PointingHand);

            if response.dragged()
                && let Some(pos) = response.interact_pointer_pos()
            {
                self.drag_secs = Some(value_at(pos));
            }

            if response.drag_stopped() || response.clicked() {
                // On release the cursor may be unavailable already — take the last drag position
                let secs = response
                    .interact_pointer_pos()
                    .map(value_at)
                    .or(self.drag_secs);
                self.drag_secs = None;

                if let Some(secs) = secs {
                    self.dispatch(Action::Seek(secs));
                }
            }
        }

        let fraction = if duration > 0.0 {
            (self.drag_secs.unwrap_or(position) / duration).clamp(0.0, 1.0)
        } else {
            0.0
        };

        components::paint_slider(ui, rect, fraction, SLIDER_TRACK_HEIGHT, SLIDER_THUMB_RADIUS);
    }

    /// Volume: a button at the right edge of row `row`, above the remaining time. Click — mute.
    /// On hover a block with a slider appears above the button; it doesn't hide while the cursor
    /// is on the button, on the block or between them, and while the slider is dragged
    fn render_volume(&mut self, ui: &mut Ui, row: Rect) {
        let button_rect = Rect::from_min_size(
            pos2(row.right() - VOLUME_BUTTON_SIZE, row.top()),
            vec2(VOLUME_BUTTON_SIZE, VOLUME_BUTTON_SIZE),
        );
        let button = components::icon_button(
            ui,
            button_rect,
            components::HitShape::Circle,
            "volume",
            true,
        );
        let button_hovered = button.response.hovered();
        if button.clicked {
            self.dispatch(Action::ToggleMute);
        }
        paint_volume(
            ui,
            button_rect,
            self.core.controller.volume().audible(),
            button.hover,
        );

        // The block is centered above the button, but within the window
        let window = ui.ctx().content_rect();
        let mut popup = Rect::from_center_size(
            pos2(
                button_rect.center().x,
                button_rect.top() - VOLUME_POPUP_GAP - VOLUME_POPUP_SIZE.y / 2.0,
            ),
            VOLUME_POPUP_SIZE,
        );
        popup = popup.translate(vec2((window.right() - popup.right()).min(0.0), 0.0));

        let popup_id = Id::unique("volume_popup");
        let slider_id = popup_id.with("slider");
        let was_open = ui.data(|data| {
            data.get_temp::<bool>(popup_id.with("open"))
                .unwrap_or(false)
        });
        // A rectangle covering the button and the block: the cursor goes from the button to the block through the gap
        let in_zone = ui
            .ctx()
            .pointer_hover_pos()
            .is_some_and(|pos| button_rect.union(popup).expand(4.0).contains(pos));
        let open = button_hovered || (was_open && in_zone) || ui.ctx().is_being_dragged(slider_id);
        ui.data_mut(|data| data.insert_temp(popup_id.with("open"), open));
        let visibility = ui.ctx().animate_bool_with_time_and_easing(
            popup_id.with("fade"),
            open,
            HOVER_FADE_SECS,
            egui::emath::easing::cubic_out,
        );
        if visibility == 0.0 {
            return;
        }

        // The mouse wheel over the button or the block also controls volume
        if open && in_zone {
            let scroll = ui.input(|input| input.smooth_scroll_delta.y);
            if scroll != 0.0 {
                let level = self.core.controller.volume().audible() + scroll / VOLUME_SCROLL_RANGE;
                self.dispatch(Action::SetVolume(level));
            }
        }

        // The block is above the central area: it's taller than the bottom panel
        let mut level = None;
        let audible = self.core.controller.volume().audible();
        egui::Area::new(popup_id)
            .order(Order::Foreground)
            .fixed_pos(popup.min)
            .constrain(false)
            .show(ui.ctx(), |ui| {
                ui.multiply_opacity(visibility);
                let (rect, _) = ui.allocate_exact_size(popup.size(), Sense::hover());
                ui.painter().rect(
                    rect,
                    VOLUME_POPUP_RADIUS,
                    BANNER_FILL,
                    Stroke::new(1.0, BANNER_BORDER),
                    StrokeKind::Inside,
                );

                // It can be dragged across the full block width, not just the thin track
                let slider = rect.shrink2(vec2(0.0, VOLUME_POPUP_PADDING));
                let response = ui
                    .interact(slider, slider_id, Sense::click_and_drag())
                    .on_hover_cursor(CursorIcon::PointingHand);
                let scale = vertical_scale(slider, VOLUME_THUMB_RADIUS);
                if (response.dragged() || response.clicked())
                    && let Some(pos) = response.interact_pointer_pos()
                {
                    // Bottom to top: silence at the bottom, maximum at the top
                    level = Some(((scale.max - pos.y) / scale.span()).clamp(0.0, 1.0));
                }
                paint_vertical_slider(ui, slider, level.unwrap_or(audible));
            });
        if let Some(level) = level {
            self.dispatch(Action::SetVolume(level));
        }
    }
}

/// Where the vertical slider thumb center moves in `rect`: from the bottom (0.0) to
/// the top (1.0), inset by the thumb radius — it doesn't go past the edges
fn vertical_scale(rect: Rect, thumb_radius: f32) -> Rangef {
    Rangef::new(rect.top() + thumb_radius, rect.bottom() - thumb_radius)
}

/// Vertical volume slider in `rect`: a centered track, filled from the bottom up to
/// `fraction` (0.0..=1.0), and a round thumb
fn paint_vertical_slider(ui: &Ui, rect: Rect, fraction: f32) {
    let painter = ui.painter();
    let track = Rect::from_center_size(rect.center(), vec2(VOLUME_TRACK_WIDTH, rect.height()));
    painter.rect_filled(track, CornerRadius::same(255), SLIDER_TRACK);

    let scale = vertical_scale(rect, VOLUME_THUMB_RADIUS);
    let thumb_y = scale.max - scale.span() * fraction;
    let filled = Rect::from_min_max(pos2(track.left(), thumb_y), track.max);
    painter.rect_filled(filled, CornerRadius::same(255), TEXT);
    painter.circle_filled(pos2(rect.center().x, thumb_y), VOLUME_THUMB_RADIUS, TEXT);
}
