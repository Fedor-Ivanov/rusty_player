//! Styling of dropdown and context menus.

use eframe::egui::{self, Response, RichText, Ui, vec2};

const MENU_TEXT_SIZE: f32 = 15.0;

/// Menu item styling: larger than the default — bigger font and padding around the text.
/// Call at the start of the menu content, before `menu_item`
pub fn menu_style(ui: &mut Ui) {
    ui.spacing_mut().button_padding = vec2(12.0, 7.0);
    ui.spacing_mut().item_spacing.y = 2.0;
}

pub fn menu_item(ui: &mut Ui, text: &str) -> Response {
    ui.add(egui::Button::new(RichText::new(text).size(MENU_TEXT_SIZE)))
}

/// Submenu: an item with an arrow; on hover a nested menu opens next to it
/// with the same styling
pub fn submenu(ui: &mut Ui, text: &str, add_contents: impl FnOnce(&mut Ui)) {
    ui.menu_button(RichText::new(text).size(MENU_TEXT_SIZE), |ui| {
        menu_style(ui);
        add_contents(ui);
    });
}
