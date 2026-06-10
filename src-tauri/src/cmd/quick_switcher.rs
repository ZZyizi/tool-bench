use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder, WindowEvent};

pub const QS_WINDOW_LABEL: &str = "quick-switcher";
pub const QS_DEFAULT_SHORTCUT: &str = "Alt+Space";

/// Grace period after the window is shown during which a transient
/// `Focused(false)` event is ignored. Covers the focus-grab race
/// between `set_focus()` and first paint in webview2.
const SHOW_GRACE: Duration = Duration::from_millis(600);

/// Delay before hiding on blur. A `Focused(true)` event within this
/// window cancels the hide (user clicked back into QS). A final
/// `is_focused()` check at fire time is a safety net for any
/// focus-restore we missed.
const BLUR_HIDE_DELAY: Duration = Duration::from_millis(200);

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
    // Window was somehow destroyed after pre-creation; re-create it.
    build_qs_window(app, true)
}

/// Pre-create the quick-switcher webview at startup so the first
/// Alt+Space is instant — the webview is already warm.
pub fn precreate(app: &AppHandle) {
    if let Err(e) = build_qs_window(app, false) {
        eprintln!("[toolBench] failed to pre-create quick-switcher: {e}");
    }
}

fn build_qs_window(app: &AppHandle, show: bool) -> Result<(), String> {
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
    .focused(show);

    builder = builder.visible(false);
    let window = builder.build().map_err(|e| e.to_string())?;

    // Swallow Alt+Space-triggered system menu on this window. Same mechanism
    // as the main window — see `windows_hook::subclass`.
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        crate::windows_hook::subclass(hwnd.0 as isize);
    }

    let blur_app = app.clone();
    let last_show = Arc::new(Mutex::new(Instant::now() - SHOW_GRACE * 2));
    let last_show_for_closure = last_show.clone();
    let pending_hide: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
    let pending_hide_for_closure = pending_hide.clone();

    window.on_window_event(move |event| {
        match event {
            WindowEvent::Focused(false) => {
                let now = Instant::now();
                let shown = *last_show_for_closure.lock().unwrap();
                eprintln!("[qs] Focused(false) at {:?}", now);
                if now.duration_since(shown) < SHOW_GRACE {
                    eprintln!("[qs]   → skipped (within SHOW_GRACE)");
                    return;
                }
                *pending_hide_for_closure.lock().unwrap() = Some(now);
                eprintln!("[qs]   → scheduled hide");

                let app_clone = blur_app.clone();
                let pending_clone = pending_hide_for_closure.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(BLUR_HIDE_DELAY);
                    let mut pending = pending_clone.lock().unwrap();
                    let was_pending = pending.take().is_some();
                    drop(pending);
                    eprintln!("[qs] timer fired, was_pending={was_pending}");
                    if was_pending {
                        if let Some(w) = app_clone.get_webview_window(QS_WINDOW_LABEL) {
                            // Final sanity check: if the window regained focus
                            // in the meantime (e.g. user clicked back), leave
                            // it alone. This replaces the old
                            // Focused(true)-cancels-hide path, which was
                            // firing on spurious webview2 focus bounces
                            // (hover, taskbar preview, etc.) and stalling
                            // the hide for arbitrary durations.
                            if w.is_focused().unwrap_or(false) {
                                eprintln!("[qs]   → window is focused, skip hide");
                            } else {
                                let _ = w.hide();
                                QS_VISIBLE.store(false, Ordering::Relaxed);
                                eprintln!("[qs]   → window hidden");
                            }
                        }
                    }
                });
            }
            _ => {}
        }
    });

    if show {
        center_screen(&window);
        window.show().map_err(|e| e.to_string())?;
        let _ = window.set_focus();
        *last_show.lock().unwrap() = Instant::now();
        QS_VISIBLE.store(true, Ordering::Relaxed);
    }
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
