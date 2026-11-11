//! Search on library pages: the query and found songs.

use crate::{core::library::Track, ui::app::PlayerApp};

/// Search in a library view. One query per open view: it's cleared when the view changes
/// (see `PlayerApp::navigate`). Artists searches by artist name, Albums — by album
/// title, Songs — by track title and artist name
#[derive(Default)]
pub(crate) struct LibrarySearch {
    pub(crate) query: String,
    /// Tracks found in Songs; `None` — the query is empty, the whole library is shown
    pub(crate) found_songs: Option<FoundSongs>,
}

impl LibrarySearch {
    pub(crate) fn is_empty(&self) -> bool {
        self.query.is_empty()
    }
}

/// Tracks found in Songs. Recomputed only when the query or the library changed:
/// the library may have tens of thousands of tracks, and frames are drawn constantly
pub(crate) struct FoundSongs {
    pub(crate) revision: u64,
    /// The lowercase query that was searched for
    pub(crate) query: String,
    /// Library indices of the found tracks
    pub(crate) indices: Vec<usize>,
    /// Copies of the found tracks — the list shows them
    pub(crate) tracks: Vec<Track>,
}

/// Search query for comparison: trimmed, lowercase
pub(crate) fn normalized_query(query: &str) -> String {
    query.trim().to_lowercase()
}

/// Whether `query` (already from `normalized_query`) is in `text`, case-insensitive.
/// An empty query matches everything
pub(crate) fn contains_query(text: &str, query: &str) -> bool {
    query.is_empty() || text.to_lowercase().contains(query)
}

impl PlayerApp {
    /// Recomputes the tracks found in Songs if the query or the library changed
    pub(crate) fn refresh_found_songs(&mut self) {
        let query = normalized_query(&self.search.query);
        if query.is_empty() {
            self.search.found_songs = None;
            return;
        }
        let revision = self.core.library.revision();
        let up_to_date = self
            .search
            .found_songs
            .as_ref()
            .is_some_and(|found| found.revision == revision && found.query == query);
        if up_to_date {
            return;
        }
        let (indices, tracks) = self
            .core
            .library
            .tracks()
            .iter()
            .enumerate()
            .filter(|(_, track)| {
                contains_query(&track.title, &query) || contains_query(&track.artist, &query)
            })
            .map(|(index, track)| (index, track.clone()))
            .unzip();
        self.search.found_songs = Some(FoundSongs {
            revision,
            query,
            indices,
            tracks,
        });
    }
}
