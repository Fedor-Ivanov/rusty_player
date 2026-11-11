//! UI texts in French.

use super::{Plural, PluralForm, Texts};

/// Unlike English, zero is singular too: "0 piste", "1 piste", "2 pistes"
fn plural_form(count: usize) -> PluralForm {
    if count <= 1 {
        PluralForm::One
    } else {
        PluralForm::Many
    }
}

pub static TEXTS: Texts = Texts {
    plural_form,

    library: "Bibliothèque",
    now_playing: "Lecture en cours",
    artists: "Artistes",
    albums: "Albums",
    songs: "Morceaux",
    queue: "File d'attente",
    playlists: "Playlists",
    settings: "Réglages",

    track_count: Plural::two("piste", "pistes"),
    song_count: Plural::two("morceau", "morceaux"),
    album_count: Plural::two("album", "albums"),
    playlist_count: Plural::two("playlist", "playlists"),
    artist_count: Plural::two("artiste", "artistes"),
    more_file_count: Plural::two("autre fichier", "autres fichiers"),

    no_tracks: "Aucune piste",
    library_empty: "La bibliothèque est vide",
    no_results: "Aucun résultat",
    no_albums: "Aucun album",
    no_playlists: "Aucune playlist",

    search: "Rechercher",
    found_of: "{found} sur {tracks}",
    bitrate: "{kbps} kb/s",

    open_files: "Ouvrir des fichiers",
    add_files: "Ajouter des fichiers",
    add_folder: "Ajouter un dossier",
    audio_files: "Audio",
    adding_progress: "Ajout : {done} sur {total}",
    updating_library_progress: "Mise à jour de la bibliothèque : {done} sur {total}",
    scanning_folder: "Analyse du dossier…",
    no_audio_found: "Aucun fichier audio pris en charge trouvé",
    more_files_failed: "…impossible d'ajouter {more}",

    show_artist: "Afficher l'artiste",

    go_to_album: "Aller à l'album",

    show_in_library: "Afficher dans la bibliothèque",

    lyrics: "Paroles",

    no_lyrics: "Pas de paroles",

    searching_lyrics: "Recherche des paroles…",

    lyrics_unavailable: "Impossible de joindre LRCLIB, nouvel essai plus tard",
    lyrics_offline: "Activez la recherche en ligne dans les réglages pour trouver les paroles",

    instrumental: "Instrumental",
    show_album: "Afficher l'album",
    show_current_track: "Afficher la piste en cours",
    file_missing: "Fichier introuvable : il a été déplacé ou supprimé",
    shuffle: "Lecture aléatoire",
    repeat_all: "Tout répéter",
    repeat_one: "Répéter un morceau",

    play: "Lire",
    add_to_queue: "Ajouter à la file d'attente",
    remove_from_library: "Supprimer de la bibliothèque",
    remove_from_queue: "Retirer de la file d'attente",
    play_items: "Lire {items}",
    add_items_to_queue: "Ajouter {items} à la file d'attente",
    remove_items_from_library: "Supprimer {items} de la bibliothèque",
    remove_items_from_queue: "Retirer {items} de la file d'attente",
    remove: "Supprimer",
    cancel: "Annuler",
    ok: "OK",
    done: "Terminé",

    save_queue: "Enregistrer comme playlist",
    save_queue_to: "Enregistrer dans « {playlist} »",
    save_as_new_playlist: "Enregistrer comme nouvelle playlist…",
    clear_queue: "Vider la file d'attente",
    add_to_playlist: "Ajouter à une playlist",
    new_playlist: "Nouvelle playlist…",
    remove_from_this_playlist: "Retirer de la playlist",
    add_to_library: "Ajouter à la bibliothèque",
    add_items_to_library: "Ajouter {items} à la bibliothèque",
    remove_items_from_this_playlist: "Retirer {items} de la playlist",
    rename: "Renommer",
    delete: "Supprimer",
    save: "Enregistrer",
    save_playlist_title: "Enregistrer la file d'attente comme playlist",
    new_playlist_title: "Nouvelle playlist",
    rename_playlist_title: "Renommer la playlist",
    playlist_name_hint: "Nom de la playlist",
    delete_playlist_title: "Supprimer la playlist ?",
    delete_playlist_message: "« {playlist} » sera supprimée. \
                              Les morceaux eux-mêmes ne seront pas supprimés.",
    delete_playlists_title: "Supprimer {playlists} ?",
    delete_playlists_message: "{playlists} seront supprimées. Les morceaux eux-mêmes ne seront pas supprimés.",
    delete_items: "Supprimer {items}",

    language: "Langue",
    version: "Version",
    online_search: "Recherche en ligne",
    online_search_note: "Photos d'artistes depuis Deezer et paroles depuis LRCLIB. Les noms d'artistes et les titres des morceaux sont envoyés à ces services.",
    glass_tint: "Couleur",
    transparency: "Transparence",
    tint_graphite: "Graphite",
    tint_blue: "Bleu",
    tint_violet: "Violet",
    tint_green: "Vert",
    tint_rose: "Rose",
    tint_amber: "Ambre",

    remove_album_title: "Supprimer l'album de la bibliothèque ?",
    remove_album_message: "« {album} » sera supprimé de votre bibliothèque. \
                           Les fichiers resteront sur votre disque.",
    remove_albums_title: "Supprimer {albums} de la bibliothèque ?",
    remove_albums_message: "{albums} ({tracks}) seront supprimés de votre bibliothèque. \
                            Les fichiers resteront sur votre disque.",
    remove_artist_title: "Supprimer l'artiste de la bibliothèque ?",
    remove_artist_message: "Toutes les pistes de « {artist} » seront supprimées de votre \
                            bibliothèque. Les fichiers resteront sur votre disque.",
    remove_artists_title: "Supprimer {artists} de la bibliothèque ?",
    remove_artists_message: "Toutes les pistes de {artists} ({tracks}) seront supprimées de \
                             votre bibliothèque. Les fichiers resteront sur votre disque.",
    remove_song_title: "Supprimer le morceau de la bibliothèque ?",
    remove_song_message: "« {song} » sera supprimé de votre bibliothèque. \
                          Le fichier restera sur votre disque.",
    remove_songs_title: "Supprimer {songs} de la bibliothèque ?",
    remove_songs_message: "{songs} seront supprimés de votre bibliothèque. \
                           Les fichiers resteront sur votre disque.",
};
