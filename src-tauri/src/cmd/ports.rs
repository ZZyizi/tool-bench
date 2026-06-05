use crate::platform::port_scanner::{PortInfo, Protocol};
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

fn filter_ports(ports: Vec<PortInfo>, query: &str) -> Vec<PortInfo> {
    let q = query.trim();
    if q.is_empty() {
        return ports;
    }
    // Pure-digit query → exact port match. "8" must NOT match 80/8080/8000.
    let port_query: Option<u16> = q.parse().ok();
    let lower = q.to_lowercase();
    ports
        .into_iter()
        .filter(|p| {
            if let Some(pq) = port_query {
                if p.port == pq {
                    return true;
                }
            }
            p.process_name
                .as_ref()
                .map(|n| n.to_lowercase().contains(&lower))
                .unwrap_or(false)
        })
        .collect()
}

#[tauri::command]
pub fn list_ports(query: String, state: State<'_, AppState>) -> Result<Vec<PortInfo>, String> {
    let ports = state.scanner.list().map_err(|e| e.to_string())?;
    Ok(filter_ports(ports, &query))
}

#[tauri::command]
pub fn kill_port(port: u16, state: State<'_, AppState>) -> Result<KillResult, String> {
    // Intentionally re-list without the search filter: a user may have narrowed
    // the view, but kill should target the actual port regardless of the active
    // query.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_port(port: u16, pid: u32, name: Option<&str>) -> PortInfo {
        PortInfo {
            protocol: Protocol::Tcp,
            port,
            pid,
            state: "LISTEN".to_string(),
            process_name: name.map(String::from),
        }
    }

    #[test]
    fn empty_query_returns_all() {
        let ports = vec![make_port(80, 1, Some("nginx")), make_port(443, 2, None)];
        let r = filter_ports(ports.clone(), "");
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn digit_query_matches_port_exactly() {
        let ports = vec![
            make_port(80, 1, None),
            make_port(8080, 2, None),
            make_port(8000, 3, None),
        ];
        let r = filter_ports(ports, "8080");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].port, 8080);
    }

    #[test]
    fn single_digit_does_not_match_prefix() {
        // "8" must NOT match 80, 8080, 8000 — those are different ports.
        let ports = vec![
            make_port(80, 1, None),
            make_port(8080, 2, None),
            make_port(8000, 3, None),
        ];
        let r = filter_ports(ports, "8");
        assert_eq!(r.len(), 0);
    }

    #[test]
    fn text_query_matches_process_name_substring_case_insensitive() {
        let ports = vec![
            make_port(1, 1, Some("python.exe")),
            make_port(2, 2, Some("Python3")),
            make_port(3, 3, Some("node")),
            make_port(4, 4, None),
        ];
        let r = filter_ports(ports, "python");
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn text_query_skips_entries_with_no_process_name() {
        let ports = vec![
            make_port(80, 1, None),
            make_port(8080, 2, Some("node")),
        ];
        let r = filter_ports(ports, "python");
        assert_eq!(r.len(), 0);
    }
}
