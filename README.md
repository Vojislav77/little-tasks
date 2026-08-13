# Little Tasks

A simple tasks app — a small always-available quick-add
popover, a full task editor with task lists, SQLite persistence, and cross-device
portability via a portable JSON bundle (a stand-in for a future sync service).

Built with **Tauri v2** (Rust backend + web UI) and **Svelte 5**. Runs on
**Fedora 44 / KDE**, and packages for **Windows, macOS and Linux**.

---

## Features

- **System tray popover** (KDE StatusNotifier / AppIndicator friendly)
  - quick add: title + target task list
  - search across task title, link and comment
  - task list with checkbox toggle and delete right on the row
  - open any task in the editor
  - left-click toggles the popover; right-click menu: Open Little Tasks /
    Quick Add / New Task / Quit
- **Full editor window** (single shared instance — no duplicates)
  - left sidebar: search, "All tasks" view, task lists (create / rename / delete)
  - compose input to add a task
  - task list with checkbox bullets (completed tasks struck through)
  - **Task details** section: title, Done toggle, **Link** and **Comment** fields
  - save with `Ctrl+S` (dirty-dot indicator)
  - **Settings** modal (⚙): *Start with system* and *Show only pending tasks*,
    plus **Export JSON / Import JSON**
  - footer: © 2026 Vojislav Korać · v0.1.0
- **Keyboard shortcuts**: `Ctrl+N` new task, `Ctrl+F` focus search, `Ctrl+S` save,
  `Ctrl+Q` quit, `Esc` hide popover
- **Local memory**: tasks persist across restarts in SQLite
- **Portability (MVP)**: export / import all task lists and tasks as one JSON
  "sync bundle"

---

## Tech stack

| Layer   | Choice                                        |
| ------- | --------------------------------------------- |
| Shell   | Tauri v2 (preferred over Electron)            |
| Backend | Rust (commands, tray, windows, SQLite)        |
| UI      | Svelte 5 + Vite (TypeScript)                  |
| Storage | SQLite via `rusqlite` (bundled)               |
| Dialogs | `@tauri-apps/plugin-dialog`                   |
| Autostart | `@tauri-apps/plugin-autostart`              |
| Logging | `tauri-plugin-log` (stdout + file)            |

---

## Repository layout

```
.
├── app/                        # Svelte UI ("app/")
│   ├── lib/
│   │   ├── api.ts              # typed wrappers around Tauri commands
│   │   ├── format.ts           # task preview / when-format helpers
│   │   ├── toasts.ts           # toast store
│   │   └── types.ts            # shared frontend types
│   ├── windows/
│   │   ├── TrayWindow.svelte   # tray popover (quick add + task list)
│   │   └── EditorWindow.svelte # full editor + settings modal
│   └── components/             # TaskListItem, TaskListRow, Toasts
├── src-tauri/
│   ├── src/
│   │   ├── core/               # task domain (types, validation, time helpers)
│   │   ├── storage/            # SQLite + migrations (task_lists, tasks, settings)
│   │   ├── sync/               # bundle format, codec, MVP local file sync
│   │   └── backend/            # Tauri commands, tray, window management
│   ├── capabilities/default.json
│   ├── tauri.conf.json
│   └── icons/                  # generated app + tray icons
├── .github/workflows/build.yml # CI: Windows / macOS / Linux artifacts
└── package.json
```

Domain logic (`core`, `storage`, `sync`) has **no desktop-only assumptions**:
it can be reused later from a CLI, a mobile app, or a sync service.

---

## Prerequisites (Fedora 44 / KDE)

Rust toolchain:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

System libraries required by Tauri v2:

```bash
sudo dnf install -y \
  gcc-c++ \
  webkit2gtk4.1-devel \
  gtk3-devel \
  openssl-devel \
  librsvg2-devel \
  libayatana-appindicator-glib-devel \
  patchelf
```

Node.js ≥ 20 with npm (v22 works great).

---

## Run locally (dev workflow)

```bash
# 1) install frontend deps
npm install

# 2) start tray + editor windows (builds Rust, serves Vite)
npm run dev:desktop        # = tauri dev

#   — starts the system tray icon
#   — creates the SQLite DB + runs migrations on first run
#   — left-click tray toggles the popover; right-click for the menu
```

The database lives in the OS app-data dir (created automatically on first run):

| OS      | Path                                                                 |
| ------- | -------------------------------------------------------------------- |
| Linux   | `~/.local/share/com.littletasks.desktop/little-tasks.db`             |
| macOS   | `~/Library/Application Support/com.littletasks.desktop/`             |
| Windows | `%APPDATA%\com.littletasks.desktop\`                                 |

Logs are written to stdout and to the app data dir under `logs/little-tasks.log`.

> Browser-only UI development: `npm run dev` serves the UI at
> http://localhost:1420 without Tauri (dialogs/shortcuts that need Tauri are no-ops).

---

## Testing

```bash
# Rust: storage CRUD, search correctness, export/import round-trip,
#       migration application, bundle codec, validation
cd src-tauri && cargo test --release

# Frontend: format helpers
npm run test
```

---

## Build for Windows / macOS / Linux

Tauri handles cross-compilation from each OS (native builds are simplest):

```bash
npm run tauri build
```

- **Linux** produces `.rpm`, `.deb`, and `.AppImage` (in
  `src-tauri/target/release/bundle/`). `rpm` is handy on Fedora.
- **macOS** produces a `.dmg` (+ `.app`).
- **Windows** produces an MSI/NSIS installer.

Per-OS prerequisites:

| OS      | Requirements                                                                 |
| ------- | ---------------------------------------------------------------------------- |
| Linux   | packages listed in *Prerequisites*; `rpm-build` + `dpkg` for .rpm/.deb targets |
| macOS   | Xcode command line tools (`xcode-select --install`)                          |
| Windows | Visual Studio Build Tools (C++), WebView2 runtime (preinstalled on Win 11)    |

### Fedora note: AppImage builds

linuxdeploy ships an old `strip` that chokes on newer `.relr.dyn` sections
(Fedora 40+). If AppImage bundling fails with `Strip call failed … .relr.dyn`,
build with stripping disabled:

```bash
NO_STRIP=1 npm run tauri build -- --bundles appimage
```

The CI workflow already sets `NO_STRIP=1` on its Linux job.

### Package a release

```bash
# tagged release → CI builds artifacts for all 3 OSes
npm run tauri build -- --bundles appimage,rpm,deb   # Linux example
# or configure in src-tauri/tauri.conf.json under "bundle"
```

CI (`.github/workflows/build.yml`) runs `vitest`, `vite build`, `cargo test`, and
`tauri build` on `ubuntu-latest`, `macos-latest`, and `windows-latest`, uploading
the bundle folders as artifacts.

---

## Data model

Two tables (plus a `settings` table for the toggles):

- **`task_lists`** — `id`, `title`, `created_at`, `updated_at`
- **`tasks`** — `id`, `list_id` (→ `task_lists`, cascade delete), `title`,
  `done`, `link`, `comment`, `created_at`, `updated_at`

Deleting a list deletes its tasks. Completed tasks stay in the list but are
struck through (hide them with *Show only pending tasks* in Settings).

---

## Export / Import ("sync bundle")

- **Export**: *Editor → ⚙ Settings → Export JSON*. A save dialog writes a single
  JSON file:

```json
{
  "schemaVersion": 1,
  "appVersion": "0.1.0",
  "exportedAt": "2026-08-03T18:28:00Z",
  "taskLists": [
    { "id": "…uuid…", "title": "Work", "createdAt": "…", "updatedAt": "…" }
  ],
  "tasks": [
    {
      "id": "…uuid…",
      "listId": "…uuid…",
      "title": "Ship the AppImage",
      "done": false,
      "link": "https://example.com",
      "comment": "remember NO_STRIP=1",
      "createdAt": "…",
      "updatedAt": "…"
    }
  ],
  "meta": { "source": "little-tasks", "listCount": 1, "taskCount": 1 }
}
```

- **Import**: *Editor → ⚙ Settings → Import JSON*. A file picker selects a bundle.

**Import semantics** (robust):

- Schema version is validated; a newer-than-supported bundle is rejected with a
  friendly error (never a crash).
- Optional fields (`title`, `link`, `comment`, `done`, `meta`) fall back to safe
  defaults; the old snake_case keys (`list_id`, `created_at`, …) are also accepted.
- **Upsert by id using `updated_at` comparison — the newer record wins.**
- Tasks whose list is missing are skipped and counted (`skippedInvalid`); the
  rest import.

A future real sync service implements the same `SyncProvider` trait
(`src-tauri/src/sync/mod.rs`) and swaps in for `LocalFileSync`.

---

## Encryption at rest (design note)

The MVP stores **plaintext** (encrypted-no for now). To avoid a rewrite later:

- All reads/writes go through the `TaskStorage` trait
  (`src-tauri/src/storage/mod.rs`).
- Encryption can be added as a wrapping layer / new `TaskStorage` implementation
  that encrypts rows before writing and decrypts after reading — callers
  (commands, sync, UI) never change.
- Add a key-management step (e.g., OS keychain) when that lands.

---

## Manual smoke test checklist (Fedora 44 / KDE)

1. `npm run dev:desktop`
2. Tray icon appears (top-right panel). Left-click toggles the popover; right-click
   shows the menu with Open Little Tasks / Quick Add / New Task / Quit.
3. Popover quick-add: choose a list, type a title, click *Add task* → row appears.
4. Search: type a term matching title, link, or comment → list filters live.
5. Toggle a checkbox on a row → task gets struck through; restart → still done.
6. Click a row → editor opens (same window reused if already open) with that task.
7. Editor: edit the title, add a Link and a Comment, `Ctrl+S` saves (dirty dot clears).
8. Sidebar: create a list with ＋, rename (✎) and delete (🗑) it; delete confirms
   and removes its tasks.
9. `Ctrl+N` new task, `Ctrl+F` focuses sidebar search.
10. Settings: enable *Start with system* (re-login to verify) and *Show only
    pending tasks* (done tasks disappear from lists).
11. Export JSON → save a file. Delete a task. Import the same file → the task is
    restored (newer `updated_at` wins).
12. Quit via tray → relaunch → all tasks still there (SQLite persistence).
13. Run `cargo test --release` and `npm run test` → all green.

---

## Roadmap / future targets

- **Auto-update**: scaffold-only placeholder (config hook in `tauri.conf.json`
  `plugins.updater` is intentionally left off; add the plugin + a release feed to enable).
- **Mobile (Android/iOS)**: not packaged yet, but `core/`, `storage/`, `sync/`
  avoid desktop-specific APIs so a mobile shell can reuse them.
- **Real sync**: replace `LocalFileSync` with a `SyncProvider` HTTP implementation.
- **Encryption at rest**: wrapping `TaskStorage` implementation (see above).

## License

MIT.
