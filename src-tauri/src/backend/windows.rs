// backend/windows.rs
//
// Window management: the tray popover and the full editor window.
//
// - Tray popover: small frameless always-on-top window, hidden at launch,
//   shown near the top-right of the primary monitor. Hides on focus loss
//   or Escape.
// - Editor: normal window created on demand; closing it hides instead of
//   destroying so "open note" from the tray always reuses the same window.

use tauri::{
    AppHandle, Manager, PhysicalPosition, Position, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};

pub const TRAY_LABEL: &str = "tray";
pub const EDITOR_LABEL: &str = "editor";

const PENDING_NEW: &str = "__new__";
const PENDING_DEFAULT: &str = "__default__";
pub fn tray_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(TRAY_LABEL)
}

pub fn editor_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(EDITOR_LABEL)
}

pub fn create_tray_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let win = WebviewWindowBuilder::new(app, TRAY_LABEL, WebviewUrl::App("index.html".into()))
        .title("Little Tasks — Quick Add")
        .icon(app.default_window_icon().cloned().expect("app icon configured"))?
        .inner_size(400.0, 620.0)
        .min_inner_size(360.0, 480.0)
        .resizable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(true)
        .visible(false)
        .build()?;

    // Auto-hide when the popover loses focus (clicking elsewhere).
    let app_clone = app.clone();
    win.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event {
            let _ = app_clone.get_webview_window(TRAY_LABEL).map(|w| w.hide());
        }
    });

    Ok(win)
}

pub fn create_editor_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let win = WebviewWindowBuilder::new(app, EDITOR_LABEL, WebviewUrl::App("index.html".into()))
        .title("Little Tasks")
        .icon(app.default_window_icon().cloned().expect("app icon configured"))?
        .inner_size(1100.0, 720.0)
        .min_inner_size(760.0, 500.0)
        .resizable(true)
        .center()
        .build()?;

    // Closing the editor hides it (tray app behavior). Quit from the tray
    // menu or Ctrl+Q actually exits.
    let app_clone = app.clone();
    win.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = app_clone.get_webview_window(EDITOR_LABEL).map(|w| w.hide());
        }
    });

    Ok(win)
}

/// Show the editor window, creating it if needed. Optionally open a task
/// (or start a brand-new task) once the window is ready.
pub fn open_editor(app: &AppHandle, task_id: Option<String>) {
    let pending = match task_id {
        Some(id) => format!("task:{id}"),
        None => PENDING_DEFAULT.to_string(),
    };

    if let Some(win) = editor_window(app) {
        set_pending_action(app, pending);
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    } else {
        set_pending_action(app, pending);
        if let Ok(win) = create_editor_window(app) {
            let _ = win.show();
        }
    }
}

/// Open the editor with no task selected → default view.
pub fn open_new_task(app: &AppHandle) {
    open_editor_with_pending(app, PENDING_NEW.to_string());
}

fn open_editor_with_pending(app: &AppHandle, pending: String) {
    if let Some(win) = editor_window(app) {
        set_pending_action(app, pending);
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    } else {
        set_pending_action(app, pending);
        if let Ok(win) = create_editor_window(app) {
            let _ = win.show();
        }
    }
}

/// Show / hide the tray popover, positioned near the top-right corner of
/// the primary monitor (where the KDE tray usually lives).
pub fn toggle_tray_popover(app: &AppHandle) {
    if let Some(win) = tray_window(app) {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
        } else {
            position_popover(&win);
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

fn position_popover(win: &WebviewWindow) {
    let Ok(Some(monitor)) = win.primary_monitor() else {
        return;
    };
    let scale = monitor.scale_factor();
    let mpos = monitor.position();
    let msize = monitor.size();

    // Logical window size → physical.
    let win_size = win.inner_size().unwrap_or(tauri::PhysicalSize::new(400, 620));
    let ww = win_size.width as f64 / scale;
    let _wh = win_size.height as f64 / scale;

    let margin = 16.0 * scale;
    let x = mpos.x as f64 + (msize.width as f64) - (ww * scale) - margin;
    let y = mpos.y as f64 + margin;

    let _ = win.set_position(Position::Physical(PhysicalPosition::new(x as i32, y as i32)));
}

fn set_pending_action(app: &AppHandle, action: String) {
    if let Some(state) = app.try_state::<crate::backend::AppState>() {
        if let Ok(mut guard) = state.pending_action.lock() {
            *guard = Some(action);
        }
    }
}
