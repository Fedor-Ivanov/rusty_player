// In a Windows release build, don't open a console window next to the app.
// In a debug build the console stays — errors are written to it
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod core;
mod ui;

use eframe::egui;

/// App name: window title and the name of the data (library) folder
pub const APP_NAME: &str = "LMP";

fn main() -> eframe::Result {
    // The icon is embedded into the binary at build time, no separate file is needed.
    // On macOS — a squircle following Apple's template, otherwise — a vinyl record on a transparent background
    #[cfg(target_os = "macos")]
    const ICON_PNG: &[u8] = include_bytes!("../assets/images/icon_macos.png");
    #[cfg(not(target_os = "macos"))]
    const ICON_PNG: &[u8] = include_bytes!("../assets/images/icon_windows.png");
    let icon = eframe::icon_data::from_png_bytes(ICON_PNG).expect("app icon is not a valid PNG");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            // Initial widths: library 240 + center + track list 300 — the center gets ~440
            .with_inner_size([980.0, 620.0])
            // Minimums: side panels 180 each + central area 260 = 620, plus a margin
            .with_min_inner_size([640.0, 400.0])
            .with_icon(icon)
            // Transparent window: the system draws a real blur under it (see window_glass.rs)
            .with_transparent(true),
        renderer: ui::style::window_glass::renderer(),
        ..Default::default()
    };

    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_theme(egui::Theme::Dark);
            if let Err(e) = ui::style::window_glass::apply(cc) {
                eprintln!("Could not enable the window blur: {e}");
            }
            Ok(Box::new(ui::PlayerApp::new(cc)))
        }),
    )
}
