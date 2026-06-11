use super::port_scanner::{PortError, PortInfo, PortScanner, Protocol};
use std::collections::HashSet;
use std::process::Command;

pub struct WindowsPortScanner;

impl PortScanner for WindowsPortScanner {
    fn list(&self) -> Result<Vec<PortInfo>, PortError> {
        // -b attaches "[process.exe]" lines to LISTENING entries, but requires
        // admin to see non-system processes. We accept the tradeoff: system
        // services will get names, user processes show as None on unelevated runs.
        let output = Command::new("cmd")
            .args(&["/C", "chcp 65001 > nul && netstat -anob"])
            .output()
            .map_err(PortError::IoError)?;
        if !output.status.success() {
            return Err(PortError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        let text = String::from_utf8_lossy(&output.stdout);
        parse_netstat(&text)
    }

    fn kill(&self, pid: u32) -> Result<(), PortError> {
        let output = Command::new("taskkill")
            .args(&["/PID", &pid.to_string(), "/F"])
            .output()
            .map_err(PortError::IoError)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if stderr.contains("Access is denied") {
                return Err(PortError::PermissionDenied);
            }
            return Err(PortError::CommandFailed(stderr));
        }
        Ok(())
    }
}

pub fn parse_netstat(output: &str) -> Result<Vec<PortInfo>, PortError> {
    let mut out = Vec::new();
    let mut seen: HashSet<(Protocol, u16, u32, String)> = HashSet::new();
    // netstat -b attaches up to two trailer lines to each LISTENING row:
    //   line 1: service name (e.g. "RpcSs") or "Cannot obtain ownership information"
    //   line 2: "[executable.exe]"
    // ESTABLISHED/etc. rows have no trailers. The trailer belongs to the
    // *preceding* main row, not the next one — we hold the last main row in
    // pending_row and patch it when we see "[exe]".
    let mut pending_row: Option<PortInfo> = None;

    let flush = |row: Option<PortInfo>, out: &mut Vec<PortInfo>, seen: &mut HashSet<(Protocol, u16, u32, String)>| {
        if let Some(p) = row {
            if seen.insert((p.protocol, p.port, p.pid, p.state.clone())) {
                out.push(p);
            }
        }
    };

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("Active") || line.starts_with("Proto") {
            continue;
        }
        let cols: Vec<&str> = line.split_whitespace().collect();

        let protocol = match cols.first().copied() {
            Some("TCP") => Protocol::Tcp,
            Some("UDP") => Protocol::Udp,
            _ => {
                // Trailer line. Only "[exe]" carries useful info; the service
                // name / "Cannot obtain ownership" line we just ignore.
                if let Some(ref mut row) = pending_row {
                    if line.starts_with('[') && line.ends_with(']') && line.len() >= 2 {
                        row.process_name = Some(line[1..line.len() - 1].to_string());
                    }
                }
                continue;
            }
        };

        if cols.len() < 4 {
            continue;
        }
        let local = cols[1];
        let port = match local.rsplit(':').next() {
            Some(p) => match p.parse::<u16>() {
                Ok(p) => p,
                Err(_) => continue,
            },
            None => continue,
        };
        let state = if protocol == Protocol::Tcp {
            cols.get(3).copied().unwrap_or("").to_string()
        } else {
            String::new()
        };
        let pid: u32 = match cols.last().and_then(|s| s.parse().ok()) {
            Some(p) => p,
            None => continue,
        };

        // A new main row is coming → flush the previous one (with whatever
        // name its trailers gave us).
        flush(pending_row.take(), &mut out, &mut seen);
        pending_row = Some(PortInfo {
            protocol,
            port,
            pid,
            state,
            process_name: None,
        });
    }
    flush(pending_row.take(), &mut out, &mut seen);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tcp_listen() {
        let input = "\
Active Connections

  Proto  Local Address          Foreign Address        State           PID
  TCP    0.0.0.0:135            0.0.0.0:0              LISTENING       1234
  TCP    192.168.1.5:8080       0.0.0.0:0              LISTENING       5678
";
        let ports = parse_netstat(input).unwrap();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].protocol, Protocol::Tcp);
        assert_eq!(ports[0].port, 135);
        assert_eq!(ports[0].pid, 1234);
        assert_eq!(ports[0].state, "LISTENING");
        assert_eq!(ports[1].port, 8080);
        assert_eq!(ports[1].pid, 5678);
    }

    #[test]
    fn parse_udp() {
        let input = "\
  UDP    0.0.0.0:5353          *:*                                    9012
";
        let ports = parse_netstat(input).unwrap();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].protocol, Protocol::Udp);
        assert_eq!(ports[0].port, 5353);
        assert_eq!(ports[0].pid, 9012);
        assert_eq!(ports[0].state, "");
    }

    #[test]
    fn parse_skips_invalid_lines() {
        let input = "\
  TCP    invalid:line           0.0.0.0:0              LISTENING       notapid
  TCP    0.0.0.0:80             0.0.0.0:0              LISTENING       42
";
        let ports = parse_netstat(input).unwrap();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port, 80);
        assert_eq!(ports[0].pid, 42);
    }

    #[test]
    fn parse_empty() {
        let ports = parse_netstat("").unwrap();
        assert_eq!(ports.len(), 0);
    }

    #[test]
    fn parse_dedupes_same_proto_port_pid_state() {
        let input = "\
  TCP    0.0.0.0:8080       0.0.0.0:0              LISTENING       1234
  TCP    [::]:8080          [::]:0                 LISTENING       1234
";
        let ports = parse_netstat(input).unwrap();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port, 8080);
        assert_eq!(ports[0].pid, 1234);
    }

    #[test]
    fn parse_b_flag_attaches_process_name_to_listening() {
        let input = "\
  TCP    0.0.0.0:8080           0.0.0.0:0              LISTENING       1234
 [python.exe]
  TCP    0.0.0.0:8081           0.0.0.0:0              LISTENING       5678
  CDPSvc
 [svchost.exe]
";
        let ports = parse_netstat(input).unwrap();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].port, 8080);
        assert_eq!(ports[0].process_name.as_deref(), Some("python.exe"));
        assert_eq!(ports[1].port, 8081);
        assert_eq!(ports[1].process_name.as_deref(), Some("svchost.exe"));
    }

    #[test]
    fn parse_b_flag_without_permission_yields_none() {
        // No admin: netstat prints "Cannot obtain ownership information" / 服务名
        // line, no "[exe]" trailer. process_name must stay None.
        let input = "\
  TCP    0.0.0.0:445            0.0.0.0:0              LISTENING       4
 无法获取所有者信息
";
        let ports = parse_netstat(input).unwrap();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port, 445);
        assert_eq!(ports[0].pid, 4);
        assert_eq!(ports[0].process_name, None);
    }

    #[test]
    fn parse_b_flag_pending_name_does_not_bleed_to_next_row() {
        // Established row has no trailer; the [name.exe] belongs to the previous
        // LISTENING row, not the ESTABLISHED one.
        let input = "\
  TCP    0.0.0.0:8080           0.0.0.0:0              LISTENING       1234
 [python.exe]
  TCP    192.168.1.1:443        1.2.3.4:1234           ESTABLISHED     9999
";
        let ports = parse_netstat(input).unwrap();
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].process_name.as_deref(), Some("python.exe"));
        assert_eq!(ports[1].process_name, None);
    }
}
