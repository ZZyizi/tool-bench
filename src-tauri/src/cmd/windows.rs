use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

pub const CLOSE_QUIT: u8 = 0;
pub const CLOSE_HIDE: u8 = 1;
const USE_AND_GO_GRACE: Duration = Duration::from_millis(250);

#[tauri::command]
pub async fn open_tool_window(
    app: AppHandle,
    plugin_id: String,
    title: Option<String>,
    width: Option<f64>,
    height: Option<f64>,
    use_and_go: Option<bool>,
) -> Result<(), String> {
    let label = format!("tool-{}", plugin_id);
    if let Some(existing) = app.get_webview_window(&label) {
        existing.show().map_err(|e| e.to_string())?;
        existing.unminimize().ok();
        existing.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }
    let mut window_title = title.unwrap_or_else(|| plugin_id.clone());
    if use_and_go.unwrap_or(false) {
        window_title = format!("{}-快速启动", window_title);
    }
    let url_path = format!("index.html?plugin={}", plugin_id);
    let mut builder = WebviewWindowBuilder::new(
        &app,
        &label,
        WebviewUrl::App(PathBuf::from(url_path)),
    )
    .title(window_title)
    .min_inner_size(640.0, 400.0);
    if let (Some(w), Some(h)) = (width, height) {
        builder = builder.inner_size(w, h);
    } else {
        builder = builder.inner_size(900.0, 600.0);
    }
    let window = builder.build().map_err(|e| e.to_string())?;

    if use_and_go.unwrap_or(false) {
        let app_handle = app.clone();
        let label_for_event = label.clone();
        // A generation counter invalidated on every focus-regain. Each pending
        // close captures the current generation when scheduled; on fire, it only
        // closes if its captured generation is still current. This makes the
        // close robust against transient focus loss during drag / resize /
        // maximize on Windows.
        let generation: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));
        let gen_handler = generation.clone();

        window.on_window_event(move |event| {
            if let WindowEvent::Focused(focused) = event {
                if *focused {
                    gen_handler.fetch_add(1, Ordering::Relaxed);
                    return;
                }
                let gen_inner = gen_handler.clone();
                let app_inner = app_handle.clone();
                let label_inner = label_for_event.clone();
                let captured = gen_handler.load(Ordering::Relaxed);

                std::thread::spawn(move || {
                    std::thread::sleep(USE_AND_GO_GRACE);
                    if gen_inner.load(Ordering::Relaxed) == captured {
                        if let Some(w) = app_inner.get_webview_window(&label_inner) {
                            let _ = w.close();
                        }
                    }
                });
            }
        });
    }

    Ok(())
}

#[tauri::command]
pub fn set_close_behavior(
    state: tauri::State<'_, Arc<AtomicU8>>,
    behavior: String,
) -> Result<(), String> {
    let value = match behavior.as_str() {
        "hide" => CLOSE_HIDE,
        _ => CLOSE_QUIT,
    };
    state.store(value, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub fn close_tool_window(app: AppHandle, plugin_id: String) -> Result<(), String> {
    let label = format!("tool-{}", plugin_id);
    if let Some(w) = app.get_webview_window(&label) {
        w.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}
