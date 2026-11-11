//! Artists: a grid of all artists and a single artist's profile — header and their albums.

use eframe::egui::{self, Align, Align2, FontId, Label, Rect, RichText, Sense, Ui, vec2};

use super::{
    ARTIST_NAME_SIZE,
    album::ALBUM_PAGE_COVER_SIZE,
    placeholder,
    search::{contains_query, normalized_query},
    shared::{GroupAction, group_action, group_context_menu},
};
use crate::{
    core::artist_images::ArtistImage,
    ui::{
        action::Action,
        app::{ListFocus, PlayerApp},
        components::{self, ButtonStyle, TileLine},
        covers::{Cover, CoverCache, CoverSize},
        dialogs::remove::GroupKind,
        i18n::t,
        navigation::Route,
        style::theme::{HIGHLIGHT, HIGHLIGHT_STRONG, TEXT, TEXT_FAINT, TEXT_MUTED},
    },
};

/// Artist image placeholder: a circle with the first letter of the name. Lightens on hover
pub(crate) fn paint_artist_placeholder(ui: &Ui, rect: Rect, name: &str, hover: f32) {
    let painter = ui.painter();
    painter.circle_filled(
        rect.center(),
        rect.width() / 2.0,
        HIGHLIGHT.lerp_to_gamma(HIGHLIGHT_STRONG, hover),
    );
    let initial: String = name
        .chars()
        .next()
        .into_iter()
        .flat_map(char::to_uppercase)
        .collect();
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        initial,
        FontId::proportional(rect.width() * 0.36),
        TEXT_FAINT,
    );
}

/// Artist image as a circle; while there's none (being looked up or not found) — a letter placeholder
fn paint_artist_image(
    ui: &Ui,
    rect: Rect,
    covers: &mut CoverCache,
    image: ArtistImage,
    name: &str,
    hover: f32,
) {
    let size = if rect.width() > 200.0 {
        CoverSize::Large
    } else {
        CoverSize::Thumbnail
    };
    let cover = match &image {
        ArtistImage::File(path) => covers.get(std::slice::from_ref(path), size),
        ArtistImage::Url(url) => covers.get_url(url, size),
        ArtistImage::Searching | ArtistImage::Missing => Cover::Missing,
    };
    if let Cover::Ready(..) = cover {
        // Rounding is half the side: a circle. Rounding can't exceed 255
        let radius = (rect.width() / 2.0).min(255.0) as u8;
        components::paint_cover(ui, rect, cover, radius);
        return;
    }
    paint_artist_placeholder(ui, rect, name, hover);
}

impl PlayerApp {
    /// Artists as a grid: image (photo, or a letter placeholder until there is one) and name.
    /// Clicking an artist opens their profile
    pub(crate) fn render_artists(&mut self, ui: &mut Ui) {
        self.core.refresh_groups();
        let artists = &self.core.groups.artists;
        if artists.is_empty() {
            placeholder(ui, t().library_empty);
            return;
        }
        // Shown artists — indices into `groups.artists`: search by name
        let query = normalized_query(&self.search.query);
        let shown: Vec<usize> = (0..artists.len())
            .filter(|&index| contains_query(&artists[index].name, &query))
            .collect();
        if shown.is_empty() {
            placeholder(ui, t().no_results);
            return;
        }
        // The name is centered under the round image. The grid borrows the artists —
        // a menu action is remembered and performed after it. Tile indices belong
        // to the library and the query — when they change the selection is reset
        self.artists_grid
            .set_scope(&format!("{}/{query}", self.core.library.revision()));
        let mut action = None;
        // Playlists for the "Add to playlist ›" submenu
        let playlists = self.playlist_entries();
        let (core, covers) = (&mut self.core, &mut self.covers);
        let response = self.artists_grid.show(
            ui,
            shown.len(),
            1,
            Align::Center,
            self.focus == ListFocus::Grid,
            |ui, index, image_rect, tile| {
                // Those shown on screen are looked up first — see `ArtistImages`
                let name = core.groups.artists[shown[index]].name.clone();
                let image = core.artist_image(&name);
                paint_artist_image(ui, image_rect, covers, image, &name, tile.hover);
                if let Some(menu_action) = group_context_menu(tile, GroupKind::Artist, &playlists) {
                    action = Some(menu_action);
                }
                vec![TileLine {
                    text: name,
                    primary: true,
                }]
            },
        );
        if response.interacted {
            self.focus = ListFocus::Grid;
        }
        if response.delete {
            action = Some(GroupAction::Remove);
        }

        if let Some(index) = response.opened {
            self.dispatch(Action::Navigate(Route::Artist(
                self.core.groups.artists[shown[index]].name.clone(),
            )));
        } else if let Some(action) = action {
            // Artists — in grid order, their tracks — album by album
            let names: Vec<String> = self
                .artists_grid
                .selection()
                .iter()
                .map(|&index| self.core.groups.artists[shown[index]].name.clone())
                .collect();
            let tracks: Vec<usize> = names
                .iter()
                .flat_map(|name| self.artist_track_indices(name))
                .collect();
            self.dispatch(group_action(action, GroupKind::Artist, names, tracks));
        }
    }

    /// Artist profile: header and their albums as a grid
    pub(crate) fn render_artist_profile(&mut self, ui: &mut Ui, artist: &str) {
        self.render_artist_header(ui, artist);
        self.render_album_grid(ui, "artist_albums", Some(artist));
    }

    /// Artist profile header: a large name, how many albums and songs, the
    /// "Play" (replace the queue with their tracks and play) and "Add to playlist" buttons
    pub(crate) fn render_artist_header(&mut self, ui: &mut Ui, artist: &str) {
        self.core.refresh_groups();
        let tracks = self.artist_track_indices(artist);
        // Only tracks whose files are present can be played — those are counted
        let playable = tracks
            .iter()
            .filter(|&&index| !self.core.library.is_missing(index))
            .count();
        let album_count = self.artist_albums(artist).count();

        let image = self.core.artist_image(artist);
        let mut play_all = false;
        let mut add_to_queue = false;
        egui::Frame::new()
            .inner_margin(egui::Margin {
                left: 16,
                right: 16,
                top: 0,
                bottom: 20,
            })
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    // Artist photo as a circle — the size of the cover on the album page
                    let (image_rect, _) = ui.allocate_exact_size(
                        vec2(ALBUM_PAGE_COVER_SIZE, ALBUM_PAGE_COVER_SIZE),
                        Sense::hover(),
                    );
                    paint_artist_image(ui, image_rect, &mut self.covers, image, artist, 0.0);
                    ui.add_space(20.0);

                    ui.vertical(|ui| {
                        ui.add(
                            Label::new(RichText::new(artist).size(ARTIST_NAME_SIZE).color(TEXT))
                                .selectable(false)
                                .truncate(),
                        );
                        ui.add(
                            Label::new(
                                RichText::new(format!(
                                    "{} · {}",
                                    t().count(album_count, &t().album_count),
                                    t().count(playable, &t().song_count),
                                ))
                                .size(13.0)
                                .color(TEXT_MUTED),
                            )
                            .selectable(false),
                        );
                        ui.add_space(12.0);

                        // Without tracks (all files gone) the buttons have nothing to do
                        ui.add_enabled_ui(playable > 0, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 8.0;
                                play_all =
                                    components::pill_button(ui, t().play, ButtonStyle::Primary)
                                        .clicked();
                                add_to_queue = components::pill_button(
                                    ui,
                                    t().add_to_queue,
                                    ButtonStyle::Secondary,
                                )
                                .clicked();
                            });
                        });
                    });
                });
            });

        if play_all {
            self.dispatch(Action::PlayLibraryTracks(tracks));
        } else if add_to_queue {
            self.dispatch(Action::AddToQueue(tracks));
        }
    }
}
