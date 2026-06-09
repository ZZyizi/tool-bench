use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

pub const QS_WINDOW_LABEL: &str = "quick-switcher";
pub const QS_DEFAULT_SHORTCUT: &str = "Alt+Space";

static QS_VISIBLE: AtomicBool = AtomicBool::new(false);

#[tauri::command]
pub fn open_quick_switcher(app: AppHandle) -> Result<(), String> {
    toggle_or_create(&app)
}

fn toggle_or_create(app: &AppHandle) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window(QS_WINDOW_LABEL) {
        if existing.is_visible().unwrap_or(false) {
            existing.hide().map_err(|e| e.to_string())?;
            QS_VISIBLE.store(false, Ordering::Relaxed);
            return Ok(());
        }
        existing.show().map_err(|e| e.to_string())?;
        let _ = existing.unminimize();
        let _ = existing.set_focus();
        center_screen(&existing);
        QS_VISIBLE.store(true, Ordering::Relaxed);
        return Ok(());
    }

    let url_path = "index.html?window=quick-switcher".to_string();
    let mut builder = WebviewWindowBuilder::new(
        app,
        QS_WINDOW_LABEL,
        WebviewUrl::App(url_path.into()),
    )
    .title("快速启动")
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(true)
    .inner_size(720.0, 380.0)
    .min_inner_size(480.0, 240.0)
    .max_inner_size(960.0, 640.0)
    .focused(true);

    builder = builder.visible(false);
    let window = builder.build().map_err(|e| e.to_string())?;
    center_screen(&window);
    window.show().map_err(|e| e.to_string())?;
    let _ = window.set_focus();
    QS_VISIBLE.store(true, Ordering::Relaxed);
    Ok(())
}

fn center_screen(window: &tauri::WebviewWindow) {
    // Position the window at the geometric center of the primary monitor.
    // All values are physical pixels — `monitor.size()` and `monitor.position()`
    // both already report physical pixels, and `set_position` expects a
    // `PhysicalPosition`, so no scale_factor conversion is required.
    let Ok(monitor) = window.primary_monitor() else {
        return;
    };
    let Some(monitor) = monitor else {
        return;
    };
    let size = window
        .inner_size()
        .unwrap_or(tauri::PhysicalSize::new(720, 380));
    let pos = monitor.position();
    let mon_w = monitor.size().width as i32;
    let mon_h = monitor.size().height as i32;
    let x = pos.x + (mon_w - size.width as i32) / 2;
    let y = pos.y + (mon_h - size.height as i32) / 2;
    let _ = window.set_position(PhysicalPosition::new(x, y));
}
