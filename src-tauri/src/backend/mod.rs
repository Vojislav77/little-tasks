// backend/mod.rs
//
// Tauri backend glue: shared app state, commands, tray icon and window
// management. Wires together `core`, `storage` and `sync`.

pub mod commands;
pub mod tray;
pub mod windows;

use std::sync::Mutex;

use tauri::Manager;

use crate::storage::sqlite::SqliteStorage;

/// Shared application state handed to every Tauri command.
pub struct AppState {
    pub storage: Mutex<SqliteStorage>,
    /// A pending action (`task:<id>`, `list:<id>` or `__new__`) waiting for
    /// the editor window to claim it when it becomes ready. Avoids a race
    /// between "open editor" and "window finished loading".
    pub pending_action: Mutex<Option<String>>,
}

impl AppState {
    pub fn new(path: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let storage = SqliteStorage::open(path)?;
        Ok(Self {
            storage: Mutex::new(storage),
            pending_action: Mutex::new(None),
        })
    }
}

/// Resolve the on-disk SQLite database path inside the OS app-data dir
/// (respects XDG on Linux, Application Support on macOS, %APPDATA% on
/// Windows — no desktop-specific assumptions beyond those standard paths).
pub fn db_path(app: &tauri::AppHandle) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("little-tasks.db"))
}
