# LMP

An audio player for macOS and Windows, written in Rust with [egui](https://github.com/emilk/egui) / eframe.
Plays MP3, FLAC, WAV, OGG, M4A and AAC.

## Features

- **Library** — add files or whole folders (with subfolders); browse by Artists, Albums and Songs,
  with search in each view. Files stay where they are: the library only remembers paths and tags,
  and removing a track from the library never deletes the file. Files that went missing from disk
  are marked in the lists.
- **Queue** — what's playing now, with shuffle and repeat one / repeat all. Tracks can be added
  to the queue without adding them to the library. The queue is restored on the next launch.
- **Saved playlists** — save the queue as a playlist, create playlists from library tracks,
  rename, delete. A queue started from a playlist stays linked to it, so saving writes the
  changes back.
- **Now Playing** — cover, track details (album, year, genre, bitrate) and a blurred cover backdrop.
- **Lyrics** — from an `.lrc` file next to the track, from tags, or from LRCLIB. Synced lyrics
  highlight the current line and seek on click.
- **Covers and artist images** — album covers from tags or from an image in the album folder
  (`cover.jpg`, `folder.png`, `front.jpg`…); artist images from `artist.jpg` (`.png`, `.webp`)
  next to the artist's music, otherwise from Deezer.
- **Drag and drop** — drop files and folders onto the window: to the left of the queue they go
  to the library, onto the queue they go to the queue.
- **System integration** — media keys, headphone buttons, Control Center (macOS) and the media
  overlay (Windows); the window is translucent glass over the system blur, with adjustable tint
  and transparency.
- **Languages** — English, French, Russian and Ukrainian.
- Old MP3 tags in Windows-1251 that show up as `Ïîëåò øìåëÿ` are decoded correctly.

### Online services

Online search is **off by default** and can be turned on in Settings. When it is on, the app
sends artist names and track titles to:

- [LRCLIB](https://lrclib.net) — lyrics;
- [Deezer](https://developers.deezer.com/api) — artist photos (only a link is cached; the photos
  themselves are never written to disk).

No keys or accounts are needed. With online search off, only local lyrics and images are used.

### Keyboard

| Key | Action |
| --- | --- |
| Space | Play / pause |
| ⌘→ / ⌘← (Ctrl on Windows) | Next / previous track |
| ⌘A (Ctrl+A) | Select all in the active list or grid |
| Delete / Backspace | Remove the selection |
| Esc | Clear the selection or the search field |

Lists and grids also support ⌘/Ctrl+click, Shift+click and selecting with a mouse marquee.

### Where data is stored

- **macOS** — `~/Library/Application Support/LMP/`
- **Windows** — `%APPDATA%\LMP\data\`

The folder holds `library.json`, `playlists.json`, `playlist.json` (the queue), `settings.json`,
and the online answer caches `lyrics/` and `artists/`. Deleting the folder resets the app.

## Development

```sh
cargo run    # debug build; dependencies are still optimized (see Cargo.toml)
cargo test
```

The code is split into two parts:

- `src/core` — player logic without UI: library, queue and playback, playlists, settings,
  lyrics and artist images. Knows nothing about egui and is covered by tests.
- `src/ui` — the egui interface. All changes to app data go through `Action`
  (`src/ui/action.rs`), so everything the app can do is listed in one place.
  `src/ui/mod.rs` describes the rest of the modules.

## Building

Both builds are made on macOS, including the Windows one, which is cross-compiled with MinGW.

### Prerequisites

- `cargo-bundle` — builds the `.app` and `.dmg`;
- the `x86_64-pc-windows-gnu` target and MinGW — build the Windows `.exe`
  (the linker is set in `.cargo/config.toml`);
- `makensis` — builds the Windows installer.

```sh
cargo install cargo-bundle
rustup target add x86_64-pc-windows-gnu
brew install mingw-w64
brew install makensis
```

### macOS

```sh
cargo bundle --release
```

Output:

- `target/release/bundle/osx/LMP.app` — the app itself;
- `target/release/bundle/dmg/LMP.dmg` — disk image for distribution: it contains
  the app and a shortcut to the Applications folder, so you install by dragging the app there.

The build is Apple Silicon (arm64) only.

### Windows

First the app, then the installer from it:

```sh
cargo build --release --target x86_64-pc-windows-gnu
mkdir -p target/installer
makensis -DVERSION=0.1.0 installer/windows.nsi
```

Output:

- `target/x86_64-pc-windows-gnu/release/lmp.exe` — the app, runs without installation;
- `target/installer/LMP Setup 0.1.0.exe` — the installer: installs into Program Files,
  creates Start menu shortcuts and an optional desktop shortcut, and registers the app
  in "Apps & features" so it can be uninstalled.

The icon is embedded into the `.exe` at build time (`build.rs`); the installer script is
`installer/windows.nsi`.

### Releasing a new version

The version number lives in two places and has to be changed in both:

- `version` in `Cargo.toml`;
- `-DVERSION=` in the `makensis` command.

### Code signing

The builds are not signed:

- **macOS** — on another Mac, Gatekeeper will say the app can't be opened.
  To open it: right-click → Open.
- **Windows** — SmartScreen will show "Windows protected your PC".
  To run it: More info → Run anyway.

Getting rid of these warnings requires signing: an Apple Developer ID plus notarization on macOS,
and a code-signing certificate on Windows.
