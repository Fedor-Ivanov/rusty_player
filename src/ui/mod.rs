//! UI built with egui.
//!
//! - `app` — `PlayerApp`: window state and panel layout;
//! - `action` — user actions on data and the only place where they are performed;
//! - `navigation` — what's open in the center (`Route`);
//! - `panels` — always on screen: library, queue, player;
//! - `pages` — central area pages;
//! - `dialogs` — modal windows;
//! - `components` — reusable elements, know nothing about the player;
//! - `style` — colors, icons, window glass; `i18n` — translations;
//! - `loading` — adding files; `covers` — cover cache;
//! - `open_with` — files opened with the app from the system.

mod action;
mod app;
mod components;
mod covers;
mod dialogs;
mod file_drop;
mod i18n;
mod loading;
mod navigation;
pub mod open_with;
mod pages;
mod panels;
mod shortcuts;
pub mod style;
mod system_media;

pub use app::PlayerApp;
