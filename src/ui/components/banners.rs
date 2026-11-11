//! Banners at the top center of the window: error and track adding progress.

use eframe::egui::{
    self, Align, CornerRadius, CursorIcon, Label, Layout, Rect, RichText, Sense, Stroke, vec2,
};

use crate::ui::{
    i18n::{fill, t},
    style::theme::{BANNER_BORDER, BANNER_FILL, RED, SLIDER_TRACK, TEXT, TEXT_MUTED},
};

const ERROR_BANNER_MAX_WIDTH: f32 = 440.0;
const ERROR_BANNER_BUTTON_WIDTH: f32 = 48.0;
/// Banners at the top of the window (error, progress): offset from the top edge and the gap between them
pub const BANNER_TOP: f32 = 16.0;
pub const BANNER_GAP: f32 = 8.0;
const LOADING_BANNER_WIDTH: f32 = 300.0;

/// An error banner at the top center, above the whole window, with a red border: text,
/// below it on the right — an "OK" button.
/// All app errors are shown only this way. Returns `true` when "OK" is pressed.
/// `top` — offset from the top of the window: below the progress banner if it's visible.
pub fn error_banner(ctx: &egui::Context, message: &str, top: f32) -> bool {
    let mut dismissed = false;
    egui::Area::new(egui::Id::unique("error_banner"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_TOP, vec2(0.0, top))
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(BANNER_FILL)
                .stroke(Stroke::new(1.5, RED))
                .corner_radius(12)
                .inner_margin(egui::Margin::symmetric(16, 12))
                .show(ui, |ui| {
                    ui.set_max_width(ERROR_BANNER_MAX_WIDTH);
                    // Text on top, wrapped into lines
                    ui.add(Label::new(RichText::new(message).color(TEXT)).wrap());
                    ui.add_space(8.0);

                    // "OK" below the text, at the right edge. The row width follows the text, not
                    // the banner's maximum: a short error gives a narrow banner
                    let ok = egui::Button::new(RichText::new(t().ok).color(TEXT))
                        .corner_radius(8)
                        .min_size(vec2(ERROR_BANNER_BUTTON_WIDTH, 0.0));
                    let width = ui.min_rect().width().max(ERROR_BANNER_BUTTON_WIDTH);
                    ui.scope_builder(
                        egui::UiBuilder::new()
                            .max_rect(Rect::from_min_size(
                                ui.cursor().min,
                                vec2(width, ui.spacing().interact_size.y),
                            ))
                            .layout(Layout::right_to_left(Align::Min)),
                        |ui| {
                            dismissed = ui
                                .add(ok)
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked();
                        },
                    );
                });
        });
    dismissed
}

/// Track adding progress banner at the top center, above the whole window — where
/// errors are. `progress` — how many processed and how many total; `None` while the folder walk
/// is running and the total is unknown. Returns whether "Cancel" was pressed, and the banner's bottom edge —
/// so the error banner is placed below it rather than on top.
/// `template` — progress text with `{done}` and `{total}`: "Adding {done} of {total}".
pub fn loading_banner(
    ctx: &egui::Context,
    template: &str,
    progress: Option<(usize, usize)>,
) -> (bool, f32) {
    let mut cancel = false;
    let response = egui::Area::new(egui::Id::unique("loading_banner"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_TOP, vec2(0.0, BANNER_TOP))
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(BANNER_FILL)
                .stroke(Stroke::new(1.0, BANNER_BORDER))
                .corner_radius(12)
                .inner_margin(egui::Margin::symmetric(16, 12))
                .show(ui, |ui| {
                    // Fixed width: otherwise the banner would jitter as the digits change
                    ui.set_width(LOADING_BANNER_WIDTH);
                    ui.horizontal(|ui| {
                        let text = match progress {
                            Some((done, total)) => fill(
                                template,
                                &[("done", &done.to_string()), ("total", &total.to_string())],
                            ),
                            None => {
                                ui.add(egui::Spinner::new().size(12.0).color(TEXT_MUTED));
                                t().scanning_folder.to_owned()
                            }
                        };
                        ui.add(
                            Label::new(RichText::new(text).size(13.0).color(TEXT))
                                .selectable(false),
                        );

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let button = egui::Button::new(
                                RichText::new(t().cancel).size(13.0).color(TEXT_MUTED),
                            )
                            .frame(false);
                            cancel = ui
                                .add(button)
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked();
                        });
                    });

                    // Progress bar — in the style of the player slider. While the total is unknown,
                    // there's none: a spinner in the row above instead
                    if let Some((done, total)) = progress {
                        ui.add_space(8.0);
                        let (rect, _) =
                            ui.allocate_exact_size(vec2(ui.available_width(), 4.0), Sense::hover());
                        let fraction = if total == 0 {
                            1.0
                        } else {
                            done as f32 / total as f32
                        };
                        let filled = Rect::from_min_size(
                            rect.min,
                            vec2(rect.width() * fraction, rect.height()),
                        );
                        ui.painter()
                            .rect_filled(rect, CornerRadius::same(255), SLIDER_TRACK);
                        ui.painter()
                            .rect_filled(filled, CornerRadius::same(255), TEXT);
                    }
                });
        })
        .response;
    (cancel, response.rect.bottom())
}
