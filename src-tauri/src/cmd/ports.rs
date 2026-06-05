use crate::platform::port_scanner::PortInfo;
use serde::Serialize;
use tauri::State;

use crate::AppState;

#[derive(Serialize)]
pub struct KillResult {
    pub success: bool,
    pub pid: u32,
    pub port: u16,
    pub message: String,
}

#[tauri::command]
pub fn list_ports(state: State<'_, AppState>) -> Result<Vec<PortInfo>, String> {
    state.scanner.list().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn kill_port(port: u16, state: State<'_, AppState>) -> Result<KillResult, String> {
    let ports = state
        .scanner
        .list()
        .map_err(|e| e.to_string())?;
    let matching: Vec<&PortInfo> = ports.iter().filter(|p| p.port == port).collect();
    if matching.is_empty() {
        return Ok(KillResult {
            success: false,
            pid: 0,
            port,
            message: format!("No process found listening on port {}", port),
        });
    }
    let target = matching
        .iter()
        .find(|p| p.state.is_empty() || p.state == "LISTEN" || p.state == "LISTENING")
        .copied()
        .unwrap_or(matching[0]);
    match state.scanner.kill(target.pid) {
        Ok(()) => Ok(KillResult {
            success: true,
            pid: target.pid,
            port,
            message: format!("Killed PID {} on port {}", target.pid, port),
        }),
        Err(e) => Ok(KillResult {
            success: false,
            pid: target.pid,
            port,
            message: e.to_string(),
        }),
    }
}
