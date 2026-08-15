// backend/commands.rs
//
// Tauri commands exposed to the web UI. These are the ONLY surface the
// frontend talks to. Every mutation bumps `updated_at`, persists to
// SQLite, and broadcasts a `tasks-changed` event so all open windows
// refresh their lists.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::backend::AppState;
use crate::core::task::{self, Task, TaskList};
use crate::storage::{sqlite::SqliteStorage, TaskStorage};
use crate::sync::bundle::ImportSummary;
use crate::sync::local_file::LocalFileSync;
use crate::sync::SyncProvider;

pub const EVENT_TASKS_CHANGED: &str = "tasks-changed";
/// Broadcast to all windows whenever any setting changes.
pub const EVENT_SETTINGS_CHANGED: &str = "settings-changed";

pub const SETTING_START_WITH_SYSTEM: &str = "start_with_system";
pub const SETTING_SHOW_PENDING_ONLY: &str = "show_pending_only";

fn storage<'a, 'b>(
    state: &'a State<'b, AppState>,
) -> Result<std::sync::MutexGuard<'a, SqliteStorage>, String> {
    state
        .storage
        .lock()
        .map_err(|_| "internal: storage lock poisoned".to_string())
}

fn emit_tasks_changed(app: &AppHandle) {
    let _ = app.emit(EVENT_TASKS_CHANGED, ());
}

// ---------------------------------------------------------------------------
// Task lists CRUD
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_task_lists(state: State<AppState>) -> Result<Vec<TaskList>, String> {
    storage(&state)?
        .list_task_lists()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_task_list(state: State<AppState>, id: String) -> Result<Option<TaskList>, String> {
    storage(&state)?.get_task_list(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_task_list(
    app: AppHandle,
    state: State<AppState>,
    title: String,
) -> Result<TaskList, String> {
    let now = task::now_iso8601();
    let list = TaskList {
        id: uuid::Uuid::new_v4().to_string(),
        title,
        created_at: now.clone(),
        updated_at: now,
    };
    storage(&state)?
        .create_task_list(&list)
        .map_err(|e| e.to_string())?;
    emit_tasks_changed(&app);
    Ok(list)
}

#[tauri::command]
pub fn update_task_list(
    app: AppHandle,
    state: State<AppState>,
    list: TaskList,
) -> Result<TaskList, String> {
    let mut l = list;
    l.updated_at = task::now_iso8601();
    task::validate_task_list(&l).map_err(|e| e.to_string())?;
    storage(&state)?
        .update_task_list(&l)
        .map_err(|e| e.to_string())?;
    emit_tasks_changed(&app);
    Ok(l)
}

#[tauri::command]
pub fn delete_task_list(app: AppHandle, state: State<AppState>, id: String) -> Result<bool, String> {
    let deleted = storage(&state)?
        .delete_task_list(&id)
        .map_err(|e| e.to_string())?;
    emit_tasks_changed(&app);
    Ok(deleted)
}

// ---------------------------------------------------------------------------
// Tasks CRUD
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_tasks(
    state: State<AppState>,
    list_id: Option<String>,
) -> Result<Vec<Task>, String> {
    storage(&state)?
        .list_tasks(list_id.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_tasks(state: State<AppState>, query: String) -> Result<Vec<Task>, String> {
    storage(&state)?
        .search_tasks(&query)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_task(state: State<AppState>, id: String) -> Result<Option<Task>, String> {
    storage(&state)?.get_task(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_task(
    app: AppHandle,
    state: State<AppState>,
    list_id: String,
    title: String,
    link: Option<String>,
    comment: Option<String>,
) -> Result<Task, String> {
    let now = task::now_iso8601();
    let task = Task {
        id: uuid::Uuid::new_v4().to_string(),
        list_id,
        title,
        done: false,
        link: link.unwrap_or_default(),
        comment: comment.unwrap_or_default(),
        created_at: now.clone(),
        updated_at: now,
    };
    storage(&state)?
        .create_task(&task)
        .map_err(|e| e.to_string())?;
    emit_tasks_changed(&app);
    Ok(task)
}

/// Full save: bump `updated_at`, validate, upsert.
#[tauri::command]
pub fn update_task(app: AppHandle, state: State<AppState>, task: Task) -> Result<Task, String> {
    let mut t = task;
    t.updated_at = task::now_iso8601();
    task::validate_task(&t).map_err(|e| e.to_string())?;
    storage(&state)?
        .update_task(&t)
        .map_err(|e| e.to_string())?;
    emit_tasks_changed(&app);
    Ok(t)
}

#[tauri::command]
pub fn delete_task(app: AppHandle, state: State<AppState>, id: String) -> Result<bool, String> {
    let deleted = storage(&state)?
        .delete_task(&id)
        .map_err(|e| e.to_string())?;
    emit_tasks_changed(&app);
    Ok(deleted)
}

#[tauri::command]
pub fn toggle_task(app: AppHandle, state: State<AppState>, id: String) -> Result<Task, String> {
    let mut s = storage(&state)?;
    let mut task = s
        .get_task(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("task {id} not found"))?;
    task.done = !task.done;
    task.updated_at = task::now_iso8601();
    task::validate_task(&task).map_err(|e| e.to_string())?;
    s.update_task(&task).map_err(|e| e.to_string())?;
    emit_tasks_changed(&app);
    Ok(task)
}

// ---------------------------------------------------------------------------
// Export / Import (data portability)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn export_bundle(
    state: State<'_, AppState>,
    path: String,
) -> Result<ExportResult, String> {
    let app_version = env!("CARGO_PKG_VERSION").to_string();
    let sync = LocalFileSync::new();
    let guard = storage(&state)?;
    let task_count = guard.list_tasks(None).map_err(|e| e.to_string())?.len();
    let list_count = guard.list_task_lists().map_err(|e| e.to_string())?.len();
    sync.export(&*guard, &app_version, &path)
        .map_err(|e| e.to_string())?;
    Ok(ExportResult {
        path,
        task_count,
        list_count,
    })
}

#[tauri::command]
pub async fn import_bundle(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<ImportSummary, String> {
    let sync = LocalFileSync::new();
    let mut guard = storage(&state)?;
    let summary = sync
        .import(&mut *guard, &path)
        .map_err(|e| e.to_string())?;
    drop(guard);
    emit_tasks_changed(&app);
    Ok(summary)
}

// ---------------------------------------------------------------------------
// Window / tray helpers
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn hide_tray(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(crate::backend::windows::TRAY_LABEL) {
        let _ = win.hide();
    }
    Ok(())
}

/// Open the editor focused on a specific task (from the tray list).
#[tauri::command]
pub fn open_editor(app: AppHandle, task_id: Option<String>) -> Result<(), String> {
    crate::backend::windows::open_editor(&app, task_id);
    let _ = app
        .get_webview_window(crate::backend::windows::TRAY_LABEL)
        .map(|w| w.hide());
    Ok(())
}

/// Open the editor with a fresh, empty task input.
#[tauri::command]
pub fn new_task(app: AppHandle) -> Result<(), String> {
    crate::backend::windows::open_new_task(&app);
    let _ = app
        .get_webview_window(crate::backend::windows::TRAY_LABEL)
        .map(|w| w.hide());
    Ok(())
}

/// Claim a pending action left for the editor window.
#[tauri::command]
pub fn take_pending_action(state: State<AppState>) -> Result<Option<String>, String> {
    let mut guard = state
        .pending_action
        .lock()
        .map_err(|_| "internal: pending action lock poisoned".to_string())?;
    Ok(guard.take())
}

#[tauri::command]
pub fn quit_app(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub start_with_system: bool,
    pub show_pending_only: bool,
}

fn parse_bool_setting(value: Option<String>) -> bool {
    value.as_deref() == Some("1")
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<AppSettings, String> {
    let s = storage(&state)?;
    Ok(AppSettings {
        start_with_system: parse_bool_setting(
            s.get_setting(SETTING_START_WITH_SYSTEM)
                .map_err(|e| e.to_string())?,
        ),
        show_pending_only: parse_bool_setting(
            s.get_setting(SETTING_SHOW_PENDING_ONLY)
                .map_err(|e| e.to_string())?,
        ),
    })
}

#[tauri::command]
pub fn set_setting(
    app: AppHandle,
    state: State<AppState>,
    key: String,
    value: String,
) -> Result<AppSettings, String> {
    let normalized = if value == "1" || value.eq_ignore_ascii_case("true") {
        "1"
    } else {
        "0"
    };

    // Apply the side-effect first so a failure here can't leave the persisted
    // setting (and the checkbox) out of sync with reality.
    if key == SETTING_START_WITH_SYSTEM {
        apply_autostart(&app, normalized == "1")?;
    }

    if let Err(e) = storage(&state)?.set_setting(&key, normalized) {
        // Persisting failed: roll back the autostart change we just made.
        if key == SETTING_START_WITH_SYSTEM {
            let _ = apply_autostart(&app, normalized != "1");
        }
        return Err(e.to_string());
    }

    let settings = get_settings(state)?;
    let _ = app.emit(EVENT_SETTINGS_CHANGED, &settings);
    Ok(settings)
}

/// Apply the "start with system" preference.
///
/// On Linux the upstream `auto-launch` crate (used by
/// `tauri-plugin-autostart`) writes the autostart `.desktop` entry with an
/// unquoted `Exec=` line, which silently fails whenever the app path contains
/// spaces (e.g. an AppImage named "Little Tasks_*.AppImage"). We write that
/// entry ourselves with a properly quoted `Exec=` instead.
pub(crate) fn apply_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        linux_apply_autostart(app, enabled)
    }
    #[cfg(not(target_os = "linux"))]
    {
        use tauri_plugin_autostart::ManagerExt;
        let manager = app.autolaunch();
        if enabled {
            manager.enable().map_err(|e| e.to_string())
        } else {
            manager.disable().map_err(|e| e.to_string())
        }
    }
}

#[cfg(target_os = "linux")]
fn linux_apply_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    // Prefer the AppImage path (spaces and all) so the entry keeps working
    // wherever the AppImage lives; fall back to the current executable.
    let app_path = app
        .env()
        .appimage
        .and_then(|p| p.to_str().map(String::from))
        .or_else(|| std::env::current_exe().ok().map(|p| p.display().to_string()))
        .ok_or_else(|| "could not resolve the app path for autostart".to_string())?;

    let dir = app
        .path()
        .home_dir()
        .map(|h| h.join(".config").join("autostart"))
        .map_err(|e| e.to_string())?;

    // Same filename the plugin would use, so we can clean up any prior entry.
    let entry = dir.join(format!("{}.desktop", app.package_info().name));

    if !enabled {
        if entry.exists() {
            std::fs::remove_file(&entry).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }

    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let data = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Version=1.0\n\
         Name={}\n\
         Comment=Launch Little Tasks when you log in\n\
         Exec=\"{}\"\n\
         StartupNotify=false\n\
         Terminal=false\n\
         X-GNOME-Autostart-enabled=true\n",
        app.package_info().name,
        escape_desktop_exec(&app_path),
    );
    std::fs::write(&entry, data).map_err(|e| e.to_string())?;
    Ok(())
}

/// Escape a path for use inside a quoted `Exec=` value, per the Desktop Entry
/// Specification (`%` must become `%%`, `"` must become `\"`).
#[cfg(target_os = "linux")]
fn escape_desktop_exec(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('%', "%%")
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub path: String,
    pub task_count: usize,
    pub list_count: usize,
}
