use super::port_scanner::{PortError, PortInfo, PortScanner, Protocol};
use std::process::Command;

pub struct WindowsPortScanner;

impl PortScanner for WindowsPortScanner {
    fn list(&self) -> Result<Vec<PortInfo>, PortError> {
        let output = Command::new("cmd")
            .args(&["/C", "chcp 65001 > nul && netstat -ano"])
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
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("Active") || line.starts_with("Proto") {
            continue;
        }
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 4 {
            continue;
        }
        let protocol = match cols[0] {
            "TCP" => Protocol::Tcp,
            "UDP" => Protocol::Udp,
            _ => continue,
        };
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
        out.push(PortInfo {
            protocol,
            port,
            pid,
            state,
            process_name: None,
        });
    }
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
}
