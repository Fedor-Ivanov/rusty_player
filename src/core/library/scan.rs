//! Which files the player can play and where to find them.

use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};
use symphonia::{
    core::{
        codecs::{CodecParameters, audio::AudioDecoderOptions},
        formats::probe::Hint,
        io::MediaSourceStream,
    },
    default::{get_codecs, get_probe},
};

/// Extensions of supported audio files (lowercase).
pub const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "flac", "wav", "ogg", "m4a", "aac"];

/// All audio files of supported formats in a folder, including subfolders, sorted by path
/// case-insensitively.
/// Other files (covers, playlists, etc.) are skipped; subfolders
/// that couldn't be read are skipped too, so one such folder doesn't break the whole adding.
pub fn find_audio_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    fn walk(dir: &Path, found: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                let _ = walk(&path, found);
            } else if is_supported_extension(&path) {
                found.push(path);
            }
        }
        Ok(())
    }

    let mut found = Vec::new();
    walk(dir, &mut found).map_err(|e| format!("Could not read the folder: {e}"))?;
    // Case-insensitive, like in Finder and Explorer
    found.sort_by_key(|path| path.to_string_lossy().to_lowercase());
    Ok(found)
}

fn is_supported_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| SUPPORTED_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// Checks that a file can be played: known extension, container and codec.
pub fn check_supported(path: &Path) -> Result<(), String> {
    // Level 0: extension
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .filter(|e| SUPPORTED_EXTENSIONS.contains(&e.as_str()))
        .ok_or_else(|| "Unsupported file format".to_string())?;

    let file = File::open(path).map_err(|e| format!("Could not open the file: {e}"))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    hint.with_extension(&ext);

    // Level 1: container
    let format = get_probe()
        .probe(&hint, mss, Default::default(), Default::default())
        .map_err(|_| "Unsupported or damaged file".to_string())?;

    // Take the first audio track
    let params = format
        .tracks()
        .iter()
        .find_map(|t| match &t.codec_params {
            Some(CodecParameters::Audio(p)) => Some(p),
            _ => None,
        })
        .ok_or_else(|| "The file has no audio track".to_string())?;

    // Level 2: codec
    get_codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .map_err(|_| "This audio codec is not supported".to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::find_audio_files;
    use std::fs;

    #[test]
    fn finds_audio_files_recursively() {
        let dir = std::env::temp_dir().join(format!("lmp_test_{}", std::process::id()));
        fs::create_dir_all(dir.join("CD2")).unwrap();
        for name in ["b.mp3", "a.FLAC", "cover.jpg", "CD2/c.ogg", "CD2/notes.txt"] {
            fs::write(dir.join(name), b"").unwrap();
        }

        let found = find_audio_files(&dir).unwrap();
        fs::remove_dir_all(&dir).unwrap();

        let names: Vec<_> = found
            .iter()
            .map(|p| p.strip_prefix(&dir).unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["a.FLAC", "b.mp3", "CD2/c.ogg"]);
    }
}
