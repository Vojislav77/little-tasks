// backend/tray.rs
//
// System tray icon (StatusNotifier/AppIndicator on KDE). Left-click toggles
// the popover; right-click shows the menu.

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle,
};

use crate::backend::windows;

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let open_editor = MenuItem::with_id(app, "open_editor", "Open Little Tasks", true, None::<&str>)?;
    let quick_add = MenuItem::with_id(app, "quick_add", "Quick Add", true, None::<&str>)?;
    let new_task = MenuItem::with_id(app, "new_task", "New Task", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[
            &open_editor,
            &quick_add,
            &new_task,
            &sep,
            &quit,
        ],
    )?;

    let icon = tauri::image::Image::from_bytes(include_bytes!("../../icons/tray-icon.png"))
        .expect("tray icon must be a valid PNG");

    let builder = TrayIconBuilder::with_id("little-tasks-tray")
        .icon(icon)
        .tooltip("Little Tasks")
        .menu(&menu)
        // Left-click should toggle the popover, not open the menu.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open_editor" => windows::open_editor(app, None),
            "quick_add" => windows::toggle_tray_popover(app),
            "new_task" => windows::open_new_task(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                windows::toggle_tray_popover(tray.app_handle());
            }
        });

    builder.build(app)?;
    Ok(())
}
