use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder, WindowEvent};

pub const QS_WINDOW_LABEL: &str = "quick-switcher";
pub const QS_DEFAULT_SHORTCUT: &str = "Alt+Space";
/// Grace period after the window is shown or moved during which a transient
/// `Focused(false)` event is ignored. Without this, two distinct OS-level
/// focus resets close the window incorrectly:
///   1. Right after `set_focus()`, webview2's focus-grab race produces a
///      brief blur.
///   2. Right after a drag ends, webview2 rebuilds the focus state and
///      again emits a brief blur.
/// 600ms is comfortably longer than either race in practice but short
/// enough that an intentional click-outside still feels instant.
const BLUR_GRACE: Duration = Duration::from_millis(600);

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

    // Install the "click outside to dismiss" handler once at creation. The
    // toggle branch above never re-registers, which is fine because the
    // listener is bound to the window and persists across show/hide cycles.
    let blur_app = app.clone();
    let last_show = Arc::new(Mutex::new(Instant::now() - BLUR_GRACE * 2));
    let last_show_for_closure = last_show.clone();
    let last_move = Arc::new(Mutex::new(Instant::now() - BLUR_GRACE * 2));
    let last_move_for_closure = last_move.clone();
    window.on_window_event(move |event| {
        match event {
            WindowEvent::Focused(false) => {
                let now = Instant::now();
                // 1) Grace period right after the window was shown — covers
                //    the focus-grab race between set_focus() and first paint.
                let shown = *last_show_for_closure.lock().unwrap();
                if now.duration_since(shown) < BLUR_GRACE {
                    return;
                }
                // 2) Grace period after the window was moved — covers the
                //    spurious blur that Windows can send right after a drag
                //    operation completes. Without this, dragging the window
                //    to a new position closes it.
                let moved = *last_move_for_closure.lock().unwrap();
                if now.duration_since(moved) < BLUR_GRACE {
                    return;
                }
                if let Some(w) = blur_app.get_webview_window(QS_WINDOW_LABEL) {
                    let _ = w.hide();
                    QS_VISIBLE.store(false, Ordering::Relaxed);
                }
            }
            WindowEvent::Moved(_) => {
                // Bump last_move on every drag step so the post-drag grace
                // window extends naturally as the user keeps dragging.
                *last_move_for_closure.lock().unwrap() = Instant::now();
            }
            _ => {}
        }
    });

    center_screen(&window);
    window.show().map_err(|e| e.to_string())?;
    let _ = window.set_focus();
    *last_show.lock().unwrap() = Instant::now();
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
