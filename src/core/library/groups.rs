//! Grouping library tracks by artists and albums — for the "Artists" and "Albums" views.

use std::{collections::HashMap, path::PathBuf};

use super::Track;

pub const UNKNOWN_ARTIST: &str = "Unknown Artist";
pub const UNKNOWN_ALBUM: &str = "Unknown Album";

/// A track's album artist — tracks are grouped into albums and artists by it. The
/// Album Artist tag, or if missing — the track's primary artist (see `primary_artist`): tracks
/// with featured artists ("Angèle; Kiddy Smile") are not torn away from their album
pub fn album_artist(track: &Track) -> String {
    let tagged = track.album_artist.trim();
    let artist = if tagged.is_empty() {
        primary_artist(&track.artist)
    } else {
        tagged
    };
    if artist.is_empty() {
        UNKNOWN_ARTIST
    } else {
        artist
    }
    .to_owned()
}

/// The track has a real album: the tags have both the album title and the artist. Without them the track
/// goes into "Unknown Album" together with all similar ones — these tracks have nothing in common, and
/// taking a cover from such an "album" for a track is wrong: it would belong to someone else
pub fn has_known_album(track: &Track) -> bool {
    !track.album.trim().is_empty() && album_artist(track) != UNKNOWN_ARTIST
}

/// Primary artist from the artist field: up to ";" (how multiple artists are written)
/// and up to "feat.", "ft.", "featuring". Comma and "&" are not separators — they occur in names:
/// "Earth, Wind & Fire", "Simon & Garfunkel"
fn primary_artist(artist: &str) -> &str {
    const MARKERS: [&str; 6] = [" feat.", " feat ", " ft.", " ft ", " featuring ", "(feat"];
    // Separators are ASCII: in the ASCII-lowercased string they are at the same positions as
    // in the original
    let lower = artist.to_ascii_lowercase();
    let end = MARKERS
        .iter()
        .filter_map(|marker| lower.find(marker))
        .chain(artist.find(';'))
        .min()
        .unwrap_or(artist.len());
    artist[..end].trim()
}

pub struct ArtistGroup {
    pub name: String,
    /// Track indices in the library
    pub tracks: Vec<usize>,
}

pub struct AlbumGroup {
    pub title: String,
    pub artist: String,
    /// The earliest year among the album's tracks
    pub year: Option<u16>,
    /// Files whose tags to search for the album cover — all its tracks in album order.
    /// The cover is stored in each file's tags separately and not all have it: the first one
    /// found is used. "Unknown …" (tracks without album or artist in tags, see
    /// `has_known_album`) has none: one of these tracks' cover is not the cover of the others,
    /// a placeholder is used instead
    pub cover_candidates: Vec<PathBuf>,
    /// Track indices in the library — in album order: by disc and track number
    pub tracks: Vec<usize>,
}

/// Library artists and albums, alphabetically, case-insensitive.
/// An album is a "title + artist" pair: same-named albums by different artists
/// are not merged into one
pub fn group(tracks: &[Track]) -> (Vec<ArtistGroup>, Vec<AlbumGroup>) {
    let name_or = |text: &str, unknown: &str| {
        let text = text.trim();
        if text.is_empty() { unknown } else { text }.to_owned()
    };

    let mut artists: HashMap<String, ArtistGroup> = HashMap::new();
    let mut albums: HashMap<(String, String), AlbumGroup> = HashMap::new();
    for (index, track) in tracks.iter().enumerate() {
        let artist = album_artist(track);
        let album = name_or(&track.album, UNKNOWN_ALBUM);

        artists
            .entry(artist.to_lowercase())
            .or_insert_with(|| ArtistGroup {
                name: artist.clone(),
                tracks: Vec::new(),
            })
            .tracks
            .push(index);

        let group = albums
            .entry((album.to_lowercase(), artist.to_lowercase()))
            .or_insert_with(|| AlbumGroup {
                title: album,
                artist,
                year: None,
                cover_candidates: Vec::new(),
                tracks: Vec::new(),
            });
        group.tracks.push(index);
        group.year = match (group.year, track.year) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
    }

    let mut artists: Vec<ArtistGroup> = artists.into_values().collect();
    artists.sort_by_cached_key(|a| a.name.to_lowercase());
    let mut albums: Vec<AlbumGroup> = albums.into_values().collect();
    // Album tracks — by disc number, then by track number. Without a number — at the end of the disc,
    // in library order; without a disc number, treated as the first disc
    for album in &mut albums {
        album.tracks.sort_by_key(|&index| {
            let track = &tracks[index];
            (
                track.disc_number.unwrap_or(1),
                track.track_number.unwrap_or(u32::MAX),
                index,
            )
        });
        // Album tracks share the album title and artist — the first one is enough
        let known = album
            .tracks
            .first()
            .is_some_and(|&index| has_known_album(&tracks[index]));
        album.cover_candidates = if known {
            album
                .tracks
                .iter()
                .map(|&index| tracks[index].path.clone())
                .collect()
        } else {
            Vec::new()
        };
    }
    albums.sort_by_cached_key(|a| (a.title.to_lowercase(), a.artist.to_lowercase()));
    (artists, albums)
}

#[cfg(test)]
mod tests {
    use super::{UNKNOWN_ALBUM, UNKNOWN_ARTIST, group, has_known_album};
    use crate::core::library::Track;
    use std::path::PathBuf;

    fn numbered(name: &str, disc: Option<u32>, number: Option<u32>) -> Track {
        Track {
            disc_number: disc,
            track_number: number,
            ..track(name, "Band", "Album", None)
        }
    }

    #[test]
    fn album_tracks_follow_disc_and_track_numbers() {
        let tracks = [
            numbered("d2-1", Some(2), Some(1)),
            numbered("no number", None, None),
            numbered("d1-2", Some(1), Some(2)),
            numbered("d1-1", None, Some(1)),
        ];
        let (_, albums) = group(&tracks);
        let order: Vec<_> = albums[0]
            .tracks
            .iter()
            .map(|&i| tracks[i].title.as_str())
            .collect();
        // No disc number — first disc; no track number — at the end of its disc
        assert_eq!(order, ["d1-1", "d1-2", "no number", "d2-1"]);
        // The album cover is searched in its tracks in the same order
        let covers: Vec<_> = albums[0]
            .cover_candidates
            .iter()
            .map(|path| path.to_str().unwrap())
            .collect();
        assert_eq!(
            covers,
            ["d1-1.mp3", "d1-2.mp3", "no number.mp3", "d2-1.mp3"]
        );
    }

    fn track(name: &str, artist: &str, album: &str, year: Option<u16>) -> Track {
        Track {
            path: PathBuf::from(format!("{name}.mp3")),
            title: name.into(),
            artist: artist.into(),
            duration_secs: 0,
            album: album.into(),
            year,
            track_number: None,
            disc_number: None,
            album_artist: String::new(),
        }
    }

    #[test]
    fn only_tagged_albums_are_known() {
        assert!(has_known_album(&track("a", "Band", "Album", None)));
        // No album or no artist — "Unknown …": no cover is taken from it
        assert!(!has_known_album(&track("b", "Band", " ", None)));
        assert!(!has_known_album(&track("c", "", "Album", None)));
        assert!(!has_known_album(&track("d", "", "", None)));
    }

    #[test]
    fn featured_artists_stay_in_album() {
        let tracks = [
            track("1", "Angèle", "Brol", Some(2018)),
            track("2", "Angèle; Romeo Elvis", "Brol", Some(2018)),
            track("3", "Tricky Feat. Anthony Kiedis", "Blowback", Some(2001)),
            track("4", "Earth, Wind & Fire", "Gratitude", None),
            // The Album Artist tag takes precedence over the track artist
            Track {
                album_artist: "Various Artists".into(),
                ..track("5", "Muse", "Hits", None)
            },
            Track {
                album_artist: "Various Artists".into(),
                ..track("6", "Adele", "Hits", None)
            },
        ];
        let (artists, albums) = group(&tracks);

        let albums: Vec<_> = albums
            .iter()
            .map(|a| (a.title.as_str(), a.artist.as_str(), a.tracks.len()))
            .collect();
        assert_eq!(
            albums,
            [
                ("Blowback", "Tricky", 1),
                ("Brol", "Angèle", 2),
                ("Gratitude", "Earth, Wind & Fire", 1),
                ("Hits", "Various Artists", 2),
            ]
        );
        let names: Vec<_> = artists.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(
            names,
            ["Angèle", "Earth, Wind & Fire", "Tricky", "Various Artists"]
        );
    }

    #[test]
    fn groups_by_artist_and_album() {
        let tracks = [
            track("1", "Muse", "Origin of Symmetry", Some(2001)),
            track("2", "muse", "Origin of Symmetry", Some(2002)),
            track("3", "Adele", "21", None),
            track("4", "Adele", "Greatest Hits", Some(2015)),
            track("5", "Björk", "Greatest Hits", Some(2002)),
            track("6", "", "", None),
        ];
        let (artists, albums) = group(&tracks);

        let names: Vec<_> = artists.iter().map(|a| a.name.as_str()).collect();
        // Case-insensitive, the name is taken from the first track encountered
        assert_eq!(names, ["Adele", "Björk", "Muse", UNKNOWN_ARTIST]);
        assert_eq!(artists[2].tracks, [0, 1]);

        let albums: Vec<_> = albums
            .iter()
            .map(|a| (a.title.as_str(), a.artist.as_str(), a.year, a.tracks.len()))
            .collect();
        assert_eq!(
            albums,
            [
                ("21", "Adele", None, 1),
                // Same-named albums by different artists are different albums
                ("Greatest Hits", "Adele", Some(2015), 1),
                ("Greatest Hits", "Björk", Some(2002), 1),
                // The album year is the earliest of its tracks
                ("Origin of Symmetry", "Muse", Some(2001), 2),
                (UNKNOWN_ALBUM, UNKNOWN_ARTIST, None, 1),
            ]
        );
    }

    #[test]
    fn unknown_albums_have_no_cover() {
        let tracks = [
            track("1", "Band", "Album", None),
            track("2", "", "", None),
            track("3", "", "", None),
            track("4", "Band", "", None),
        ];
        let (_, albums) = group(&tracks);
        let covers: Vec<_> = albums
            .iter()
            .map(|a| {
                (
                    a.title.as_str(),
                    a.artist.as_str(),
                    a.cover_candidates.len(),
                )
            })
            .collect();
        // Untagged tracks are in one "album", but no cover is searched for it: placeholder
        assert_eq!(
            covers,
            [
                ("Album", "Band", 1),
                (UNKNOWN_ALBUM, "Band", 0),
                (UNKNOWN_ALBUM, UNKNOWN_ARTIST, 0),
            ]
        );
    }
}
