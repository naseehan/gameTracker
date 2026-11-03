# GameTracker: work so far and handoff notes

For an AI assistant or developer picking up this project. It summarizes what was changed,
why, how the code is organized now, and what is still open.

## The app

GameTracker is a **Windows-only** desktop app that records how long you play PC games. It
finds games in Steam, Epic Games and GOG (or folders you scan), watches running processes,
records play sessions in a local SQLite database, and shows a library, statistics and history.

- **Stack:** Tauri 2 (Rust backend in `src-tauri/`) + React 19, TypeScript, Tailwind 4,
  Zustand and Recharts (`src/`).
- **Name/ID:** product `GameTracker`, identifier `com.naseeh.gametracker`. It used to be
  `tauri-app` / `com.naseeh.tauri-app`.
- **Data:** `%APPDATA%\com.naseeh.gametracker\gametracker.db`, plus a `covers\` folder.
- **Logs:** `%LOCALAPPDATA%\com.naseeh.gametracker\logs\gametracker.log`.
- **Run:** `npm run tauri dev`. **Test:** `cd src-tauri && cargo test` (58 tests) and
  `npx tsc --noEmit`.

## Current state (important)

- **Almost nothing is committed.** Git was set up during this work. There is one commit
  (`fix some bugs`) with the critical fixes. The tray work, the detection work and the big
  final batch are all **uncommitted** in the working tree (about 90 files). New files are
  marked with `git add -N`.
- The owner's rule: **never add `Co-Authored-By: Claude` or any AI attribution** to
  commits, PRs or code.
- Rust builds with **zero compiler warnings**. Clippy reports 13 style-only suggestions
  (`Default` impls, `sort_by_key`), which were intentionally left.
- The owner has run the new build: the data folder moved correctly, game tracking works,
  and cover art downloaded for all 5 games, including through SteamGridDB.

## What was done, in order

### 1. Review
The starting codebase had been built by patching files with ~24 Python scripts, had no git,
and contained a stale duplicate copy of the whole project. The review found data-corrupting
bugs, a UI that froze, broken features, and a lot of dead code.

### 2. Critical fixes (committed as `fix some bugs`)
- **Export/import:** export always failed once there was a game (it read a column index
  that didn't exist). Import dropped `is_removed`, so removed games came back. Import while
  a game was running could overwrite an imported session.
- **Crash recovery:** open sessions used to be closed at the *next app start*, so a crash
  overnight produced an 11-hour session. Running sessions now write `last_heartbeat_at`
  every 30 s. Recovery closes them at the last heartbeat. A heartbeat gap over 5 minutes
  (sleep or hibernate) splits the session, so sleep isn't counted as play.
- **Frozen UI:** all commands were synchronous, so they ran on the main thread, and
  discovery held the single DB mutex while walking folders. All commands are now `async`,
  heavy work runs in `spawn_blocking`, and discovery scans without the lock (`scan` then
  `apply`).
- Discovery no longer `unwrap()`s inside its SQL transaction; with `panic = "abort"`, any
  failure used to kill the app. Mutexes are taken with a poison-tolerant `lock()` helper.
- **Cover images:** the asset protocol is enabled and scoped to `$APPDATA/covers/**`. One
  `<GameCover>` component replaced three broken URL schemes. Each cover change gets a new
  file name, so cached images don't show.
- **Side fixes:** the detector is refreshed after discovery, add, remove and import; removed
  games are no longer tracked; a failed launcher scan no longer marks its games
  "Not Installed"; auto-discovery settings apply without a restart.

### 3. Background tracking (uncommitted)
- A tray icon (`src-tauri/src/tray.rs`) with Open and Quit. Closing the window hides it to
  the tray.
- "Start with Windows" via `tauri-plugin-autostart`, off by default. It launches with
  `--minimized`, so the window stays hidden.
- Single-instance: opening the app again focuses the existing window. The main window starts
  `visible: false` and is shown in `setup` unless the app was started with `--minimized`.
- The tray tooltip shows the games being played.

### 4. Game detection accuracy (uncommitted)
- **Install-folder matching** (`detection/detector.rs`): any process started from inside a
  game's install folder counts as that game. This handles launchers, Unreal `*-Shipping.exe`
  binaries and multi-exe games.
  - The longest matching folder wins.
  - Drive roots, `Program Files`, `Users`, and folders that contain other games' folders are
    ignored.
  - Helper exes (crash reporters, installers) and redistributable or `Engine` folders are
    ignored (`detection/filters.rs`).
- **Name-only matching** is used only when the process path can't be read.
- **Scanner** (`discovery/scanner.rs`):
  - Skip rules match whole folder names, so "Engine" no longer hides *Space Engineers*.
  - Depth 5, so it reaches Unreal binaries.
  - `.EXE` in capitals is accepted.
  - `Launcher.exe` is kept only when a game has nothing else.
  - A library folder like `D:\Games` is no longer added as a fake game.
- **GOG registry parsing** matches value names exactly, so `exeFile` no longer overwrites
  `exe`. `reg.exe` runs with `CREATE_NO_WINDOW`, so no console window flashes.
- **Duplicates:** one `normalize_name()` (lowercase, letters and digits only) is used
  everywhere. Discovery and manual adds attach to an existing same-name game. A **one-time**
  startup merge combines existing duplicates, gated by the setting `library_names_v2`.
  Renames that would clash with another game are rejected.

### 5. Shipping, statistics, errors, security and product (uncommitted)
- **Statistics** (`stats/queries.rs`, `stats/game_queries.rs`, `stats/time.rs`):
  - A running session is counted once.
  - Every summary number follows the selected range.
  - Days are local everywhere; sessions crossing midnight are split.
  - Empty days show as zero.
  - Buckets: hourly for "today", monthly for histories over 90 days.
  - Local midnight is computed safely, so the daylight-saving-at-midnight crash is gone.
- **Identity:**
  - Renamed to GameTracker, with a new icon (`src-tauri/icons/app-icon.svg`, generated
    with `tauri icon`).
  - The window is 1280×800 with a 1024×640 minimum.
  - The installer is NSIS only.
  - First start copies the DB and covers from the old `com.naseeh.tauri-app` folder (the old
    folder is kept as a backup) and rewrites cover paths.
- **Updater:** `tauri-plugin-updater` is wired in (`updates.rs`) but **off** while
  `plugins.updater.pubkey` and `endpoints` in `tauri.conf.json` are empty.
  `RELEASING.md` explains enabling it and code signing. No signing certificate exists yet.
- **Errors:**
  - Toasts (`toastStore`, `Toaster`) and a promise-based confirm dialog (`confirmStore`,
    `ConfirmDialog`) replaced every `alert()` and `confirm()`.
  - An accessible `Modal` closes on Escape and traps focus.
  - A startup error screen replaces the endless "Loading...".
  - Logging uses `tauri-plugin-log`: Rust `log::` macros, and JS `reportError()` in
    `src/utils/errors.ts`. Settings has "Open log folder".
- **Security:**
  - Release builds have a CSP; `devCsp` is null so Vite HMR keeps working.
  - **All file dialogs run in Rust** (`pick_folder`, `pick_and_scan_folder`,
    `choose_game_cover`, `export_data`, `import_data`, etc.), so no command accepts a file
    path from the webview. The JS dialog permission and package were removed.
  - Capabilities: `core`, `opener`, `deep-link`, `log`.
- **Cover art** (`covers.rs`):
  - Steam games use Steam's local `appcache/librarycache` (old and new layouts), then
    Steam's CDN.
  - Other games use SteamGridDB with the user's API key, stored in Windows Credential
    Manager (keyring entry `steamgriddb_api_key`).
  - Covers are only fetched for games without one; a game with no cover found is retried
    after 7 days (`games.cover_lookup_at`).
  - Network work happens outside the DB lock.
- **Push instead of poll:** the backend emits `library-changed` and `sessions-changed`
  (`events.rs`). `useBackendSync()` in `Layout` keeps the library and active-session stores
  current. The only remaining timer is a once-a-minute refresh while a game is running.
- **New pages and UI:**
  - **Home** (now playing, last 7 days, sessions grouped by day) replaced the Recently
    Played and Currently Playing pages. Navigation is Home / Library / Statistics /
    Settings.
  - A live "now playing" pill sits in the sidebar.
  - Game cards show two-line titles and are real `<button>`s, so keyboard Enter works.
  - The game page has inline rename, a ⋯ menu (change cover, remove with an optional
    "delete history"), a Detection card (edit the programs and install folders that identify
    the game), and session edit/delete with "Load more".
  - Settings gained Cover Art (API key), Removed Games (restore), rewritten Privacy text,
    and Help & About (version, updates, logs).
  - Onboarding: Welcome → "Find my games" runs discovery and lists results per launcher →
    optional folder scan → done, with a "Start with Windows" toggle. Sign-in moved out of
    onboarding; it's in Settings only.
- **Cleanup:**
  - Deleted the patch scripts, the duplicate `gameTracker/` folder, `dev.log`,
    `test_sysinfo.rs`, the empty DB file, and template files and assets.
  - Removed the unused `greet`, `get_games`, `get_active_processes` and
    `get_detected_games` commands and their state, plus the `tailwind-merge` package.
  - The process monitor now refreshes only names and exe paths.
  - Wrote a real `README.md`, `RELEASING.md` and `.env.example`.

## How the code fits together

- `src-tauri/src/lib.rs`: plugin setup, tray, startup, and two background threads:
  - **discovery**: wakes every 30 s and runs when the configured interval has passed;
  - **process events + heartbeat**: `recv_timeout` on the monitor channel, heartbeat every
    30 s.
- `commands/mod.rs`: every UI command. All are `async`. Slow ones use `run_blocking`. Any
  command that changes data emits an event. `AppState` holds the `AppHandle`, the DB
  `Mutex<Connection>`, the tracker, the detector, and settings atomics.
- **Lock order:** take `db` before `session_tracker` and `detector`, never the reverse. The
  process handler calls the detector *before* locking the DB.
- `database/mod.rs`: schema and migrations (`ALTER TABLE` with ignored errors, the
  existing pattern), data-folder move, duplicate merge, `load_detector`.
  `database/editing.rs`: rename, session edit/delete, detection edits, restore.
- `tracking/tracker.rs`: sessions keyed by game, with several PIDs per session; heartbeats,
  sleep splitting and recovery.
- Bundled SQLite **enforces foreign keys by default**, so test fixtures must insert the
  `games` row first.
- **Frontend:** `src/pages/*`, `src/components/*` (including `game/` and `statistics/`),
  Zustand stores in `src/store/*`, `useBackendEvent`/`useBackendSync` in
  `src/hooks/useBackendEvent.ts`. Types are hand-written in `src/types/index.ts` and must
  match the Rust DTOs.

## Not verified yet

- A release build (`npm run tauri build`) hasn't been tried. It's needed to confirm the
  NSIS bundle and that the CSP doesn't block anything, especially Supabase sign-in.
- Tray, autostart and single-instance behaviour in the real app. Most UI changes were
  checked in a browser with a mocked backend, not in the Tauri window.

## Open items / suggested next steps

1. Commit the uncommitted work, ideally as a few logical commits.
2. Run a release build and test it on a clean PC.
3. Code signing: buy a certificate, or use Azure Trusted Signing (see `RELEASING.md`).
4. Turn on updates: generate an updater key, pick an endpoint (GitHub Releases), and set
   `createUpdaterArtifacts`.
5. Account feature (Supabase): it does nothing yet besides sign-in, and the free tier pauses
   after 7 idle days. Decide whether to build sync or remove it.
6. Switch registry reads from `reg.exe` to the `winreg` crate, so non-English names aren't
   garbled.
7. Generate TypeScript types from Rust (`tauri-specta` / `ts-rs`) instead of hand-copying.
8. Versioned DB migrations (`PRAGMA user_version`), indexes on `game_sessions(game_id,
   started_at)`, and `UNIQUE` constraints instead of count-then-insert.
9. Smaller items: lazy-load Recharts (the bundle is one ~860 KB chunk), clean up the clippy
   style suggestions, add a one-time "still running in the tray" hint, and add a
   pause-tracking item to the tray menu.
