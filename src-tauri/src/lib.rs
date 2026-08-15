// lib.rs — Little Tasks application entry (used by main.rs and mobile later).

mod backend;
mod core;
mod storage;
mod sync;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("little-tasks.log".into()),
                    }),
                ])
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            // 1) Database open + migrations (with logging + error dialog).
            let db_path = backend::db_path(app.handle()).map_err(|e| {
                log::error!("could not resolve data dir: {e}");
                e
            })?;
            log::info!("opening database at {}", db_path.display());
            let state = match backend::AppState::new(&db_path) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("database open/migrate failed: {e}");
                    use tauri_plugin_dialog::DialogExt;
                    let _ = app
                        .dialog()
                        .message(format!(
                            "Little Tasks could not open its database:\n\n{e}\n\n\
                             Check the log for details and try reinstalling."
                        ))
                        .title("Little Tasks — Startup Error")
                        .kind(tauri_plugin_dialog::MessageDialogKind::Error)
                        .blocking_show();
                    std::process::exit(1);
                }
            };
            app.manage(state);

            // 1b) Apply persisted "start with system" preference (idempotent).
            {
                use crate::backend::commands::SETTING_START_WITH_SYSTEM;
                use crate::storage::TaskStorage;
                let enabled = app
                    .state::<crate::backend::AppState>()
                    .storage
                    .lock()
                    .ok()
                    .and_then(|s| s.get_setting(SETTING_START_WITH_SYSTEM).ok().flatten())
                    .map(|v| v == "1")
                    .unwrap_or(false);
                if enabled {
                    if let Err(e) = crate::backend::commands::apply_autostart(app.handle(), true) {
                        log::warn!("could not apply autostart preference: {e}");
                    }
                }
            }

            // 2) Tray popover window (hidden until tray click).
            backend::windows::create_tray_window(app.handle())?;

            // 3) System tray icon.
            backend::tray::setup_tray(app.handle())?;

            log::info!("Little Tasks is running");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            backend::commands::list_task_lists,
            backend::commands::create_task_list,
            backend::commands::update_task_list,
            backend::commands::delete_task_list,
            backend::commands::get_task_list,
            backend::commands::list_tasks,
            backend::commands::search_tasks,
            backend::commands::get_task,
            backend::commands::create_task,
            backend::commands::update_task,
            backend::commands::delete_task,
            backend::commands::toggle_task,
            backend::commands::export_bundle,
            backend::commands::import_bundle,
            backend::commands::hide_tray,
            backend::commands::take_pending_action,
            backend::commands::open_editor,
            backend::commands::new_task,
            backend::commands::quit_app,
            backend::commands::get_settings,
            backend::commands::set_setting,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Little Tasks");

    app.run(|_app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            log::info!("Little Tasks exiting");
        }
    });
}
