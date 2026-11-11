//! App settings. Kept between launches in `settings.json` in the data folder —
//! next to the library and the queue, but separately: changing a setting doesn't rewrite
//! the library, and the library format doesn't depend on settings.

use serde::{Deserialize, Serialize};
use std::{fs, io::ErrorKind, path::PathBuf};

use crate::{core::playback::Volume, core::storage::write_atomically};

const FILE_NAME: &str = "settings.json";

#[derive(
    Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord,
)]
pub enum Language {
    #[default]
    English,
    Russian,
    French,
    Ukrainian,
}

impl Language {
    /// All languages — in the order of the list in settings
    pub const ALL: [Language; 4] = [
        Language::English,
        Language::French,
        Language::Russian,
        Language::Ukrainian,
    ];
}

/// Glass tint — tinting over the system blur under the window. The colors themselves are in the UI
/// theme
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GlassTint {
    #[default]
    Graphite,
    Blue,
    Violet,
    Green,
    Rose,
    Amber,
}

impl GlassTint {
    /// All tints — in the order of the circles in settings
    pub const ALL: [GlassTint; 6] = [
        GlassTint::Graphite,
        GlassTint::Blue,
        GlassTint::Violet,
        GlassTint::Green,
        GlassTint::Rose,
        GlassTint::Amber,
    ];
}

/// Window glass: tint and transparency
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct Glass {
    pub tint: GlassTint,
    /// 0.0 — the tint is opaque, nothing is visible behind the window; 1.0 — the most transparent
    /// glass the UI allows
    pub transparency: f32,
}

impl Default for Glass {
    fn default() -> Self {
        Self {
            tint: GlassTint::default(),
            transparency: 0.4,
        }
    }
}

/// All settings. A field missing from the file gets its default value
/// (`serde(default)`): a file from a previous app version that didn't have this setting yet
/// is read without errors — no separate format version is needed
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Settings {
    language: Language,
    volume: Volume,
    glass: Glass,
    /// Search online for what's missing from files: artist photos (Deezer) and
    /// lyrics (LRCLIB). Off until the user turns it on: otherwise the app would send
    /// artist names and track titles to the services without asking
    online_lookups: bool,
}

impl Settings {
    pub fn language(&self) -> Language {
        self.language
    }

    pub fn update_language(&mut self, language: Language) {
        self.language = language;
    }

    pub fn volume(&self) -> Volume {
        self.volume
    }

    pub fn update_volume(&mut self, volume: Volume) {
        self.volume = volume;
    }

    pub fn glass(&self) -> Glass {
        self.glass
    }

    pub fn online_lookups(&self) -> bool {
        self.online_lookups
    }

    pub fn update_online_lookups(&mut self, online: bool) {
        self.online_lookups = online;
    }

    pub fn update_glass(&mut self, glass: Glass) {
        self.glass = Glass {
            transparency: glass.transparency.clamp(0.0, 1.0),
            ..glass
        };
    }
}

/// Settings file. Remembers what's in it — doesn't rewrite unchanged settings
pub struct SettingsStore {
    /// Where to save; `None` — the data folder is unavailable, settings live only until exit
    file: Option<PathBuf>,
    saved: Settings,
}

impl SettingsStore {
    /// Loads settings from data folder `dir`. No file — default settings.
    /// The third value is an error to show the user if the file couldn't be read
    pub fn load(dir: Option<PathBuf>) -> (Self, Settings, Option<String>) {
        let file = dir.map(|dir| dir.join(FILE_NAME));
        let (settings, error) = match file.as_ref().map(fs::read_to_string) {
            None => (Settings::default(), None),
            Some(Err(e)) if e.kind() == ErrorKind::NotFound => (Settings::default(), None),
            Some(Err(e)) => (
                Settings::default(),
                Some(format!("Could not read the settings: {e}")),
            ),
            Some(Ok(text)) => match serde_json::from_str(&text) {
                Ok(settings) => (settings, None),
                Err(e) => (
                    Settings::default(),
                    Some(format!("The settings file is damaged, using defaults: {e}")),
                ),
            },
        };
        let store = Self {
            file,
            saved: settings.clone(),
        };
        (store, settings, error)
    }

    /// Saves settings if they changed since the last save
    pub fn save_if_changed(&mut self, settings: &Settings) -> Result<(), String> {
        if *settings == self.saved {
            return Ok(());
        }
        // Considered saved even on error: otherwise writing would repeat every frame.
        // The next change will try again
        self.saved = settings.clone();
        let Some(file) = &self.file else {
            return Ok(());
        };
        write_atomically(file, settings).map_err(|e| format!("Could not save the settings: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::{Glass, GlassTint, Language, Settings, SettingsStore};
    use crate::core::playback::Volume;
    use std::fs;

    #[test]
    fn saves_loads_and_fills_missing_fields() {
        let dir = std::env::temp_dir().join(format!("lmp_settings_{}", fastrand::u64(..)));

        let (mut store, mut settings, error) = SettingsStore::load(Some(dir.clone()));
        assert!(error.is_none());
        assert_eq!(settings, Settings::default());

        settings.update_language(Language::Russian);
        settings.update_volume(Volume {
            level: 0.3,
            muted: true,
        });
        settings.update_glass(Glass {
            tint: GlassTint::Violet,
            transparency: 0.7,
        });
        store.save_if_changed(&settings).unwrap();
        let (_, loaded, error) = SettingsStore::load(Some(dir.clone()));
        assert!(error.is_none());
        assert_eq!(loaded, settings);

        // An old-version file without volume: the language is read, the volume is default
        fs::write(dir.join(super::FILE_NAME), r#"{"language":"Russian"}"#).unwrap();
        let (_, loaded, error) = SettingsStore::load(Some(dir.clone()));
        assert!(error.is_none());
        assert_eq!(loaded.language(), Language::Russian);
        assert_eq!(loaded.volume(), Volume::default());
        assert_eq!(loaded.glass(), Glass::default());
        // Online search — only if the user enabled it
        assert!(!loaded.online_lookups());

        fs::remove_dir_all(dir).unwrap();
    }
}
