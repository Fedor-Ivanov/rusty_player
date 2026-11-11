//! Adding by drag and drop: files and folders from Finder or Explorer are dropped onto the window.
//! While they're carried over the window, two zones are visible — "to library" (everything left of the queue) and "to queue"
//! (the queue panel); the zone under the cursor is highlighted. What's dropped is added to the same place
//! the "+" button adds to.
//!
//! While files are carried over the window, the system doesn't tell the app where the cursor is: mouse
//! move events don't arrive on either macOS or Windows (winit reports only the files themselves). So
//! the cursor is queried from the system directly (`Cursor`) — every frame while files are over the window.

use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Rect, Stroke, StrokeKind};

use super::{
    action::Action,
    app::PlayerApp,
    i18n::t,
    loading::Destination,
    style::theme::{HIGHLIGHT, HIGHLIGHT_STRONG, HOVER_FADE_SECS, TEXT, TEXT_MUTED},
};

/// Zone inset from the window edges and from each other
const ZONE_INSET: f32 = 12.0;
const ZONE_RADIUS: u8 = 16;
const ZONE_TEXT_SIZE: f32 = 20.0;
/// Dimming the window under the zones — so the labels are readable over any content
const DIM: Color32 = Color32::from_black_alpha(140);

impl PlayerApp {
    /// Zones over the window while files are carried over it, and adding the dropped ones. `queue` — where the queue
    /// panel is: the "to queue" zone is there, left of it — "to library". Call at the end of the frame, on top of everything
    pub(crate) fn render_file_drop(&mut self, ctx: &egui::Context, queue: Rect) {
        let (hovering, dropped) = ctx.input(|input| {
            (
                !input.raw.hovered_files.is_empty(),
                input
                    .raw
                    .dropped_files
                    .iter()
                    .map(|file| file.path().to_path_buf())
                    .collect::<Vec<_>>(),
            )
        });
        let window = ctx.content_rect();
        let zones = [
            (Destination::Library, window.with_max_x(queue.left())),
            (
                Destination::Queue,
                Rect::from_x_y_ranges(queue.x_range(), window.y_range()),
            ),
        ];
        let target = self.drop_cursor.position(ctx).and_then(|pointer| {
            zones
                .iter()
                .find(|(_, zone)| zone.contains(pointer))
                .map(|(destination, _)| *destination)
        });

        if !dropped.is_empty() {
            // Couldn't get the cursor — to the library: from there tracks can be both played and added
            // to the queue
            let destination = target.unwrap_or(Destination::Library);
            self.dispatch(Action::AddPaths {
                paths: dropped,
                destination,
            });
        }

        let visibility = ctx.animate_bool_with_time_and_easing(
            egui::Id::unique("file_drop"),
            hovering,
            HOVER_FADE_SECS,
            egui::emath::easing::cubic_out,
        );
        if visibility == 0.0 {
            return;
        }
        // The cursor position is found by polling — frames are needed even without events
        if hovering {
            ctx.request_repaint();
        }

        let painter = ctx
            .layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::unique("file_drop"),
            ))
            .with_clip_rect(window);
        let fade = |color: Color32| color.gamma_multiply(visibility);
        painter.rect_filled(window, 0, fade(DIM));
        for (destination, zone) in zones {
            let active = target == Some(destination);
            let label = match destination {
                Destination::Queue => t().add_to_queue,
                _ => t().add_to_library,
            };
            let rect = zone.shrink(ZONE_INSET);
            let (fill, stroke, text) = if active {
                (HIGHLIGHT_STRONG, Stroke::new(2.0, TEXT), TEXT)
            } else {
                (HIGHLIGHT, Stroke::new(1.0, TEXT_MUTED), TEXT_MUTED)
            };
            painter.rect(
                rect,
                CornerRadius::same(ZONE_RADIUS),
                fade(fill),
                Stroke::new(stroke.width, fade(stroke.color)),
                StrokeKind::Inside,
            );
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(ZONE_TEXT_SIZE),
                fade(text),
            );
        }
    }
}

/// Cursor in window coordinates — queried from the system. `None` — the system didn't provide the window
/// (or an unsupported platform): then the zone under the cursor is unknown
pub(crate) struct Cursor(Option<platform::Cursor>);

impl Cursor {
    pub(crate) fn new(cc: &eframe::CreationContext) -> Self {
        Self(platform::Cursor::new(cc))
    }

    /// Cursor position — in egui points
    fn position(&self, ctx: &egui::Context) -> Option<egui::Pos2> {
        self.0.as_ref()?.position(ctx)
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use eframe::egui::{self, Pos2, pos2};
    use objc2::rc::Retained;
    use objc2_app_kit::NSView;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    pub struct Cursor {
        view: Retained<NSView>,
    }

    impl Cursor {
        pub fn new(cc: &eframe::CreationContext) -> Option<Self> {
            let handle = cc.window_handle().ok()?;
            let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
                return None;
            };
            // SAFETY: the view belongs to the app window; we hold it with our own reference
            let view = unsafe { Retained::retain(handle.ns_view.as_ptr().cast::<NSView>()) }?;
            Some(Self { view })
        }

        /// Cursor in view points (origin — top left corner), converted to egui points
        pub fn position(&self, ctx: &egui::Context) -> Option<Pos2> {
            let window = self.view.window()?;
            let in_window = window.mouseLocationOutsideOfEventStream();
            let point = self.view.convertPoint_fromView(in_window, None);
            let y = if self.view.isFlipped() {
                point.y
            } else {
                self.view.bounds().size.height - point.y
            };
            let zoom = ctx.zoom_factor();
            Some(pos2(point.x as f32 / zoom, y as f32 / zoom))
        }
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use eframe::egui::{self, Pos2, pos2};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::{
        Foundation::{HWND, POINT},
        Graphics::Gdi::ScreenToClient,
        UI::WindowsAndMessaging::GetCursorPos,
    };

    pub struct Cursor {
        hwnd: HWND,
    }

    impl Cursor {
        pub fn new(cc: &eframe::CreationContext) -> Option<Self> {
            match cc.window_handle().ok()?.as_raw() {
                RawWindowHandle::Win32(handle) => Some(Self {
                    hwnd: handle.hwnd.get() as HWND,
                }),
                _ => None,
            }
        }

        /// Cursor in window client area pixels, converted to egui points
        pub fn position(&self, ctx: &egui::Context) -> Option<Pos2> {
            let mut point = POINT { x: 0, y: 0 };
            // SAFETY: the window is alive for the whole app lifetime, `point` is our variable
            let ok = unsafe {
                GetCursorPos(&mut point) != 0 && ScreenToClient(self.hwnd, &mut point) != 0
            };
            if !ok {
                return None;
            }
            let scale = ctx.pixels_per_point();
            Some(pos2(point.x as f32 / scale, point.y as f32 / scale))
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod platform {
    use eframe::egui::{self, Pos2};

    pub struct Cursor;

    impl Cursor {
        pub fn new(_cc: &eframe::CreationContext) -> Option<Self> {
            None
        }

        pub fn position(&self, _ctx: &egui::Context) -> Option<Pos2> {
            None
        }
    }
}
