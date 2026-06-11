use crate::platform::port_scanner::{PortInfo, PortScanner};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

const SYSTEM_PROCESS_NAMES: &[&str] = &[
    // Windows
    "system", "svchost.exe", "lsass.exe", "csrss.exe", "services.exe",
    "wininit.exe", "winlogon.exe", "smss.exe",
    // Unix
    "init", "systemd", "kthreadd", "ksoftirqd", "migration",
    "rcu_sched", "watchdog", "launchd",
];

#[derive(Serialize)]
pub struct KillResult {
    pub success: bool,
    pub pid: u32,
    pub port: u16,
    pub message: String,
}

#[derive(Serialize)]
pub struct KillByNameResult {
    pub success: bool,
    pub name: String,
    pub killed: u32,
    pub failed: u32,
    pub message: String,
}

#[derive(Serialize)]
pub struct FilteredPorts {
    pub ports: Vec<PortInfo>,
    pub hidden_system: usize,
}

fn is_system_process(p: &PortInfo) -> bool {
    // PIDs 0..100 cover the well-known kernel/system range on both platforms
    // (Windows: System, CSRSS, LSASS, services; Unix: init/systemd, kthreads).
    if p.pid < 100 {
        return true;
    }
    if let Some(name) = &p.process_name {
        let lower = name.to_lowercase();
        if SYSTEM_PROCESS_NAMES.iter().any(|s| lower == *s) {
            return true;
        }
    }
    false
}

fn hide_system(ports: Vec<PortInfo>) -> (Vec<PortInfo>, usize) {
    let mut visible = Vec::with_capacity(ports.len());
    let mut hidden = 0usize;
    for p in ports {
        if is_system_process(&p) {
            hidden += 1;
        } else {
            visible.push(p);
        }
    }
    (visible, hidden)
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

pub fn list_ports_inner(scanner: &dyn PortScanner, query: &str) -> Result<FilteredPorts, String> {
    let raw = scanner.list().map_err(|e| e.to_string())?;
    let after_query = filter_ports(raw, query);
    let (ports, hidden_system) = hide_system(after_query);
    Ok(FilteredPorts { ports, hidden_system })
}

pub fn kill_port_inner(scanner: &dyn PortScanner, port: u16) -> Result<KillResult, String> {
    // Intentionally re-list without the search filter and without hiding system
    // processes: a user may have narrowed the view, but kill should target the
    // actual port regardless of the active query. (System processes typically
    // refuse kill with PermissionDenied — that surfaces as a failed KillResult
    // in the UI.)
    let ports = scanner.list().map_err(|e| e.to_string())?;
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
    match scanner.kill(target.pid) {
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

pub fn kill_by_process_name_inner(
    scanner: &dyn PortScanner,
    name: &str,
) -> Result<KillByNameResult, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("process name must not be empty".into());
    }
    let ports = scanner.list().map_err(|e| e.to_string())?;
    let pids = pids_matching_name(&ports, trimmed);

    if pids.is_empty() {
        return Ok(KillByNameResult {
            success: false,
            name: trimmed.to_string(),
            killed: 0,
            failed: 0,
            message: format!("No process named \"{}\" is listening on any port", trimmed),
        });
    }

    let mut killed: u32 = 0;
    let mut failed: u32 = 0;
    for pid in &pids {
        match scanner.kill(*pid) {
            Ok(()) => killed += 1,
            Err(_) => failed += 1,
        }
    }
    let success = killed > 0;
    let message = if failed == 0 {
        format!("Killed {} \"{}\" process(es)", killed, trimmed)
    } else {
        format!("Killed {}, failed {} (\"{}\")", killed, failed, trimmed)
    };
    Ok(KillByNameResult {
        success,
        name: trimmed.to_string(),
        killed,
        failed,
        message,
    })
}

fn pids_matching_name(ports: &[PortInfo], name: &str) -> Vec<u32> {
    let mut pids: Vec<u32> = ports
        .iter()
        .filter(|p| p.process_name.as_deref() == Some(name))
        .map(|p| p.pid)
        .collect();
    pids.sort_unstable();
    pids.dedup();
    pids
}

// ---- dispatch wrappers + register ----
//
// The three commands below are no longer individually registered with
// `#[tauri::command]` — they are reached via `dispatch` in cmd/dispatch.rs.
// The frontend calls them through `invoke<...>('dispatch', { name, args })`,
// and the wrappers here translate `serde_json::Value` args into the typed
// inputs the inner functions expect.

#[derive(Deserialize)]
struct ListPortsArgs {
    #[serde(default)]
    query: String,
}

#[derive(Deserialize)]
struct KillPortArgs {
    port: u16,
}

#[derive(Deserialize)]
struct KillByNameArgs {
    name: String,
}

fn list_ports_dispatch(args: Value, scanner: &dyn PortScanner) -> Result<Value, String> {
    let parsed: ListPortsArgs = super::dispatch::parse_args(args)?;
    let result = list_ports_inner(scanner, &parsed.query)?;
    serde_json::to_value(result).map_err(|e| e.to_string())
}

fn kill_port_dispatch(args: Value, scanner: &dyn PortScanner) -> Result<Value, String> {
    let parsed: KillPortArgs = super::dispatch::parse_args(args)?;
    let result = kill_port_inner(scanner, parsed.port)?;
    serde_json::to_value(result).map_err(|e| e.to_string())
}

fn kill_by_process_name_dispatch(args: Value, scanner: &dyn PortScanner) -> Result<Value, String> {
    let parsed: KillByNameArgs = super::dispatch::parse_args(args)?;
    let result = kill_by_process_name_inner(scanner, &parsed.name)?;
    serde_json::to_value(result).map_err(|e| e.to_string())
}

/// Register all ports commands into the dispatch routing table. The scanner
/// is captured into each handler closure so the `dispatch` command (which is
/// the only Tauri command on the registry side) doesn't need to know about
/// AppState.
pub fn register(r: &mut super::dispatch::CommandRegistry, scanner: Arc<dyn PortScanner>) {
    let s1 = scanner.clone();
    r.register("list_ports", move |args| list_ports_dispatch(args, s1.as_ref()));
    let s2 = scanner.clone();
    r.register("kill_port", move |args| kill_port_dispatch(args, s2.as_ref()));
    let s3 = scanner.clone();
    r.register(
        "kill_by_process_name",
        move |args| kill_by_process_name_dispatch(args, s3.as_ref()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::port_scanner::Protocol;

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
        let ports = vec![make_port(80, 1, None), make_port(8080, 2, Some("node"))];
        let r = filter_ports(ports, "python");
        assert_eq!(r.len(), 0);
    }

    #[test]
    fn is_system_process_below_pid_100() {
        assert!(is_system_process(&make_port(80, 4, None)));
        assert!(is_system_process(&make_port(80, 1, Some("init"))));
        assert!(!is_system_process(&make_port(80, 1234, Some("nginx"))));
    }

    #[test]
    fn is_system_process_by_known_name() {
        assert!(is_system_process(&make_port(80, 500, Some("svchost.exe"))));
        assert!(is_system_process(&make_port(80, 500, Some("System"))));
        assert!(is_system_process(&make_port(80, 500, Some("systemd"))));
        assert!(!is_system_process(&make_port(80, 500, Some("node"))));
    }

    #[test]
    fn is_system_process_does_not_match_substring() {
        // "myinit" must NOT be treated as system "init" — exact match only.
        assert!(!is_system_process(&make_port(80, 500, Some("myinit"))));
        assert!(!is_system_process(&make_port(80, 500, Some("customsystemd"))));
    }

    #[test]
    fn is_system_process_with_none_name_and_high_pid() {
        assert!(!is_system_process(&make_port(80, 500, None)));
    }

    #[test]
    fn hide_system_partitions_list() {
        let ports = vec![
            make_port(80, 4, None),                       // hidden: PID < 100
            make_port(443, 500, Some("svchost.exe")),     // hidden: name
            make_port(8080, 1234, Some("nginx")),         // visible
            make_port(9000, 42, Some("anything")),        // hidden: PID < 100
        ];
        let (visible, hidden) = hide_system(ports);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].port, 8080);
        assert_eq!(hidden, 3);
    }

    #[test]
    fn pids_matching_name_dedupes_same_pid_across_ports() {
        let ports = vec![
            make_port(3000, 100, Some("node")),
            make_port(3001, 100, Some("node")),
            make_port(3002, 200, Some("node")),
        ];
        let pids = pids_matching_name(&ports, "node");
        assert_eq!(pids, vec![100, 200]);
    }

    #[test]
    fn pids_matching_name_is_case_sensitive_and_exact() {
        let ports = vec![
            make_port(80, 1, Some("node")),
            make_port(81, 2, Some("Node")),
            make_port(82, 3, Some("node.exe")),
            make_port(83, 4, Some("python")),
            make_port(84, 5, None),
        ];
        assert_eq!(pids_matching_name(&ports, "node"), vec![1]);
        assert_eq!(pids_matching_name(&ports, "Node"), vec![2]);
        assert_eq!(pids_matching_name(&ports, "node.exe"), vec![3]);
        assert!(pids_matching_name(&ports, "missing").is_empty());
    }

    #[test]
    fn pids_matching_name_skips_unknown_process_names() {
        let ports = vec![make_port(80, 1, None), make_port(81, 2, Some("node"))];
        let pids = pids_matching_name(&ports, "node");
        assert_eq!(pids, vec![2]);
    }
}
