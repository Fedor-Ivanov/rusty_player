//! UI localization: all texts in English, French, Russian and Ukrainian.
//!
//! The texts of one language are a `Texts` value (see `en.rs`, `fr.rs`, `ru.rs`, `uk.rs`). A text missed in a translation
//! won't let the project build: the struct has no default values.
//! Texts with substitutions are templates with `{name}`, filled by `fill`. Numbers with a word —
//! `Texts::count`: in Russian and Ukrainian a word has three forms (1 трек, 2 трека, 5 треков).
//!
//! A new language: a variant in `settings::Language` (and in `Language::ALL`), a file with its `Texts`
//! and lines in `t()` and `language_name` — the compiler will point out what's missing.
//!
//! There's one language for the whole app: widgets need it without extra parameters, so the current
//! language is global (`set_language`), and texts are taken via `t()`.

mod en;
mod fr;
mod ru;
mod uk;

use std::sync::atomic::{AtomicU8, Ordering};

use crate::core::settings::{GlassTint, Language};

/// Plural form rule in Russian and Ukrainian: 1, 21, 101 — "трек"; 2–4, 22–24 — "трека";
/// 0, 5–20, 25–30, 111–114 — "треков"
fn slavic_plural_form(count: usize) -> PluralForm {
    match (count % 10, count % 100) {
        (1, rest) if rest != 11 => PluralForm::One,
        (2..=4, rest) if !(12..=14).contains(&rest) => PluralForm::Few,
        _ => PluralForm::Many,
    }
}

/// A word in forms for different numbers. In English `few` is the same as `many`
pub struct Plural {
    one: &'static str,
    few: &'static str,
    many: &'static str,
}

impl Plural {
    /// Two forms — like in English: 1 track, 2 tracks
    const fn two(one: &'static str, many: &'static str) -> Self {
        Self {
            one,
            few: many,
            many,
        }
    }

    /// Three forms — like in Russian: 1 трек, 2 трека, 5 треков
    const fn three(one: &'static str, few: &'static str, many: &'static str) -> Self {
        Self { one, few, many }
    }
}

/// Which word form is needed for a number
pub enum PluralForm {
    One,
    Few,
    Many,
}

pub struct Texts {
    /// Which word form to use for a number — the language's rule
    plural_form: fn(usize) -> PluralForm,

    // ---- sections
    pub library: &'static str,
    pub now_playing: &'static str,
    pub artists: &'static str,
    pub albums: &'static str,
    pub songs: &'static str,
    /// Playback queue — the right panel
    pub queue: &'static str,
    /// Saved playlists — a library section
    pub playlists: &'static str,
    pub settings: &'static str,

    // ---- words for numbers
    pub track_count: Plural,
    pub song_count: Plural,
    pub album_count: Plural,
    pub playlist_count: Plural,
    pub artist_count: Plural,
    /// "…and N more files could not be added"
    pub more_file_count: Plural,

    // ---- placeholders
    pub no_tracks: &'static str,
    pub library_empty: &'static str,
    pub no_results: &'static str,
    pub no_albums: &'static str,
    pub no_playlists: &'static str,

    // ---- search and counters in titles
    pub search: &'static str,
    /// How many found in Songs: `{found}`, `{tracks}` — total tracks with the number
    pub found_of: &'static str,
    /// Bitrate in track details: "320 kbps"
    pub bitrate: &'static str,

    // ---- adding files
    pub open_files: &'static str,
    pub add_files: &'static str,
    pub add_folder: &'static str,
    /// Filter in the file picker dialog
    pub audio_files: &'static str,
    /// Progress banner: `{done}`, `{total}`
    pub adding_progress: &'static str,
    pub updating_library_progress: &'static str,
    pub scanning_folder: &'static str,
    pub no_audio_found: &'static str,
    /// Tail of the adding errors list: `{more}` — a number with a word from `more_file_count`
    pub more_files_failed: &'static str,

    // ---- navigation and hints
    pub show_artist: &'static str,
    /// Track menu item: open its album
    pub go_to_album: &'static str,
    /// Queue track menu item: show it in the library's song list
    pub show_in_library: &'static str,
    /// Lyrics button and the placeholder when there are no lyrics
    pub lyrics: &'static str,
    pub no_lyrics: &'static str,
    pub searching_lyrics: &'static str,
    /// LRCLIB didn't answer — no lyrics yet
    pub lyrics_unavailable: &'static str,
    /// No lyrics, and online search is off
    pub lyrics_offline: &'static str,
    /// An instrumental track — that's what LRCLIB says about it
    pub instrumental: &'static str,
    pub show_album: &'static str,
    pub show_current_track: &'static str,
    pub file_missing: &'static str,
    pub shuffle: &'static str,
    pub repeat_all: &'static str,
    pub repeat_one: &'static str,

    // ---- actions
    pub play: &'static str,
    pub add_to_queue: &'static str,
    pub remove_from_library: &'static str,
    pub remove_from_queue: &'static str,
    /// Actions on several selected items: `{items}` — a number with a word
    pub play_items: &'static str,
    pub add_items_to_queue: &'static str,
    pub remove_items_from_library: &'static str,
    pub remove_items_from_queue: &'static str,
    pub remove: &'static str,
    pub cancel: &'static str,
    pub ok: &'static str,
    pub done: &'static str,

    // ---- playlists
    /// Queue button tooltips: save it as a playlist and clear it
    pub save_queue: &'static str,
    /// Save the queue into its linked playlist: `{playlist}`
    pub save_queue_to: &'static str,
    pub save_as_new_playlist: &'static str,
    pub clear_queue: &'static str,
    /// The "add to saved playlist" submenu and its first item
    pub add_to_playlist: &'static str,
    pub new_playlist: &'static str,
    /// Remove tracks on the playlist page: `{items}` — a number with a word
    pub remove_from_this_playlist: &'static str,
    /// A playlist track not from the library — into the library: `{items}` — a number with a word
    pub add_to_library: &'static str,
    pub add_items_to_library: &'static str,
    pub remove_items_from_this_playlist: &'static str,
    pub rename: &'static str,
    pub delete: &'static str,
    pub save: &'static str,
    /// Playlist name dialog titles and the field hint
    pub save_playlist_title: &'static str,
    pub new_playlist_title: &'static str,
    pub rename_playlist_title: &'static str,
    pub playlist_name_hint: &'static str,
    pub delete_playlist_title: &'static str,
    /// `{playlist}`
    pub delete_playlist_message: &'static str,
    pub delete_playlists_title: &'static str,
    pub delete_playlists_message: &'static str,
    /// "Delete 3 playlists" — the menu item for several selected
    pub delete_items: &'static str,

    // ---- settings
    pub language: &'static str,
    pub version: &'static str,
    pub online_search: &'static str,
    /// Under the switch: what is sent and where
    pub online_search_note: &'static str,
    pub glass_tint: &'static str,
    pub transparency: &'static str,
    pub tint_graphite: &'static str,
    pub tint_blue: &'static str,
    pub tint_violet: &'static str,
    pub tint_green: &'static str,
    pub tint_rose: &'static str,
    pub tint_amber: &'static str,

    // ---- remove-from-library dialog
    pub remove_album_title: &'static str,
    /// `{album}`
    pub remove_album_message: &'static str,
    /// `{albums}`
    pub remove_albums_title: &'static str,
    /// `{albums}`, `{tracks}`
    pub remove_albums_message: &'static str,
    pub remove_artist_title: &'static str,
    /// `{artist}`
    pub remove_artist_message: &'static str,
    /// `{artists}`
    pub remove_artists_title: &'static str,
    /// `{artists}`, `{tracks}`
    pub remove_artists_message: &'static str,
    pub remove_song_title: &'static str,
    /// `{song}`
    pub remove_song_message: &'static str,
    /// `{songs}`
    pub remove_songs_title: &'static str,
    /// `{songs}`
    pub remove_songs_message: &'static str,
}

impl Texts {
    /// Label of an action on tracks: for one — `one` ("Add to library"), for several —
    /// the `many` template with their number in `{items}` ("Add 3 tracks to library")
    pub fn for_tracks(&self, one: &str, many: &str, count: usize) -> String {
        match count {
            0 | 1 => one.to_owned(),
            count => fill(many, &[("items", &self.count(count, &self.track_count))]),
        }
    }

    /// A number with a word in the right form: "3 tracks", "3 трека"
    pub fn count(&self, count: usize, word: &Plural) -> String {
        let form = match (self.plural_form)(count) {
            PluralForm::One => word.one,
            PluralForm::Few => word.few,
            PluralForm::Many => word.many,
        };
        format!("{count} {form}")
    }
}

/// A template with substituted values: `{name}` in `template` is replaced with the value
pub fn fill(template: &str, values: &[(&str, &str)]) -> String {
    values
        .iter()
        .fold(template.to_owned(), |text, (name, value)| {
            text.replace(&format!("{{{name}}}"), value)
        })
}

/// Glass tint name in the current language
pub fn tint_name(tint: GlassTint) -> &'static str {
    match tint {
        GlassTint::Graphite => t().tint_graphite,
        GlassTint::Blue => t().tint_blue,
        GlassTint::Violet => t().tint_violet,
        GlassTint::Green => t().tint_green,
        GlassTint::Rose => t().tint_rose,
        GlassTint::Amber => t().tint_amber,
    }
}

/// A language's name in itself — that's how languages are shown in the picker: whoever doesn't understand
/// the current UI language will still find their own
pub fn language_name(language: Language) -> &'static str {
    match language {
        Language::English => "English",
        Language::French => "Français",
        Language::Russian => "Русский",
        Language::Ukrainian => "Українська",
    }
}

static LANGUAGE: AtomicU8 = AtomicU8::new(0);

/// Makes `language` the UI language — from the next frame all texts use it
pub fn set_language(language: Language) {
    LANGUAGE.store(language as u8, Ordering::Relaxed);
}

/// Texts in the current language
pub fn t() -> &'static Texts {
    let value = LANGUAGE.load(Ordering::Relaxed);
    let language = Language::ALL
        .into_iter()
        .find(|&language| language as u8 == value)
        .unwrap_or_default();
    match language {
        Language::English => &en::TEXTS,
        Language::French => &fr::TEXTS,
        Language::Russian => &ru::TEXTS,
        Language::Ukrainian => &uk::TEXTS,
    }
}

#[cfg(test)]
mod tests {
    use super::{Texts, en, fill, fr, ru, uk};
    use std::collections::BTreeSet;

    /// Templates with substitutions — to compare their `{…}` sets across languages
    fn templates(t: &Texts) -> [(&'static str, &'static str); 25] {
        [
            ("found_of", t.found_of),
            ("bitrate", t.bitrate),
            ("adding_progress", t.adding_progress),
            ("updating_library_progress", t.updating_library_progress),
            ("more_files_failed", t.more_files_failed),
            ("play_items", t.play_items),
            ("add_items_to_queue", t.add_items_to_queue),
            ("remove_items_from_library", t.remove_items_from_library),
            ("remove_items_from_queue", t.remove_items_from_queue),
            ("remove_album_message", t.remove_album_message),
            ("remove_albums_title", t.remove_albums_title),
            ("remove_albums_message", t.remove_albums_message),
            ("remove_artist_message", t.remove_artist_message),
            ("remove_artists_title", t.remove_artists_title),
            ("remove_artists_message", t.remove_artists_message),
            ("remove_song_message", t.remove_song_message),
            ("remove_songs_title", t.remove_songs_title),
            ("remove_songs_message", t.remove_songs_message),
            (
                "remove_items_from_this_playlist",
                t.remove_items_from_this_playlist,
            ),
            ("delete_playlist_message", t.delete_playlist_message),
            ("delete_playlists_title", t.delete_playlists_title),
            ("delete_playlists_message", t.delete_playlists_message),
            ("delete_items", t.delete_items),
            ("save_queue_to", t.save_queue_to),
            ("add_items_to_library", t.add_items_to_library),
        ]
    }

    fn placeholders(template: &str) -> BTreeSet<&str> {
        template
            .split('{')
            .skip(1)
            .filter_map(|part| part.split_once('}').map(|(name, _)| name))
            .collect()
    }

    #[test]
    fn translations_have_same_placeholders() {
        for (language, texts) in [("fr", &fr::TEXTS), ("ru", &ru::TEXTS), ("uk", &uk::TEXTS)] {
            for ((name, english), (_, translated)) in
                templates(&en::TEXTS).into_iter().zip(templates(texts))
            {
                assert_eq!(
                    placeholders(english),
                    placeholders(translated),
                    "{language}: {name}"
                );
            }
        }
    }

    #[test]
    fn ukrainian_and_french_plural_forms() {
        let t = &uk::TEXTS;
        let tracks = |n| t.count(n, &t.track_count);
        assert_eq!(tracks(1), "1 трек");
        assert_eq!(tracks(3), "3 треки");
        assert_eq!(tracks(5), "5 треків");
        assert_eq!(tracks(21), "21 трек");

        // In French 0 is singular too
        let t = &fr::TEXTS;
        assert_eq!(t.count(0, &t.track_count), "0 piste");
        assert_eq!(t.count(1, &t.track_count), "1 piste");
        assert_eq!(t.count(2, &t.track_count), "2 pistes");
    }

    #[test]
    fn russian_plural_forms() {
        let t = &ru::TEXTS;
        let tracks = |n| t.count(n, &t.track_count);
        assert_eq!(tracks(1), "1 трек");
        assert_eq!(tracks(3), "3 трека");
        assert_eq!(tracks(5), "5 треков");
        assert_eq!(tracks(11), "11 треков");
        assert_eq!(tracks(12), "12 треков");
        assert_eq!(tracks(21), "21 трек");
        assert_eq!(tracks(22), "22 трека");
        assert_eq!(tracks(111), "111 треков");
        assert_eq!(tracks(0), "0 треков");
    }

    #[test]
    fn english_plural_forms_and_fill() {
        let t = &en::TEXTS;
        assert_eq!(t.count(1, &t.track_count), "1 track");
        assert_eq!(t.count(2, &t.track_count), "2 tracks");
        assert_eq!(
            fill(t.adding_progress, &[("done", "3"), ("total", "10")]),
            "Adding 3 of 10"
        );
    }
}
