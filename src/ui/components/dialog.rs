//! Modal confirmation dialog.

use eframe::egui::{self, Align, Id, Label, Layout, RichText, Stroke};

use super::buttons::{ButtonStyle, pill_button};
use crate::ui::{
    i18n::t,
    style::theme::{BANNER_BORDER, BANNER_FILL, MODAL_BACKDROP, TEXT, TEXT_MUTED},
};

const DIALOG_WIDTH: f32 = 360.0;

/// Answer to a confirmation dialog
pub enum Confirm {
    /// Not answered yet
    Pending,
    Confirmed,
    /// "Cancel", Esc or a click outside the dialog
    Cancelled,
}

/// A modal window in the app's style: over the dimmed window, with a border like the banners.
/// Esc and a click outside ask to close it — see `ModalResponse::should_close`
pub fn modal(id_salt: &str) -> egui::Modal {
    egui::Modal::new(Id::unique(id_salt))
        .backdrop_color(MODAL_BACKDROP)
        .frame(
            egui::Frame::new()
                .fill(BANNER_FILL)
                .stroke(Stroke::new(1.0, BANNER_BORDER))
                .corner_radius(14)
                .inner_margin(20),
        )
}

/// A modal dialog over the dimmed window: a title, an explanation and two buttons —
/// "Cancel" and the `confirm_label` confirmation (red: it confirms deletion).
/// Show every frame until answered
pub fn confirm_dialog(
    ctx: &egui::Context,
    id_salt: &str,
    title: &str,
    message: &str,
    confirm_label: &str,
) -> Confirm {
    let mut answer = Confirm::Pending;
    let modal = modal(id_salt).show(ctx, |ui| {
        ui.set_width(DIALOG_WIDTH);
        ui.add(Label::new(RichText::new(title).size(17.0).color(TEXT)).selectable(false));
        ui.add_space(6.0);
        ui.add(
            Label::new(RichText::new(message).size(14.0).color(TEXT_MUTED))
                .selectable(false)
                .wrap(),
        );
        ui.add_space(18.0);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            // In right_to_left the rightmost button is added first
            if pill_button(ui, confirm_label, ButtonStyle::Destructive).clicked() {
                answer = Confirm::Confirmed;
            }
            if pill_button(ui, t().cancel, ButtonStyle::Secondary).clicked() {
                answer = Confirm::Cancelled;
            }
        });
    });
    if matches!(answer, Confirm::Pending) && modal.should_close() {
        answer = Confirm::Cancelled;
    }
    answer
}
