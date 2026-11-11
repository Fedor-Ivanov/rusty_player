//! Central area: current track info and cover, without tracks — "Open files";
//! the current track bar while a library view is open.

use eframe::egui::{
    self, Align, Align2, CursorIcon, FontId, Label, Layout, Rect, RichText, ScrollArea, Sense,
    Shape, Ui, UiBuilder, pos2, vec2,
};

use crate::{
    core::library::{
        Track, groups,
        metadata::{FileMetadata, get_file_metadata},
    },
    ui::action::Action,
    ui::{
        app::{ADD_BUTTON_SIZE, PlayerApp},
        components,
        components::format_secs,
        covers::CoverSize,
        i18n::{fill, t},
        loading::Destination,
        navigation::{AlbumKey, Route},
        style::icons::paint_lyrics,
        style::theme::{COVER_CROSSFADE_SECS, TEXT, TEXT_FAINT, TEXT_MUTED},
    },
};
use std::path::PathBuf;

const COVER_MAX_SIZE: f32 = 320.0;
/// Track details text size
const DETAILS_SIZE: f32 = 16.0;
const COVER_RADIUS: u8 = 12;
/// Lyrics: line size and the gap between lines
const LYRICS_SIZE: f32 = 20.0;
const LYRICS_LINE_GAP: f32 = 10.0;
/// Lyrics offset from the track details above — larger than the cover's: otherwise lyrics lines
/// blend into the details
const LYRICS_TOP_GAP: f32 = 16.0;
/// How far the lyrics slide up from below when appearing instead of the cover
const LYRICS_SLIDE: f32 = 12.0;
/// Minimum gap between the track title and the lyrics button to its right
const LYRICS_BUTTON_GAP: f32 = 8.0;
/// The backdrop starts dissolving this far above the bottom of the central area
const BACKDROP_FADE_ABOVE: f32 = 120.0;

impl PlayerApp {
    /// A button centered in all the remaining space
    pub(crate) fn render_add_tracks_block(&mut self, ui: &mut Ui) {
        let text = RichText::new(t().open_files).size(18.0).color(TEXT);

        // Layout in egui doesn't center vertically, so we compute the button size
        // in advance and place it in the center of the free area ourselves
        let text_size = egui::WidgetText::from(text.clone())
            .into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                egui::TextStyle::Button,
            )
            .size();
        let button_size = text_size + 2.0 * ui.spacing().button_padding;
        let button_rect =
            Rect::from_center_size(ui.available_rect_before_wrap().center(), button_size);

        let open = egui::Button::new(text).frame(false);
        if ui
            .put(button_rect, open)
            .on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text(t().open_files)
            .clicked()
        {
            self.dispatch(Action::AddFiles(Destination::Queue));
        }
    }

    /// Current track details backdrop — its blurred cover — under the central area `ui`
    /// and below, under the controls panel down to the bottom of the window. At the bottom of the central area it starts
    /// dissolving and vanishes by the bottom of the window: no sharp border with the controls panel.
    /// On track change the old backdrop smoothly changes to the new one (see `CoverFade`). Returns
    /// a shape: it's placed under the panels (see `ui`), not on top of them
    pub(crate) fn track_backdrop(&mut self, ui: &Ui) -> Option<Shape> {
        let track = self.core.controller.player().current_track.clone()?;
        let candidates = self.cover_candidates(&track);
        let cover = self.covers.get(&candidates, CoverSize::Backdrop);
        let progress = self.backdrop_fade.update(ui, cover);
        let content = ui.max_rect();
        let rect = Rect::from_min_max(
            content.min,
            pos2(content.right(), ui.ctx().content_rect().bottom()),
        );
        self.backdrop_fade
            .backdrop(rect, content.bottom() - BACKDROP_FADE_ABOVE, progress)
    }

    /// Where to look for the track's cover: first in the track itself (compilation tracks may have
    /// different images), then in the other tracks of its album — not all have a cover
    pub(crate) fn cover_candidates(&mut self, track: &Track) -> Vec<PathBuf> {
        let mut candidates = vec![track.path.clone()];
        if let Some(album) = self.core.album_of(track) {
            candidates.extend(album.cover_candidates.iter().cloned());
        }
        candidates
    }

    /// Current track details: title, artist and album (links — if they're
    /// in the library), year and genre, duration and bitrate; below — the cover centered
    /// in the free space
    pub(crate) fn render_current_track_info(&mut self, ui: &mut Ui) {
        let Some(track) = self.core.controller.player().current_track.clone() else {
            return;
        };
        // The track's artist and album in the library — looked up the same way as in grouping
        let album_in_library = self.core.album_of(&track).map(AlbumKey::of);
        // The link leads to the album artist: "Angèle; Kiddy Smile" — to Angèle
        let artist_key = groups::album_artist(&track).to_lowercase();
        let artist_in_library = self
            .core
            .groups
            .artists
            .iter()
            .find(|artist| artist.name.to_lowercase() == artist_key)
            .map(|artist| artist.name.clone());
        let cover_candidates = self.cover_candidates(&track);
        let details = self.current_details(&track);
        // "2006 · Instrumental Rock" — whichever of these exists. The year — from the file, and if the file
        // couldn't be read — from the library
        let year = details
            .as_ref()
            .and_then(|details| details.year)
            .or(track.year);
        let genre = details.as_ref().map(|details| details.genre.trim());
        let year_and_genre: Vec<String> = year
            .map(|year| year.to_string())
            .into_iter()
            .chain(genre.filter(|genre| !genre.is_empty()).map(str::to_owned))
            .collect();
        // "4:02 · 320 kbps"
        let mut length = format_secs(track.duration_secs as f32);
        if let Some(kbps) = details.as_ref().and_then(|details| details.bitrate_kbps) {
            length += " · ";
            length += &fill(t().bitrate, &[("kbps", &kbps.to_string())]);
        }

        let mut open_artist = false;
        let mut open_album = false;
        let mut seek_to = None;
        let mut toggle_lyrics = false;
        ui.vertical(|ui| {
            // The title — on the left, wrapping without going under the lyrics button on the right.
            // The button — centered on the first line of the title
            let full_width = ui.available_width();
            let title = ui
                .scope(|ui| {
                    ui.set_max_width(full_width - ADD_BUTTON_SIZE - LYRICS_BUTTON_GAP);
                    ui.label(RichText::new(&track.title).size(DETAILS_SIZE).color(TEXT))
                })
                .inner;
            let first_line =
                ui.fonts_mut(|fonts| fonts.row_height(&FontId::proportional(DETAILS_SIZE)));
            let button_rect = Rect::from_center_size(
                pos2(
                    title.rect.left() + full_width - ADD_BUTTON_SIZE / 2.0,
                    title.rect.top() + first_line / 2.0,
                ),
                vec2(ADD_BUTTON_SIZE, ADD_BUTTON_SIZE),
            );
            let (clicked, active, hover) =
                components::toggle_button(ui, button_rect, "lyrics", self.lyrics_open, t().lyrics);
            paint_lyrics(ui, button_rect, active, hover);
            toggle_lyrics = clicked;

            if !track.artist.is_empty() {
                if artist_in_library.is_some() {
                    open_artist =
                        components::text_link(ui, &track.artist, DETAILS_SIZE, TEXT_MUTED)
                            .on_hover_text(t().show_artist)
                            .clicked();
                } else {
                    // Not from the library — the same look, just without navigation
                    components::text_line(ui, &track.artist, DETAILS_SIZE, TEXT_MUTED);
                }
            }
            if !track.album.is_empty() {
                if album_in_library.is_some() {
                    open_album = components::text_link(ui, &track.album, DETAILS_SIZE, TEXT_MUTED)
                        .on_hover_text(t().show_album)
                        .clicked();
                } else {
                    components::text_line(ui, &track.album, DETAILS_SIZE, TEXT_MUTED);
                }
            }
            if !year_and_genre.is_empty() {
                components::text_line(ui, &year_and_genre.join(" · "), DETAILS_SIZE, TEXT_MUTED);
            }
            components::text_line(ui, &length, DETAILS_SIZE, TEXT_MUTED);
            ui.add_space(8.0);

            // Below — all the remaining space: lyrics or the cover. When toggled they
            // smoothly replace each other, and the lyrics slide up slightly from below
            let free = ui.available_rect_before_wrap();
            ui.allocate_rect(free, Sense::hover());
            let lyrics_shown = ui.ctx().animate_bool_with_time_and_easing(
                egui::Id::unique("lyrics_shown"),
                self.lyrics_open,
                COVER_CROSSFADE_SECS,
                egui::emath::easing::cubic_in_out,
            );
            if lyrics_shown > 0.0 {
                seek_to = ui
                    .scope(|ui| {
                        ui.multiply_opacity(lyrics_shown);
                        let slide = vec2(0.0, (1.0 - lyrics_shown) * LYRICS_SLIDE);
                        self.render_lyrics(ui, free.translate(slide), &track)
                    })
                    .inner;
            }
            if lyrics_shown < 1.0 {
                ui.scope(|ui| {
                    ui.multiply_opacity(1.0 - lyrics_shown);
                    // The cover — centered horizontally and vertically; in a small window
                    // it shrinks to fit
                    let side = free.width().min(free.height()).clamp(0.0, COVER_MAX_SIZE);
                    // While the new track's cover is being read, the old one is visible — see `CoverFade`
                    let cover = self.covers.get(&cover_candidates, CoverSize::Large);
                    let progress = self.cover_fade.update(ui, cover);
                    self.cover_fade.paint(
                        ui,
                        Rect::from_center_size(free.center(), vec2(side, side)),
                        progress,
                        COVER_RADIUS,
                    );
                });
            }
        });

        if toggle_lyrics {
            self.dispatch(Action::ToggleLyrics);
        }
        if let Some(secs) = seek_to {
            self.dispatch(Action::Seek(secs));
        }
        if open_artist && let Some(artist) = artist_in_library {
            self.dispatch(Action::Navigate(Route::Artist(artist)));
        } else if open_album && let Some(album) = album_in_library {
            self.dispatch(Action::Navigate(Route::Album(album)));
        }
    }

    /// Lyrics in `area`. Synced: the current line is brighter than the rest, the lyrics
    /// scroll to follow it, clicking a line seeks to it — then returns where to
    /// (in seconds). Plain — just text. No lyrics — a placeholder
    fn render_lyrics(&mut self, ui: &mut Ui, area: Rect, track: &Track) -> Option<f32> {
        let area = area.with_min_y(area.top() + LYRICS_TOP_GAP);
        if !matches!(&self.current_lyrics, Some((path, _)) if *path == track.path) {
            self.current_lyrics = Some((track.path.clone(), self.core.lyrics.find(track)));
        }
        let Some((_, found)) = &self.current_lyrics else {
            return None;
        };
        let Some(lyrics) = &found.lyrics else {
            let message = if found.searching {
                t().searching_lyrics
            } else if found.instrumental {
                t().instrumental
            } else if found.unavailable {
                t().lyrics_unavailable
            } else if found.offline {
                t().lyrics_offline
            } else {
                t().no_lyrics
            };
            ui.painter().text(
                area.center(),
                Align2::CENTER_CENTER,
                message,
                FontId::proportional(DETAILS_SIZE),
                TEXT_FAINT,
            );
            return None;
        };

        let current = lyrics.current_line(self.core.controller.position_ms());
        let scroll_to_current = current.is_some() && current != self.lyrics_line;
        self.lyrics_line = current;
        let mut seek_to = None;
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(area)
                .layout(Layout::top_down(Align::Min)),
        );
        // Each track has its own scroll: new lyrics start at the top
        ScrollArea::vertical()
            .id_salt(("lyrics", &track.path))
            .auto_shrink(false)
            .show(&mut child, |ui| {
                for (index, line) in lyrics.lines.iter().enumerate() {
                    // An empty line: in synced lyrics — an instrumental break, in plain ones —
                    // a gap between verses
                    if line.text.is_empty() && !lyrics.synced {
                        ui.add_space(LYRICS_SIZE);
                        continue;
                    }
                    let text = if line.text.is_empty() {
                        "♪"
                    } else {
                        &line.text
                    };
                    // Synced: the line being sung is bright, the rest are pale, so
                    // the current one is visible at once even on a light cover backdrop
                    let color = if !lyrics.synced || current == Some(index) {
                        TEXT
                    } else {
                        TEXT_FAINT
                    };
                    let mut label =
                        Label::new(RichText::new(text).size(LYRICS_SIZE).color(color)).wrap();
                    if lyrics.synced {
                        label = label.sense(Sense::click());
                    }
                    let response = ui.add(label);
                    if let Some(start) = line.start_ms {
                        let response = response.on_hover_cursor(CursorIcon::PointingHand);
                        if response.clicked() {
                            seek_to = Some(start as f32 / 1000.0);
                        }
                        if scroll_to_current && current == Some(index) {
                            response.scroll_to_me(Some(Align::Center));
                        }
                    }
                    ui.add_space(LYRICS_LINE_GAP);
                }
                // Bottom padding: the last lines also reach the middle
                if lyrics.synced {
                    ui.add_space(area.height() / 2.0);
                }
            });
        seek_to
    }

    /// Current track file tags: read only when the track changed
    fn current_details(&mut self, track: &Track) -> Option<FileMetadata> {
        match &self.current_details {
            Some((path, details)) if *path == track.path => details.clone(),
            _ => {
                let details = get_file_metadata(&track.path).ok();
                self.current_details = Some((track.path.clone(), details.clone()));
                details
            }
        }
    }

    /// The current track bar at the bottom of the central area. Returns `true` if it was clicked
    pub(crate) fn render_now_playing_bar(&mut self, ui: &mut Ui) -> bool {
        let Some(track) = &self.core.controller.player().current_track else {
            return false;
        };
        components::now_playing_bar(ui, &track.title, &track.artist)
            .on_hover_text(t().show_current_track)
            .clicked()
    }
}
