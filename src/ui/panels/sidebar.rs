//! Left panel: the library on top — a title with a "+" button and sections (Artists, Albums,
//! Songs), the settings button at the bottom. The selected section opens in the central area.

use eframe::egui::{self, Align, Ui};

use crate::ui::{
    action::Action,
    app::PlayerApp,
    components,
    i18n::t,
    loading::Destination,
    navigation::{Route, Section},
    style::theme::TEXT,
};

impl PlayerApp {
    /// The whole left panel
    pub(crate) fn render_sidebar(&mut self, ui: &mut Ui) {
        self.render_library_nav(ui);
        egui::Panel::bottom("library_sidebar_bottom")
            .frame(egui::Frame::new().inner_margin(16.0))
            .resizable(false)
            .show_separator_line(false)
            .show(ui, |ui| self.render_settings_button(ui));
    }

    /// The "Settings" button — a title of the same kind as "Library" and "Queue": it's a separate
    /// section, not another library section, and it shouldn't blend in with its items
    fn render_settings_button(&mut self, ui: &mut Ui) {
        if components::title_button(ui, t().settings).clicked() {
            self.settings_open = true;
        }
    }

    /// Library: a title with "+" and sections
    fn render_library_nav(&mut self, ui: &mut Ui) {
        egui::Frame::new().inner_margin(16.0).show(ui, |ui| {
            let row = components::block_title(ui, t().library, Align::LEFT, TEXT);
            self.render_add_button(ui, row, Destination::Library);

            ui.add_space(8.0);
            ui.spacing_mut().item_spacing.y = 2.0;
            for section in Section::LIBRARY {
                let selected = self.route.section() == section;
                if components::nav_item(ui, section.title(), section.title(), selected).clicked() {
                    // A menu item always opens the whole section: from an artist profile
                    // or album — back to the top level
                    self.dispatch(Action::Navigate(Route::from(section)));
                }
            }
        });
    }
}
