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
//! - `loading` — adding files; `covers` — cover cache.

mod action;
mod app;
mod components;
mod covers;
mod dialogs;
mod file_drop;
mod i18n;
mod loading;
mod navigation;
mod pages;
mod panels;
mod shortcuts;
pub mod style;
mod system_media;

pub use app::PlayerApp;
