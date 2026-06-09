use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const CLOSE_QUIT: u8 = 0;
pub const CLOSE_HIDE: u8 = 1;

#[tauri::command]
pub async fn open_tool_window(
    app: AppHandle,
    plugin_id: String,
    title: Option<String>,
    width: Option<f64>,
    height: Option<f64>,
) -> Result<(), String> {
    let label = format!("tool-{}", plugin_id);
    if let Some(existing) = app.get_webview_window(&label) {
        existing.show().map_err(|e| e.to_string())?;
        existing.unminimize().ok();
        existing.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }
    let window_title = title.unwrap_or_else(|| plugin_id.clone());
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
    builder.build().map_err(|e| e.to_string())?;
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
