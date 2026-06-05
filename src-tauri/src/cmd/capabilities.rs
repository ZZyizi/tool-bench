use serde::Serialize;

#[derive(Serialize)]
pub struct Capabilities {
    pub network_read: bool,
    pub process_read: bool,
    pub process_kill: bool,
    pub dns: bool,
    pub file_read: bool,
}

#[tauri::command]
pub fn list_capabilities() -> Capabilities {
    Capabilities {
        network_read: true,
        process_read: true,
        process_kill: true,
        dns: false,
        file_read: false,
    }
}
