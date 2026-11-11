//! Audio file tags: title, artist, album, duration, cover.

use lofty::{
    config::ParseOptions,
    file::{AudioFile, TaggedFileExt},
    probe::Probe,
    tag::{Accessor, ItemKey},
};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub title: String,
    pub artist: String,
    /// Album artist (Album Artist) — tracks are grouped into albums by it. Empty
    /// if it's not in the tags
    pub album_artist: String,
    pub album: String,
    pub duration_secs: u64,
    pub year: Option<u16>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    /// Genre; empty if it's not in the tags. Not stored in the library — only needed for the
    /// current track's details
    pub genre: String,
    /// Audio bitrate in kbps — without the file's overhead; if it can't be computed — the overall
    pub bitrate_kbps: Option<u32>,
}

/// Reads the tags of `path`. No title in tags — the file name without extension
pub fn get_file_metadata(path: &Path) -> Result<FileMetadata, String> {
    // The cover is not read here: metadata is read for the whole list, while covers are needed only
    // for those visible on screen — the cover cache reads them on demand (see read_cover)
    let tagged = Probe::open(path)
        .map_err(|e| format!("Could not open the file: {e}"))?
        .options(ParseOptions::new().read_cover_art(false))
        .read()
        .map_err(|e| format!("Could not read the metadata: {e}"))?;

    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());

    let title = tag
        .and_then(|t| t.title().map(|s| fix_cp1251(&s)))
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_owned()
        });

    let artist = tag
        .and_then(|t| t.artist().map(|s| fix_cp1251(&s)))
        .unwrap_or_default();

    let album_artist = tag
        .and_then(|t| t.get_string(ItemKey::AlbumArtist).map(fix_cp1251))
        .unwrap_or_default();

    let album = tag
        .and_then(|t| t.album().map(|s| fix_cp1251(&s)))
        .unwrap_or_default();

    let duration_secs = tagged.properties().duration().as_secs();
    // Year — from the recording date or a separate year field, whichever the tags have
    let year = tag.and_then(|t| t.date()).map(|date| date.year);
    // Track and disc number — for track order within an album
    let track_number = tag.and_then(|t| t.track());
    let disc_number = tag.and_then(|t| t.disk());
    let genre = tag
        .and_then(|t| t.genre().map(|s| fix_cp1251(&s)))
        .unwrap_or_default();
    let properties = tagged.properties();
    let bitrate_kbps = properties
        .audio_bitrate()
        .or_else(|| properties.overall_bitrate())
        .filter(|&kbps| kbps > 0);

    Ok(FileMetadata {
        title,
        artist,
        album_artist,
        album,
        duration_secs,
        year,
        track_number,
        disc_number,
        genre,
        bitrate_kbps,
    })
}

/// Album cover from the file's tags. Read on demand — by the UI cover cache.
pub fn read_cover(path: &Path) -> Option<Vec<u8>> {
    let tagged = Probe::open(path).ok()?.read().ok()?;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag())?;
    tag.pictures()
        .first()
        .map(|picture| picture.data().to_vec())
}

/// Album cover as an image next to the audio files in folder `dir` — it's often stored this way
/// instead of in tags. An image with a telling name is used (`cover.jpg`, `Folder.png`, `front.jpg`…),
/// and if there are none — the only image in the folder. Among several images with other
/// names (booklet scans, disc back) we don't guess: showing the wrong one is worse than none
pub fn read_folder_cover(dir: &Path) -> Option<Vec<u8>> {
    const IMAGE_EXTENSIONS: [&str; 4] = ["jpg", "jpeg", "png", "webp"];
    const COVER_NAMES: [&str; 5] = ["cover", "folder", "front", "albumart", "album"];

    let mut images: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        })
        .collect();
    images.sort();

    let named = |name: &str| {
        images.iter().find(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .is_some_and(|stem| stem.to_lowercase() == name)
        })
    };
    let cover = COVER_NAMES
        .iter()
        .find_map(|name| named(name))
        .or(match images.as_slice() {
            [single] => Some(single),
            _ => None,
        })?;
    fs::read(cover).ok()
}

/// Fixes Windows-1251 tags that were read as Latin-1.
///
/// Old MP3s often store Russian tags in Windows-1251, but ID3 declares Latin-1.
/// Then each Cyrillic byte is read as a Latin letter with a diacritic:
/// "Полет шмеля" turns into "Ïîëåò øìåëÿ". Such a string is turned back into bytes
/// and decoded as Windows-1251.
///
/// To avoid breaking real Latin text with diacritics ("Beyoncé", "Mötley Crüe"),
/// re-encode only if most letters are in the U+00C0..=U+00FF range, where
/// Windows-1251 has Cyrillic.
pub(crate) fn fix_cp1251(text: &str) -> String {
    // A Latin-1 string consists only of characters up to U+00FF — otherwise it's not our case
    if text.chars().any(|c| c as u32 > 0xFF) {
        return text.to_owned();
    }

    let letters = text.chars().filter(|c| c.is_alphabetic()).count();
    let cyrillic_in_disguise = text
        .chars()
        .filter(|&c| ('\u{C0}'..='\u{FF}').contains(&c))
        .count();
    if cyrillic_in_disguise < 2 || cyrillic_in_disguise * 2 < letters {
        return text.to_owned();
    }

    let bytes: Vec<u8> = text.chars().map(|c| c as u8).collect();
    let (decoded, _) = encoding_rs::WINDOWS_1251.decode_without_bom_handling(&bytes);
    decoded.into_owned()
}

#[cfg(test)]
mod tests {
    use super::{fix_cp1251, read_folder_cover};
    use std::fs;

    /// A new empty temp folder with files `names`; each file's content is its name
    fn folder(names: &[&str]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("lmp_cover_{}", fastrand::u64(..)));
        fs::create_dir_all(&dir).unwrap();
        for name in names {
            fs::write(dir.join(name), name).unwrap();
        }
        dir
    }

    #[test]
    fn folder_cover_prefers_cover_names_then_single_image() {
        let read = |names: &[&str]| {
            let dir = folder(names);
            let cover = read_folder_cover(&dir).map(|bytes| String::from_utf8(bytes).unwrap());
            fs::remove_dir_all(dir).unwrap();
            cover
        };
        // The only image is the cover, whatever it's called
        assert_eq!(
            read(&["01 Song.mp3", "Dot Hacker - N°3.jpg"]).as_deref(),
            Some("Dot Hacker - N°3.jpg")
        );
        // A telling name wins over other images, case doesn't matter
        assert_eq!(
            read(&["back.jpg", "Folder.PNG", "cd.jpg"]).as_deref(),
            Some("Folder.PNG")
        );
        // Several images without telling names — don't guess
        assert_eq!(read(&["back.jpg", "booklet.jpg"]), None);
        assert_eq!(read(&["01 Song.mp3"]), None);
    }

    #[test]
    fn decodes_cp1251_read_as_latin1() {
        assert_eq!(
            fix_cp1251("\"Ïîëåò øìåëÿ\" èç îïåðû \"Ñêàçêà î öàðå Ñàëòàíå\""),
            "\"Полет шмеля\" из оперы \"Сказка о царе Салтане\""
        );
    }

    #[test]
    fn keeps_latin_with_diacritics() {
        assert_eq!(fix_cp1251("Beyoncé"), "Beyoncé");
        assert_eq!(fix_cp1251("Mötley Crüe"), "Mötley Crüe");
    }

    #[test]
    fn keeps_correct_text() {
        assert_eq!(fix_cp1251("Полет шмеля"), "Полет шмеля");
        assert_eq!(fix_cp1251("Shakira, Burna Boy"), "Shakira, Burna Boy");
    }
}
