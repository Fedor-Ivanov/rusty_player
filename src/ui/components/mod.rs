//! Reusable UI elements.

mod banners;
mod buttons;
mod cover;
mod dialog;
mod format;
mod grid;
mod layout;
mod menu;
mod nav;
mod search;
mod select;
mod selection;
mod slider;
mod switch;
mod track_list;

pub use banners::{BANNER_GAP, BANNER_TOP, error_banner, loading_banner};
pub use buttons::{ButtonStyle, HitShape, icon_button, pill_button, toggle_button};
pub use cover::{CoverFade, paint_cover};
pub use dialog::{Confirm, confirm_dialog, modal};
pub use format::format_secs;
pub use grid::{Tile, TileGridView, TileLine};
pub use layout::{
    block_title, block_title_with_note, breadcrumb_title, paint_resize_line, text_line, text_link,
    title_button,
};
pub use menu::{menu_item, menu_style, submenu};
pub use nav::{NOW_PLAYING_BAR_HEIGHT, nav_item, now_playing_bar};
pub use search::title_with_search;
pub use select::select;
pub use slider::{paint_slider, slider};
pub use switch::switch;
pub use track_list::{RowState, TrackListView};
