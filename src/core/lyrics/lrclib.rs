//! Lyrics request to LRCLIB (https://lrclib.net) — an open lyrics database, no keys.
//! A track is looked up by signature: artist, title, album and duration (the service tolerates
//! a duration mismatch of a couple of seconds).

use serde::{Deserialize, Serialize};

use crate::core::net;

const GET_URL: &str = "https://lrclib.net/api/get";
const SEARCH_URL: &str = "https://lrclib.net/api/search";

/// What the track is looked up by
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TrackSignature {
    pub artist: String,
    pub title: String,
    pub album: String,
    pub duration_secs: u64,
}

/// LRCLIB answer as stored in the cache
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Remote {
    Found {
        /// Synced lyrics in LRC format
        synced: Option<String>,
        plain: Option<String>,
        /// Instrumental track
        instrumental: bool,
    },
    NotFound,
}

/// An LRCLIB record — only the needed fields. Both `/api/get` and `/api/search` (as a list) answer this way
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Track {
    #[serde(default)]
    track_name: String,
    #[serde(default)]
    artist_name: String,
    /// Seconds
    #[serde(default)]
    duration: f64,
    #[serde(default)]
    instrumental: bool,
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
}

/// Max duration mismatch for a search record — the same as `/api/get` tolerates
const DURATION_TOLERANCE_SECS: f64 = 2.0;

/// Asks LRCLIB about a track: first by exact signature (`/api/get`), and if not found —
/// by searching artist and title: the album in tags is often named differently than in LRCLIB
/// ("… (Limited Edition)"). Error — network, server busy (503) and other temporary failures:
/// such an answer is not remembered, it can be asked again
pub fn fetch(signature: &TrackSignature) -> Result<Remote, String> {
    let agent = net::agent();

    let response = agent
        .get(GET_URL)
        .query("artist_name", &signature.artist)
        .query("track_name", &signature.title)
        .query("album_name", &signature.album)
        .query("duration", signature.duration_secs.to_string())
        .call();
    let track: Option<Track> = read(response)?;
    if let Some(track) = track {
        return Ok(found(track));
    }

    let response = agent
        .get(SEARCH_URL)
        .query("artist_name", &signature.artist)
        .query("track_name", &signature.title)
        .call();
    let candidates: Vec<Track> = read(response)?.unwrap_or_default();
    Ok(best_match(candidates, signature).map_or(Remote::NotFound, found))
}

/// Response body: `None` — 404, error — anything except 200 and 404
fn read<T: serde::de::DeserializeOwned>(
    response: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<Option<T>, String> {
    let mut response = response.map_err(|e| format!("Could not reach LRCLIB: {e}"))?;
    match response.status().as_u16() {
        200 => response
            .body_mut()
            .read_json()
            .map(Some)
            .map_err(|e| format!("LRCLIB sent an unexpected answer: {e}")),
        404 => Ok(None),
        status => Err(format!("LRCLIB answered with HTTP {status}")),
    }
}

fn found(track: Track) -> Remote {
    let non_empty = |text: Option<String>| text.filter(|text| !text.trim().is_empty());
    Remote::Found {
        synced: non_empty(track.synced_lyrics),
        plain: non_empty(track.plain_lyrics),
        instrumental: track.instrumental,
    }
}

/// A search record that is definitely about this track: same title and artist (case-
/// insensitive), duration within `DURATION_TOLERANCE_SECS`. The search is fuzzy,
/// and without these checks another song's lyrics could be shown. Among matches — ones with synced
/// lyrics first, then the closest duration
fn best_match(candidates: Vec<Track>, signature: &TrackSignature) -> Option<Track> {
    let same = |a: &str, b: &str| a.trim().to_lowercase() == b.trim().to_lowercase();
    let duration = signature.duration_secs as f64;
    candidates
        .into_iter()
        .filter(|track| {
            same(&track.track_name, &signature.title)
                && same(&track.artist_name, &signature.artist)
                && (track.duration - duration).abs() <= DURATION_TOLERANCE_SECS
                && (track.instrumental
                    || track.synced_lyrics.is_some()
                    || track.plain_lyrics.is_some())
        })
        .min_by(|a, b| {
            let rank = |track: &Track| {
                (
                    track.synced_lyrics.is_none(),
                    ((track.duration - duration).abs() * 1000.0) as u64,
                )
            };
            rank(a).cmp(&rank(b))
        })
}

#[cfg(test)]
mod tests {
    use super::{Track, TrackSignature, best_match};

    fn track(name: &str, artist: &str, duration: f64, synced: bool) -> Track {
        Track {
            track_name: name.into(),
            artist_name: artist.into(),
            duration,
            instrumental: false,
            plain_lyrics: Some("Line".into()),
            synced_lyrics: synced.then(|| "[00:01.00]Line".into()),
        }
    }

    #[test]
    fn picks_matching_search_result() {
        let signature = TrackSignature {
            artist: "HIM".into(),
            title: "Join Me In Death".into(),
            album: "Razorblade Romance (Limited Edition)".into(),
            duration_secs: 217,
        };
        let candidates = vec![
            // Different song, different artist, distant duration — not a match
            track("Join Me", "HIM", 217.0, true),
            track("Join Me In Death", "HIM - Topic", 217.0, true),
            track("Join Me In Death", "HIM", 222.0, true),
            // Matches: without synced lyrics, and with them but slightly farther in duration
            track("Join Me In Death", "HIM", 217.0, false),
            track("Join Me in Death", "Him", 218.0, true),
        ];
        let best = best_match(candidates, &signature).unwrap();
        assert_eq!(best.artist_name, "Him");
        assert!(best.synced_lyrics.is_some());

        assert!(best_match(vec![track("Other", "HIM", 217.0, true)], &signature).is_none());
    }
}
