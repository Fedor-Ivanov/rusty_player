//! Albums as a grid — all of them or one artist's (in their profile).

use eframe::egui::{Align, Ui};

use super::{
    TILE_RADIUS, placeholder,
    search::{contains_query, normalized_query},
    shared::{GroupAction, Target, group_action, group_context_menu, overlay_play_button},
};
use crate::{
    core::library::groups::AlbumGroup,
    ui::action::Action,
    ui::{
        app::{ListFocus, PlayerApp},
        components::{self, TileLine},
        covers::CoverSize,
        dialogs::remove::GroupKind,
        i18n::t,
        navigation::{AlbumKey, Route},
    },
};

impl PlayerApp {
    /// All library albums
    pub(crate) fn render_albums(&mut self, ui: &mut Ui) {
        self.render_album_grid(ui, "albums", None);
    }

    /// Albums as a grid: cover, title, artist and year. With `artist` — only albums
    /// of this artist (name case-insensitive, as in grouping)
    pub(crate) fn render_album_grid(&mut self, ui: &mut Ui, id_salt: &str, artist: Option<&str>) {
        self.core.refresh_groups();
        // Whose profile — for opening an album from it; then the name is needed for comparison
        let profile = artist.map(str::to_owned);
        let artist = artist.map(str::to_lowercase);
        // Search by title — only in all albums, the artist profile has no field
        let query = match artist {
            Some(_) => String::new(),
            None => normalized_query(&self.search.query),
        };
        let albums: Vec<&AlbumGroup> = self
            .core
            .groups
            .albums
            .iter()
            .filter(|album| {
                artist
                    .as_ref()
                    .is_none_or(|artist| album.artist.to_lowercase() == *artist)
                    && contains_query(&album.title, &query)
            })
            .collect();
        // Playlists for the "Add to playlist ›" submenu
        let playlists = self.playlist_entries();
        let covers = &mut self.covers;
        if albums.is_empty() {
            let text = if artist.is_some() {
                t().no_albums
            } else if self.core.groups.albums.is_empty() {
                t().library_empty
            } else {
                t().no_results
            };
            placeholder(ui, text);
            return;
        }
        // Each grid has its own selection. It belongs to the content: in an artist profile —
        // to their albums, in search — to the found ones; after a library change tile indices
        // mean something else
        let scope = format!(
            "{}/{}/{query}",
            artist.as_deref().unwrap_or_default(),
            self.core.library.revision()
        );
        let grid = if id_salt == "artist_albums" {
            &mut self.artist_albums_grid
        } else {
            &mut self.albums_grid
        };
        grid.set_scope(&scope);

        // The grid borrows the albums and the cover cache — the action is remembered and performed after it.
        // A tile click opens the album; playback — via the button on the cover and from the menu
        let mut action = None;
        let response = grid.show(
            ui,
            albums.len(),
            3,
            Align::Min,
            self.focus == ListFocus::Grid,
            |ui, index, image_rect, tile| {
                let album = albums[index];
                let cover = covers.get(&album.cover_candidates, CoverSize::Thumbnail);
                components::paint_cover(ui, image_rect, cover, TILE_RADIUS);

                // A play button in the top left corner of the cover — fades in on tile hover
                // and plays only this album, regardless of the selection
                // The play button on the cover plays only this album, regardless of the selection
                if overlay_play_button(ui, image_rect, tile, &format!("play_album {index}")) {
                    action = Some((GroupAction::Play, Target::One(index)));
                }

                if let Some(menu_action) = group_context_menu(tile, GroupKind::Album, &playlists) {
                    action = Some((menu_action, Target::Selection));
                }

                vec![
                    TileLine {
                        text: album.title.clone(),
                        primary: true,
                    },
                    TileLine {
                        text: album.artist.clone(),
                        primary: false,
                    },
                    TileLine {
                        text: album.year.map(|year| year.to_string()).unwrap_or_default(),
                        primary: false,
                    },
                ]
            },
        );
        // Rows next to the screen — in advance: covers will be ready when scrolling
        for index in grid.nearby() {
            covers.prefetch(&albums[index].cover_candidates, CoverSize::Thumbnail);
        }
        if response.interacted {
            self.focus = ListFocus::Grid;
        }
        if response.delete {
            action = Some((GroupAction::Remove, Target::Selection));
        }

        if let Some(index) = response.opened {
            let album = AlbumKey::of(albums[index]);
            self.dispatch(Action::Navigate(match profile {
                Some(artist) => Route::ArtistAlbum { artist, album },
                None => Route::Album(album),
            }));
        } else if let Some((action, target)) = action {
            // Albums — in grid order, their tracks — album by album
            let targets: Vec<usize> = match target {
                Target::One(index) => vec![index],
                Target::Selection => {
                    let grid = if id_salt == "artist_albums" {
                        &self.artist_albums_grid
                    } else {
                        &self.albums_grid
                    };
                    grid.selection().iter().copied().collect()
                }
            };
            let titles = targets.iter().map(|&i| albums[i].title.clone()).collect();
            let tracks: Vec<usize> = targets
                .iter()
                .flat_map(|&i| albums[i].tracks.iter().copied())
                .collect();
            self.dispatch(group_action(action, GroupKind::Album, titles, tracks));
        }
    }
}
