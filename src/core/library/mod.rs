//! Files on disk: track description, tags, audio file discovery, background adding,
//! the library — the track list kept between launches.
//! Knows nothing about playback or the UI.

pub mod groups;
pub mod loader;
pub mod metadata;
pub mod scan;
mod store;
pub mod track;

pub use store::Library;
pub use track::Track;
