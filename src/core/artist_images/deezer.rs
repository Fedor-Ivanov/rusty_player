//! Artist photos from Deezer: artist search by name is an open API, no key needed.
//! Only the photo link comes from here: storing Deezer photos themselves is not allowed.

use serde::Deserialize;

use crate::core::net;

const SEARCH_URL: &str = "https://api.deezer.com/search/artist";
/// How many artists to request from search: there can be several namesakes
const SEARCH_LIMIT: &str = "10";

#[derive(Deserialize)]
struct Search {
    #[serde(default)]
    data: Vec<Artist>,
    /// Deezer reports errors (e.g. rate limit exceeded) with status 200
    error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct Artist {
    name: String,
    /// Fan count — used to pick the well-known one among namesakes
    #[serde(default)]
    nb_fan: u64,
    /// 500×500
    picture_big: Option<String>,
}

/// Link to the photo of artist `name`: `Ok(None)` — Deezer has no such artist or
/// the artist has no photo. The photo itself is not downloaded: it is loaded from the link when
/// needed on screen. Error — network or service failure: can ask again
pub fn find_picture(name: &str) -> Result<Option<String>, String> {
    let mut response = net::agent()
        .get(SEARCH_URL)
        .query("q", name)
        .query("limit", SEARCH_LIMIT)
        .call()
        .map_err(|e| format!("Could not reach Deezer: {e}"))?;
    if response.status() != 200 {
        return Err(format!("Deezer answered with HTTP {}", response.status()));
    }
    let search: Search = response
        .body_mut()
        .read_json()
        .map_err(|e| format!("Deezer sent an unexpected answer: {e}"))?;
    if let Some(error) = search.error {
        return Err(format!("Deezer answered with an error: {error}"));
    }
    Ok(best_picture(search.data, name))
}

/// Photo link of the artist with the same name (case-insensitive); among namesakes —
/// the most famous one. Similar names won't do: a placeholder is better than someone else's face
fn best_picture(artists: Vec<Artist>, name: &str) -> Option<String> {
    let name = name.trim().to_lowercase();
    artists
        .into_iter()
        .filter(|artist| artist.name.trim().to_lowercase() == name)
        .filter_map(|artist| {
            let url = artist.picture_big?;
            // Without a photo, Deezer returns a placeholder link with an empty image id
            (!url.contains("/artist//")).then_some((artist.nb_fan, url))
        })
        .max_by_key(|(fans, _)| *fans)
        .map(|(_, url)| url)
}

#[cfg(test)]
mod tests {
    use super::{Artist, best_picture};

    fn artist(name: &str, fans: u64, picture: &str) -> Artist {
        Artist {
            name: name.into(),
            nb_fan: fans,
            picture_big: Some(picture.into()),
        }
    }

    #[test]
    fn picks_most_known_namesake() {
        let artists = vec![
            artist("HIM", 68, "https://img/artist/a/500.jpg"),
            artist("HIM", 210_403, "https://img/artist/b/500.jpg"),
            artist("H.I.M.", 1_000_000, "https://img/artist/c/500.jpg"),
            // The most famous one, but without a photo
            artist("him", 5_000_000, "https://img/artist//500.jpg"),
        ];
        assert_eq!(
            best_picture(artists, "HIM").as_deref(),
            Some("https://img/artist/b/500.jpg")
        );
        let ghost = vec![artist("Ghost", 614_630, "https://img/artist/g/500.jpg")];
        assert_eq!(best_picture(ghost, "Ghost B.C."), None);
    }
}
