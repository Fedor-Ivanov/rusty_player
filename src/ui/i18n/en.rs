//! UI texts in English.

use super::{Plural, PluralForm, Texts};

fn plural_form(count: usize) -> PluralForm {
    if count == 1 {
        PluralForm::One
    } else {
        PluralForm::Many
    }
}

pub static TEXTS: Texts = Texts {
    plural_form,

    library: "Library",
    now_playing: "Now Playing",
    artists: "Artists",
    albums: "Albums",
    songs: "Songs",
    queue: "Queue",
    playlists: "Playlists",
    settings: "Settings",

    track_count: Plural::two("track", "tracks"),
    song_count: Plural::two("song", "songs"),
    album_count: Plural::two("album", "albums"),
    playlist_count: Plural::two("playlist", "playlists"),
    artist_count: Plural::two("artist", "artists"),
    more_file_count: Plural::two("more file", "more files"),

    no_tracks: "No tracks",
    library_empty: "Library is empty",
    no_results: "No results",
    no_albums: "No albums",
    no_playlists: "No playlists",

    search: "Search",
    found_of: "{found} of {tracks}",
    bitrate: "{kbps} kbps",

    open_files: "Open files",
    add_files: "Add files",
    add_folder: "Add folder",
    audio_files: "Audio",
    adding_progress: "Adding {done} of {total}",
    updating_library_progress: "Updating library {done} of {total}",
    scanning_folder: "Scanning folder…",
    no_audio_found: "No supported audio files found",
    more_files_failed: "…and {more} could not be added",

    show_artist: "Show artist",

    go_to_album: "Go to album",

    show_in_library: "Show in library",

    lyrics: "Lyrics",

    no_lyrics: "No lyrics",

    searching_lyrics: "Searching for lyrics…",

    lyrics_unavailable: "Couldn't reach LRCLIB, will try again later",
    lyrics_offline: "Turn on online search in Settings to find lyrics",

    instrumental: "Instrumental",
    show_album: "Show album",
    show_current_track: "Show the current track",
    file_missing: "File not found: it was moved or deleted",
    shuffle: "Shuffle",
    repeat_all: "Repeat all",
    repeat_one: "Repeat one",

    play: "Play",
    add_to_queue: "Add to queue",
    remove_from_library: "Remove from library",
    remove_from_queue: "Remove from queue",
    play_items: "Play {items}",
    add_items_to_queue: "Add {items} to queue",
    remove_items_from_library: "Remove {items} from library",
    remove_items_from_queue: "Remove {items} from queue",
    remove: "Remove",
    cancel: "Cancel",
    ok: "OK",
    done: "Done",

    save_queue: "Save as playlist",
    save_queue_to: "Save to \u{201C}{playlist}\u{201D}",
    save_as_new_playlist: "Save as new playlist…",
    clear_queue: "Clear queue",
    add_to_playlist: "Add to playlist",
    new_playlist: "New playlist…",
    remove_from_this_playlist: "Remove from playlist",
    add_to_library: "Add to library",
    add_items_to_library: "Add {items} to library",
    remove_items_from_this_playlist: "Remove {items} from playlist",
    rename: "Rename",
    delete: "Delete",
    save: "Save",
    save_playlist_title: "Save queue as playlist",
    new_playlist_title: "New playlist",
    rename_playlist_title: "Rename playlist",
    playlist_name_hint: "Playlist name",
    delete_playlist_title: "Delete playlist?",
    delete_playlist_message: "\u{201C}{playlist}\u{201D} will be deleted. \
                              The tracks themselves won't be removed.",
    delete_playlists_title: "Delete {playlists}?",
    delete_playlists_message: "{playlists} will be deleted. The tracks themselves won't be removed.",
    delete_items: "Delete {items}",

    language: "Language",
    version: "Version",
    online_search: "Online search",
    online_search_note: "Artist photos from Deezer and lyrics from LRCLIB. Artist names and track titles are sent to these services.",
    glass_tint: "Color",
    transparency: "Transparency",
    tint_graphite: "Graphite",
    tint_blue: "Blue",
    tint_violet: "Violet",
    tint_green: "Green",
    tint_rose: "Rose",
    tint_amber: "Amber",

    remove_album_title: "Remove album from library?",
    remove_album_message: "\u{201C}{album}\u{201D} will be removed from your library. \
                           The files will stay on your disk.",
    remove_albums_title: "Remove {albums} from library?",
    remove_albums_message: "{albums} ({tracks}) will be removed from your library. \
                            The files will stay on your disk.",
    remove_artist_title: "Remove artist from library?",
    remove_artist_message: "All tracks by \u{201C}{artist}\u{201D} will be removed from your \
                            library. The files will stay on your disk.",
    remove_artists_title: "Remove {artists} from library?",
    remove_artists_message: "All tracks by {artists} ({tracks}) will be removed from your \
                             library. The files will stay on your disk.",
    remove_song_title: "Remove song from library?",
    remove_song_message: "\u{201C}{song}\u{201D} will be removed from your library. \
                          The file will stay on your disk.",
    remove_songs_title: "Remove {songs} from library?",
    remove_songs_message: "{songs} will be removed from your library. \
                           The files will stay on your disk.",
};
