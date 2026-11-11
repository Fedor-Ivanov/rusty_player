//! UI texts in Russian.

use super::{Plural, Texts, slavic_plural_form};

pub static TEXTS: Texts = Texts {
    plural_form: slavic_plural_form,

    library: "Медиатека",
    now_playing: "Сейчас играет",
    artists: "Артисты",
    albums: "Альбомы",
    songs: "Песни",
    queue: "Очередь",
    playlists: "Плейлисты",
    settings: "Настройки",

    track_count: Plural::three("трек", "трека", "треков"),
    song_count: Plural::three("песня", "песни", "песен"),
    album_count: Plural::three("альбом", "альбома", "альбомов"),
    playlist_count: Plural::three("плейлист", "плейлиста", "плейлистов"),
    artist_count: Plural::three("артист", "артиста", "артистов"),
    more_file_count: Plural::three("файл", "файла", "файлов"),

    no_tracks: "Нет треков",
    library_empty: "Медиатека пуста",
    no_results: "Ничего не найдено",
    no_albums: "Нет альбомов",
    no_playlists: "Нет плейлистов",

    search: "Поиск",
    found_of: "{found} из {tracks}",
    bitrate: "{kbps} кбит/с",

    open_files: "Открыть файлы",
    add_files: "Добавить файлы",
    add_folder: "Добавить папку",
    audio_files: "Аудио",
    adding_progress: "Добавление: {done} из {total}",
    updating_library_progress: "Обновление медиатеки: {done} из {total}",
    scanning_folder: "Сканирование папки…",
    no_audio_found: "Не найдено поддерживаемых аудиофайлов",
    more_files_failed: "…и ещё {more} не удалось добавить",

    show_artist: "Показать артиста",

    go_to_album: "Перейти к альбому",

    show_in_library: "Показать в медиатеке",

    lyrics: "Текст песни",

    no_lyrics: "Нет текста песни",

    searching_lyrics: "Ищем текст песни…",

    lyrics_unavailable: "Не удалось связаться с LRCLIB, попробуем позже",
    lyrics_offline: "Включите поиск в интернете в настройках, чтобы найти текст",

    instrumental: "Инструментал",
    show_album: "Показать альбом",
    show_current_track: "Показать текущий трек",
    file_missing: "Файл не найден: его переместили или удалили",
    shuffle: "Перемешать",
    repeat_all: "Повторять все",
    repeat_one: "Повторять один",

    play: "Воспроизвести",
    add_to_queue: "Добавить в очередь",
    remove_from_library: "Удалить из медиатеки",
    remove_from_queue: "Убрать из очереди",
    play_items: "Воспроизвести {items}",
    add_items_to_queue: "Добавить {items} в очередь",
    remove_items_from_library: "Удалить {items} из медиатеки",
    remove_items_from_queue: "Убрать {items} из очереди",
    remove: "Удалить",
    cancel: "Отмена",
    ok: "OK",
    done: "Готово",

    save_queue: "Сохранить как плейлист",
    save_queue_to: "Сохранить в «{playlist}»",
    save_as_new_playlist: "Сохранить как новый плейлист…",
    clear_queue: "Очистить очередь",
    add_to_playlist: "Добавить в плейлист",
    new_playlist: "Новый плейлист…",
    remove_from_this_playlist: "Убрать из плейлиста",
    add_to_library: "Добавить в медиатеку",
    add_items_to_library: "Добавить {items} в медиатеку",
    remove_items_from_this_playlist: "Убрать {items} из плейлиста",
    rename: "Переименовать",
    delete: "Удалить",
    save: "Сохранить",
    save_playlist_title: "Сохранить очередь как плейлист",
    new_playlist_title: "Новый плейлист",
    rename_playlist_title: "Переименовать плейлист",
    playlist_name_hint: "Название плейлиста",
    delete_playlist_title: "Удалить плейлист?",
    delete_playlist_message: "Плейлист «{playlist}» будет удалён. Сами треки не удаляются.",
    delete_playlists_title: "Удалить {playlists}?",
    delete_playlists_message: "Выбранные плейлисты ({playlists}) будут удалены. Сами треки не удаляются.",
    delete_items: "Удалить {items}",

    language: "Язык",
    version: "Версия",
    online_search: "Поиск в интернете",
    online_search_note: "Фото исполнителей из Deezer и тексты песен из LRCLIB. Этим сервисам отправляются имена исполнителей и названия треков.",
    glass_tint: "Цвет",
    transparency: "Прозрачность",
    tint_graphite: "Графит",
    tint_blue: "Синий",
    tint_violet: "Фиолетовый",
    tint_green: "Зелёный",
    tint_rose: "Розовый",
    tint_amber: "Янтарный",

    remove_album_title: "Удалить альбом из медиатеки?",
    remove_album_message: "Альбом «{album}» будет удалён из медиатеки. \
                           Файлы останутся на диске.",
    remove_albums_title: "Удалить {albums} из медиатеки?",
    remove_albums_message: "Выбранные альбомы ({albums}, {tracks}) будут удалены из медиатеки. \
                            Файлы останутся на диске.",
    remove_artist_title: "Удалить артиста из медиатеки?",
    remove_artist_message: "Все треки артиста «{artist}» будут удалены из медиатеки. \
                            Файлы останутся на диске.",
    remove_artists_title: "Удалить {artists} из медиатеки?",
    remove_artists_message: "Все треки выбранных артистов ({artists}, {tracks}) будут удалены \
                             из медиатеки. Файлы останутся на диске.",
    remove_song_title: "Удалить песню из медиатеки?",
    remove_song_message: "Песня «{song}» будет удалена из медиатеки. Файл останется на диске.",
    remove_songs_title: "Удалить {songs} из медиатеки?",
    remove_songs_message: "Выбранные песни ({songs}) будут удалены из медиатеки. \
                           Файлы останутся на диске.",
};
