use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{metadata::get_file_metadata, scan::check_supported};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    /// Album artist from tags (Album Artist); empty if missing. Old library files
    /// don't have the field — then empty
    #[serde(default)]
    pub album_artist: String,
    pub duration_secs: u64,
    pub album: String,
    /// Year from tags. Old library files don't have the field — then `None`
    #[serde(default)]
    pub year: Option<u16>,
    /// Track and disc number from tags — track order in an album. Old library files
    /// don't have the fields — then `None`
    #[serde(default)]
    pub track_number: Option<u32>,
    #[serde(default)]
    pub disc_number: Option<u32>,
}

impl Track {
    /// Creates a track from a file: checks that it can be played and reads metadata.
    pub fn from_path(path: PathBuf) -> Result<Self, String> {
        check_supported(&path)?;
        let meta = get_file_metadata(&path)?;

        Ok(Self {
            path,
            title: meta.title,
            artist: meta.artist,
            album_artist: meta.album_artist,
            album: meta.album,
            duration_secs: meta.duration_secs,
            year: meta.year,
            track_number: meta.track_number,
            disc_number: meta.disc_number,
        })
    }
}
