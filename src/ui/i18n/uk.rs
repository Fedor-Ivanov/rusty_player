//! UI texts in Ukrainian.

use super::{Plural, Texts, slavic_plural_form};

pub static TEXTS: Texts = Texts {
    plural_form: slavic_plural_form,

    library: "Медіатека",
    now_playing: "Зараз грає",
    artists: "Артисти",
    albums: "Альбоми",
    songs: "Пісні",
    queue: "Черга",
    playlists: "Плейлисти",
    settings: "Налаштування",

    track_count: Plural::three("трек", "треки", "треків"),
    song_count: Plural::three("пісня", "пісні", "пісень"),
    album_count: Plural::three("альбом", "альбоми", "альбомів"),
    playlist_count: Plural::three("плейлист", "плейлисти", "плейлистів"),
    artist_count: Plural::three("артист", "артисти", "артистів"),
    more_file_count: Plural::three("файл", "файли", "файлів"),

    no_tracks: "Немає треків",
    library_empty: "Медіатека порожня",
    no_results: "Нічого не знайдено",
    no_albums: "Немає альбомів",
    no_playlists: "Немає плейлистів",

    search: "Пошук",
    found_of: "{found} з {tracks}",
    bitrate: "{kbps} кбіт/с",

    open_files: "Відкрити файли",
    add_files: "Додати файли",
    add_folder: "Додати папку",
    audio_files: "Аудіо",
    adding_progress: "Додавання: {done} з {total}",
    updating_library_progress: "Оновлення медіатеки: {done} з {total}",
    scanning_folder: "Сканування папки…",
    no_audio_found: "Не знайдено підтримуваних аудіофайлів",
    more_files_failed: "…і ще {more} не вдалося додати",

    show_artist: "Показати артиста",

    go_to_album: "Перейти до альбому",

    show_in_library: "Показати в медіатеці",

    lyrics: "Текст пісні",

    no_lyrics: "Немає тексту пісні",

    searching_lyrics: "Шукаємо текст пісні…",

    lyrics_unavailable: "Не вдалося зв'язатися з LRCLIB, спробуємо пізніше",
    lyrics_offline: "Увімкніть пошук в інтернеті в налаштуваннях, щоб знайти текст",

    instrumental: "Інструментал",
    show_album: "Показати альбом",
    show_current_track: "Показати поточний трек",
    file_missing: "Файл не знайдено: його переміщено або видалено",
    shuffle: "Перемішати",
    repeat_all: "Повторювати всі",
    repeat_one: "Повторювати один",

    play: "Відтворити",
    add_to_queue: "Додати до черги",
    remove_from_library: "Видалити з медіатеки",
    remove_from_queue: "Прибрати з черги",
    play_items: "Відтворити {items}",
    add_items_to_queue: "Додати {items} до черги",
    remove_items_from_library: "Видалити {items} з медіатеки",
    remove_items_from_queue: "Прибрати {items} з черги",
    remove: "Видалити",
    cancel: "Скасувати",
    ok: "OK",
    done: "Готово",

    save_queue: "Зберегти як плейлист",
    save_queue_to: "Зберегти в «{playlist}»",
    save_as_new_playlist: "Зберегти як новий плейлист…",
    clear_queue: "Очистити чергу",
    add_to_playlist: "Додати до плейлиста",
    new_playlist: "Новий плейлист…",
    remove_from_this_playlist: "Прибрати з плейлиста",
    add_to_library: "Додати до медіатеки",
    add_items_to_library: "Додати {items} до медіатеки",
    remove_items_from_this_playlist: "Прибрати {items} з плейлиста",
    rename: "Перейменувати",
    delete: "Видалити",
    save: "Зберегти",
    save_playlist_title: "Зберегти чергу як плейлист",
    new_playlist_title: "Новий плейлист",
    rename_playlist_title: "Перейменувати плейлист",
    playlist_name_hint: "Назва плейлиста",
    delete_playlist_title: "Видалити плейлист?",
    delete_playlist_message: "Плейлист «{playlist}» буде видалено. Самі треки не видаляються.",
    delete_playlists_title: "Видалити {playlists}?",
    delete_playlists_message: "Вибрані плейлисти ({playlists}) буде видалено. Самі треки не видаляються.",
    delete_items: "Видалити {items}",

    language: "Мова",
    version: "Версія",
    online_search: "Пошук в інтернеті",
    online_search_note: "Фото виконавців із Deezer і тексти пісень із LRCLIB. Цим сервісам надсилаються імена виконавців і назви треків.",
    glass_tint: "Колір",
    transparency: "Прозорість",
    tint_graphite: "Графіт",
    tint_blue: "Синій",
    tint_violet: "Фіолетовий",
    tint_green: "Зелений",
    tint_rose: "Рожевий",
    tint_amber: "Бурштиновий",

    remove_album_title: "Видалити альбом з медіатеки?",
    remove_album_message: "Альбом «{album}» буде видалено з медіатеки. \
                           Файли залишаться на диску.",
    remove_albums_title: "Видалити {albums} з медіатеки?",
    remove_albums_message: "Вибрані альбоми ({albums}, {tracks}) буде видалено з медіатеки. \
                            Файли залишаться на диску.",
    remove_artist_title: "Видалити артиста з медіатеки?",
    remove_artist_message: "Усі треки артиста «{artist}» буде видалено з медіатеки. \
                            Файли залишаться на диску.",
    remove_artists_title: "Видалити {artists} з медіатеки?",
    remove_artists_message: "Усі треки вибраних артистів ({artists}, {tracks}) буде видалено \
                             з медіатеки. Файли залишаться на диску.",
    remove_song_title: "Видалити пісню з медіатеки?",
    remove_song_message: "Пісню «{song}» буде видалено з медіатеки. Файл залишиться на диску.",
    remove_songs_title: "Видалити {songs} з медіатеки?",
    remove_songs_message: "Вибрані пісні ({songs}) буде видалено з медіатеки. \
                           Файли залишаться на диску.",
};
