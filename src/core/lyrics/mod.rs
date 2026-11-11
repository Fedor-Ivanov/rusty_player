//! Lyrics: from an `.lrc` file next to the track, from tags or from LRCLIB (see `finder`).
//! Synced lyrics (LRC format: lines with timestamps `[01:23.45]`) know
//! which line is being sung now; plain lyrics are just lines.

pub mod finder;
pub mod lrclib;

pub use finder::{LyricsFinder, TrackLyrics};

use lofty::{file::TaggedFileExt, probe::Probe, tag::ItemKey};
use std::{fs, path::Path};

use crate::core::library::metadata::fix_cp1251;

#[derive(Debug, Clone, PartialEq)]
pub struct Lyrics {
    /// By time if the lyrics are synced; otherwise in text order
    pub lines: Vec<LyricLine>,
    /// Lines have times: the current one can be highlighted
    pub synced: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LyricLine {
    /// When the line starts, in milliseconds; `None` for unsynced lyrics
    pub start_ms: Option<u64>,
    pub text: String,
}

impl Lyrics {
    /// The line being sung at `position_ms`: the last one that has started. `None` — the lyrics
    /// are not synced or the first line hasn't started yet
    pub fn current_line(&self, position_ms: u64) -> Option<usize> {
        if !self.synced {
            return None;
        }
        // Lines are sorted by time: the number of started lines is the index of the next one
        let started = self
            .lines
            .partition_point(|line| line.start_ms.is_some_and(|start| start <= position_ms));
        started.checked_sub(1)
    }
}

/// Lyrics for track `path`. First — an `.lrc` file with the same name in the same folder
/// (that's how downloaded lyrics are stored), then — lyrics in tags. Synced
/// is preferred over plain. `None` — no lyrics anywhere
pub fn load_lyrics(path: &Path) -> Option<Lyrics> {
    let mut candidates = Vec::new();
    if let Some(text) = read_lrc_file(&path.with_extension("lrc")) {
        candidates.push(text);
    }
    if let Ok(tagged) = Probe::open(path).and_then(|probe| probe.read()) {
        for tag in tagged.tags() {
            for key in [ItemKey::Lyrics, ItemKey::UnsyncLyrics] {
                if let Some(text) = tag.get_string(key) {
                    candidates.push(fix_cp1251(text));
                }
            }
        }
    }
    let parsed: Vec<Lyrics> = candidates
        .iter()
        .filter_map(|text| parse_lyrics(text))
        .collect();
    parsed
        .iter()
        .find(|lyrics| lyrics.synced)
        .or_else(|| parsed.first())
        .cloned()
}

/// `.lrc` file: usually UTF-8, but old ones may be in Windows-1251
fn read_lrc_file(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&bytes);
    Some(match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => encoding_rs::WINDOWS_1251.decode(bytes).0.into_owned(),
    })
}

/// Parses lyrics. If there are lines with timestamps — synced (LRC),
/// otherwise — plain. Empty text — `None`
pub fn parse_lyrics(text: &str) -> Option<Lyrics> {
    let mut offset_ms: i64 = 0;
    let mut timed: Vec<LyricLine> = Vec::new();
    let mut plain: Vec<String> = Vec::new();

    // Lines are separated differently: \n, \r\n and a lone \r (old Mac format — happens
    // in MP3 tags). `lines` knows only the first two
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    for raw in text.lines() {
        let line = raw.trim();
        let (times, rest) = split_timestamps(line);
        if times.is_empty() {
            // LRC metadata lines: [ar:…], [ti:…], [offset:…] — not lyrics
            if let Some(tag) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']'))
                && let Some((name, value)) = tag.split_once(':')
                && name.chars().all(|c| c.is_ascii_alphabetic())
            {
                if name.eq_ignore_ascii_case("offset") {
                    offset_ms = value.trim().parse().unwrap_or(0);
                }
                continue;
            }
            plain.push(line.to_owned());
            continue;
        }
        let text = strip_word_timestamps(rest);
        // A repeated chorus is written as one line with several timestamps
        for start in times {
            timed.push(LyricLine {
                start_ms: Some(start),
                text: text.clone(),
            });
        }
    }

    if !timed.is_empty() {
        // A positive offset means the lyrics should appear earlier
        for line in &mut timed {
            line.start_ms = line
                .start_ms
                .map(|start| (start as i64 - offset_ms).max(0) as u64);
        }
        timed.sort_by_key(|line| line.start_ms);
        return Some(Lyrics {
            lines: timed,
            synced: true,
        });
    }

    // Plain lyrics — without blank lines at the edges
    let first = plain.iter().position(|line| !line.is_empty())?;
    let last = plain.iter().rposition(|line| !line.is_empty())?;
    Some(Lyrics {
        lines: plain[first..=last]
            .iter()
            .map(|text| LyricLine {
                start_ms: None,
                text: text.clone(),
            })
            .collect(),
        synced: false,
    })
}

/// Timestamps at the start of a line (`[01:23.45][02:10.00]text`) and the rest of the line
fn split_timestamps(mut line: &str) -> (Vec<u64>, &str) {
    let mut times = Vec::new();
    while let Some(rest) = line.strip_prefix('[') {
        let Some((stamp, after)) = rest.split_once(']') else {
            break;
        };
        let Some(ms) = parse_timestamp(stamp) else {
            break;
        };
        times.push(ms);
        line = after;
    }
    (times, line.trim())
}

/// `mm:ss`, `mm:ss.xx`, `mm:ss.xxx` or `mm:ss:xx` — to milliseconds
fn parse_timestamp(stamp: &str) -> Option<u64> {
    let (minutes, rest) = stamp.split_once(':')?;
    let minutes: u64 = minutes.trim().parse().ok()?;
    let (seconds, fraction) = match rest.split_once(['.', ':']) {
        Some((seconds, fraction)) => (seconds, fraction),
        None => (rest, ""),
    };
    let seconds: u64 = seconds.trim().parse().ok()?;
    if seconds >= 60 {
        return None;
    }
    // The fraction is fractions of a second: "4" — 400 ms, "45" — 450, "456" — 456
    let fraction_ms = if fraction.is_empty() {
        0
    } else {
        let digits: String = fraction.chars().take(3).collect();
        let value: u64 = digits.parse().ok()?;
        value * 10u64.pow(3 - digits.len() as u32)
    };
    Some((minutes * 60 + seconds) * 1000 + fraction_ms)
}

/// Removes per-word timestamps of enhanced LRC: `<01:23.45>`
fn strip_word_timestamps(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('<') {
        let Some(end) = rest[start..].find('>') else {
            break;
        };
        if parse_timestamp(&rest[start + 1..start + end]).is_none() {
            out.push_str(&rest[..start + end + 1]);
        } else {
            out.push_str(&rest[..start]);
        }
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);
    // After removing timestamps, double spaces remain between words
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::{Lyrics, parse_lyrics};

    fn starts(lyrics: &Lyrics) -> Vec<(Option<u64>, &str)> {
        lyrics
            .lines
            .iter()
            .map(|line| (line.start_ms, line.text.as_str()))
            .collect()
    }

    #[test]
    fn parses_synced_lyrics() {
        let lyrics = parse_lyrics(
            "[ar:Artist]\n[offset:+100]\n[00:12.34]First\n\
             [00:05.00][01:00.5]Chorus\n[00:20]<00:20.00>Word <00:21.00>by word\n[00:30.00]",
        )
        .unwrap();
        assert!(lyrics.synced);
        assert_eq!(
            starts(&lyrics),
            [
                (Some(4_900), "Chorus"),
                (Some(12_240), "First"),
                (Some(19_900), "Word by word"),
                (Some(29_900), ""),
                (Some(60_400), "Chorus"),
            ]
        );
        assert_eq!(lyrics.current_line(0), None);
        assert_eq!(lyrics.current_line(4_900), Some(0));
        assert_eq!(lyrics.current_line(15_000), Some(1));
        assert_eq!(lyrics.current_line(100_000), Some(4));
    }

    #[test]
    fn parses_plain_lyrics() {
        let lyrics = parse_lyrics("\n\nFirst line\n\nSecond line\n\n").unwrap();
        assert!(!lyrics.synced);
        assert_eq!(
            starts(&lyrics),
            [(None, "First line"), (None, ""), (None, "Second line")]
        );
        assert_eq!(lyrics.current_line(10_000), None);
        assert_eq!(parse_lyrics("  \n "), None);

        // Lines split by a lone \r and by \r\n are the same lines
        for text in ["First line\r\rSecond line", "First line\r\n\r\nSecond line"] {
            let lyrics = parse_lyrics(text).unwrap();
            assert_eq!(
                starts(&lyrics),
                [(None, "First line"), (None, ""), (None, "Second line")]
            );
        }
    }
}
