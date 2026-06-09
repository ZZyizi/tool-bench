pub mod cmd;
pub mod platform;

use std::sync::atomic::AtomicU8;
use std::sync::Arc;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};

use crate::cmd::windows::CLOSE_HIDE;
use crate::platform::port_scanner::PortScanner;

pub struct AppState {
    pub scanner: Arc<dyn PortScanner>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let close_behavior = Arc::new(AtomicU8::new(CLOSE_HIDE));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            scanner: platform::create_scanner(),
        })
        .manage(close_behavior.clone())
        .setup(move |app| {
            build_tray(app)?;
            install_main_window_close_handler(app, close_behavior.clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cmd::ports::list_ports,
            cmd::ports::kill_port,
            cmd::capabilities::list_capabilities,
            cmd::windows::open_tool_window,
            cmd::windows::set_close_behavior,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn build_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let show_item =
        MenuItem::with_id(app, "show_main", "显示主窗口", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show_item, &separator, &quit_item])?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(
            app.default_window_icon()
                .cloned()
                .ok_or_else(|| tauri::Error::AssetNotFound("default window icon".into()))?,
        )
        .tooltip("DevToolkit")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show_main" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    let visible = w.is_visible().unwrap_or(false);
                    if visible {
                        let _ = w.hide();
                    } else {
                        let _ = w.show();
                        let _ = w.unminimize();
                        let _ = w.set_focus();
                    }
                }
            }
        })
        .build(app)?;
    Ok(())
}

fn install_main_window_close_handler(
    app: &mut tauri::App,
    close_behavior: Arc<AtomicU8>,
) {
    if let Some(main) = app.get_webview_window("main") {
        let main_for_event = main.clone();
        main.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if close_behavior.load(std::sync::atomic::Ordering::Relaxed) == CLOSE_HIDE {
                    api.prevent_close();
                    if let Err(e) = main_for_event.hide() {
                        eprintln!("[devtoolkit] failed to hide main window: {e}");
                    }
                }
            }
        });
    }
}
