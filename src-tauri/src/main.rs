// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Under a Wayland session, GTK draws its own GNOME-style client-side
    // decorations and ignores the window icon. Running through XWayland lets
    // KWin supply the native KDE titlebar, window borders and taskbar icon.
    if std::env::var("WAYLAND_DISPLAY").is_ok()
        && std::env::var("DISPLAY").is_ok()
        && std::env::var("GDK_BACKEND").is_err()
    {
        std::env::set_var("GDK_BACKEND", "x11");
    }
    little_tasks_lib::run();
}
