pub mod cmd;
pub mod platform;

use std::sync::Arc;

use crate::platform::port_scanner::PortScanner;

pub struct AppState {
    pub scanner: Arc<dyn PortScanner>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            scanner: platform::create_scanner(),
        })
        .invoke_handler(tauri::generate_handler![
            cmd::ports::list_ports,
            cmd::ports::kill_port,
            cmd::capabilities::list_capabilities,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
