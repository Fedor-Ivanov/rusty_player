//! Navigation: what's open in the central area. A single `Route` value instead of separate
//! "view", "artist", "album" fields — invalid combinations (an album in Songs) can't be expressed.
//! Transitions only via `PlayerApp::navigate`: everything that changes with them is there too.

use super::app::PlayerApp;
use crate::{
    core::{library::groups::AlbumGroup, playlists::PlaylistId},
    ui::i18n,
};

/// Section — the top navigation level: a menu item and a title
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Section {
    NowPlaying,
    Artists,
    Albums,
    Songs,
    Playlists,
}

impl Section {
    /// Library sections — menu items on the left, in order
    pub(crate) const LIBRARY: [Section; 4] = [
        Section::Artists,
        Section::Albums,
        Section::Songs,
        Section::Playlists,
    ];

    /// Name in the UI language
    pub(crate) fn title(self) -> &'static str {
        let t = i18n::t();
        match self {
            Self::NowPlaying => t.now_playing,
            Self::Artists => t.artists,
            Self::Albums => t.albums,
            Self::Songs => t.songs,
            Self::Playlists => t.playlists,
        }
    }
}

/// What's open in the central area
#[derive(Clone, PartialEq)]
pub(crate) enum Route {
    NowPlaying,
    Artists,
    /// Artist profile: their albums
    Artist(String),
    /// An album opened from the profile of artist `artist`
    ArtistAlbum {
        artist: String,
        album: AlbumKey,
    },
    Albums,
    /// An album opened from all albums
    Album(AlbumKey),
    Songs,
    Playlists,
    /// A saved playlist
    Playlist(PlaylistId),
}

impl Route {
    /// The section the page is open in
    pub(crate) fn section(&self) -> Section {
        match self {
            Self::NowPlaying => Section::NowPlaying,
            Self::Artists | Self::Artist(_) | Self::ArtistAlbum { .. } => Section::Artists,
            Self::Albums | Self::Album(_) => Section::Albums,
            Self::Songs => Section::Songs,
            Self::Playlists | Self::Playlist(_) => Section::Playlists,
        }
    }

    /// The open album — wherever it was opened from
    pub(crate) fn album(&self) -> Option<&AlbumKey> {
        match self {
            Self::Album(album) | Self::ArtistAlbum { album, .. } => Some(album),
            _ => None,
        }
    }

    /// One level up: from an album — to where it was opened from, from a profile — to artists
    pub(crate) fn parent(&self) -> Route {
        match self {
            Self::ArtistAlbum { artist, .. } => Self::Artist(artist.clone()),
            Self::Artist(_) => Self::Artists,
            Self::Album(_) => Self::Albums,
            Self::Playlist(_) => Self::Playlists,
            route => route.clone(),
        }
    }
}

impl From<Section> for Route {
    /// The whole section — the top level, with no artist or album open
    fn from(section: Section) -> Self {
        match section {
            Section::NowPlaying => Self::NowPlaying,
            Section::Artists => Self::Artists,
            Section::Albums => Self::Albums,
            Section::Songs => Self::Songs,
            Section::Playlists => Self::Playlists,
        }
    }
}

/// Which album is open. An album is a "title + artist" pair, as in grouping;
/// names are stored, not an index: album indices change along with the library
#[derive(Clone, PartialEq)]
pub(crate) struct AlbumKey {
    pub(crate) title: String,
    pub(crate) artist: String,
}

impl AlbumKey {
    pub(crate) fn of(album: &AlbumGroup) -> Self {
        Self {
            title: album.title.clone(),
            artist: album.artist.clone(),
        }
    }

    /// Case-insensitive comparison — as in grouping
    pub(crate) fn matches(&self, album: &AlbumGroup) -> bool {
        album.title.to_lowercase() == self.title.to_lowercase()
            && album.artist.to_lowercase() == self.artist.to_lowercase()
    }
}

impl PlayerApp {
    /// Opens `route`. Everything that changes along with the transition is here too:
    /// - search belongs to a section — cleared when the section changes;
    /// - album track selection belongs to the open album — reset when the album changes
    pub(crate) fn navigate(&mut self, route: Route) {
        if route == self.route {
            return;
        }
        if route.section() != self.route.section() && !self.search.is_empty() {
            self.search = Default::default();
            // Songs rows during search are the found tracks: the selection referred to them
            self.library_view.clear_selection();
        }
        if route.album() != self.route.album() {
            self.album_tracks_view.clear_selection();
        }
        // Playlist track selection belongs to the open playlist
        if !matches!((&route, &self.route), (Route::Playlist(a), Route::Playlist(b)) if a == b) {
            self.playlist_tracks_view.clear_selection();
        }
        self.route = route;
    }
}

#[cfg(test)]
mod tests {
    use super::{AlbumKey, Route, Section};

    fn album() -> AlbumKey {
        AlbumKey {
            title: "Brol".into(),
            artist: "Angèle".into(),
        }
    }

    #[test]
    fn album_belongs_to_the_section_it_was_opened_from() {
        let from_profile = Route::ArtistAlbum {
            artist: "Angèle".into(),
            album: album(),
        };
        assert!(from_profile.section() == Section::Artists);
        assert!(Route::Album(album()).section() == Section::Albums);
        assert!(from_profile.album() == Some(&album()));
        assert!(Route::Artist("Angèle".into()).album().is_none());
    }

    #[test]
    fn parent_goes_one_level_up() {
        let from_profile = Route::ArtistAlbum {
            artist: "Angèle".into(),
            album: album(),
        };
        assert!(from_profile.parent() == Route::Artist("Angèle".into()));
        assert!(Route::Artist("Angèle".into()).parent() == Route::Artists);
        assert!(Route::Album(album()).parent() == Route::Albums);
        assert!(Route::Songs.parent() == Route::Songs);
    }
}
