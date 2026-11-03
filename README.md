# GameTracker

A Windows app that records how long you play your PC games, automatically. It finds your
games in Steam, Epic Games and GOG (or folders you choose), notices when one is running,
and keeps the history on your PC.

Built with [Tauri 2](https://tauri.app) (Rust) and React + TypeScript + Tailwind.

## Features

- **Automatic tracking.** Any program started from a game's install folder counts as that
  game, so launchers and Unreal Engine games are tracked correctly. Runs in the system tray
  and can start with Windows.
- **Reliable sessions.** Running sessions write a heartbeat every 30 seconds. After a crash
  or power cut the session ends at the last heartbeat, and time the PC spent asleep isn't
  counted as play.
- **Library.** Steam, Epic and GOG discovery, folder scanning, cover art (Steam, or
  SteamGridDB with your own API key), renaming, choosing which programs and folders belong
  to a game, and restoring removed games.
- **Statistics.** Playtime per day, hour or month in local time, top games, and activity by
  weekday. Sessions that cross midnight are split across both days.
- **Your data.** Export and import as JSON, delete history, or reset. Sessions can be edited
  or deleted when something was recorded wrong.

## Development

Requirements: Node.js 20+, Rust (stable), and the
[Tauri prerequisites for Windows](https://tauri.app/start/prerequisites/).

```bash
npm install
npm run tauri dev
```

Tests and checks:

```bash
cd src-tauri && cargo test
npx tsc --noEmit
```

Optional: copy `.env.example` values into `.env` to enable sign-in (Supabase). The app works
fully without an account.

## Project layout

| Path | What's there |
| --- | --- |
| `src/pages` | Home, Library, Game details, Statistics, Settings, Onboarding |
| `src/components` | Shared UI: game cards, covers, modals, toasts, charts |
| `src/store` | Zustand stores; `useBackendSync` keeps them up to date from backend events |
| `src-tauri/src/detection` | Process monitor and the detector that maps processes to games |
| `src-tauri/src/tracking` | Sessions, heartbeats and crash recovery |
| `src-tauri/src/discovery` | Steam, Epic and GOG discovery, and the folder scanner |
| `src-tauri/src/stats` | Dashboard and per-game statistics |
| `src-tauri/src/database` | SQLite schema, migrations and library editing |
| `src-tauri/src/commands` | The commands the UI calls |

Data lives in `%APPDATA%\com.naseeh.gametracker` and logs in
`%LOCALAPPDATA%\com.naseeh.gametracker\logs` (Settings → Help & About → Open log folder).

## Releasing

See [RELEASING.md](RELEASING.md) for code signing and turning on automatic updates.
