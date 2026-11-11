//! Player keys while the window is focused: space — play/pause, ⌘→ / ⌘← (Ctrl on Windows) —
//! next and previous track, like in Apple Music. Keyboard media keys work
//! without focus too — see `system_media`.

use eframe::egui::{self, Key, Modifiers};

use super::{action::Action, app::PlayerApp};

impl PlayerApp {
    /// Called at the start of the frame, before widgets: space is taken before the focused
    /// button sees it — otherwise it would press that button instead of pausing.
    /// While text is being typed (search, playlist name), keys stay with the field
    pub(crate) fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.text_edit_focused() {
            return;
        }
        let (toggle, next, previous) = ctx.input_mut(|input| {
            (
                input.consume_key(Modifiers::NONE, Key::Space),
                input.consume_key(Modifiers::COMMAND, Key::ArrowRight),
                input.consume_key(Modifiers::COMMAND, Key::ArrowLeft),
            )
        });
        if toggle {
            self.dispatch(Action::TogglePlay);
        }
        if next {
            self.dispatch(Action::Next);
        }
        if previous {
            self.dispatch(Action::Previous);
        }
    }
}
