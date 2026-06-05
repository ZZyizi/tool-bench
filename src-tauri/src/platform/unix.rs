use super::port_scanner::{PortError, PortInfo, PortScanner, Protocol};
use std::process::Command;

pub struct UnixPortScanner;

impl PortScanner for UnixPortScanner {
    fn list(&self) -> Result<Vec<PortInfo>, PortError> {
        let output = Command::new("lsof")
            .args(&["-i", "-P", "-n"])
            .output()
            .map_err(PortError::IoError)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if stderr.contains("Permission denied") {
                return Err(PortError::PermissionDenied);
            }
            return Err(PortError::CommandFailed(stderr));
        }
        let text = String::from_utf8_lossy(&output.stdout);
        parse_lsof(&text)
    }

    fn kill(&self, pid: u32) -> Result<(), PortError> {
        let output = Command::new("kill")
            .args(&["-9", &pid.to_string()])
            .output()
            .map_err(PortError::IoError)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if stderr.contains("Operation not permitted") {
                return Err(PortError::PermissionDenied);
            }
            return Err(PortError::CommandFailed(stderr));
        }
        Ok(())
    }
}

pub fn parse_lsof(output: &str) -> Result<Vec<PortInfo>, PortError> {
    // lsof -i -P -n output columns:
    //   0:COMMAND 1:PID 2:USER 3:FD 4:TYPE 5:DEVICE 6:SIZE/OFF 7:NODE 8:NAME 9:(STATE)
    //   - TYPE (col 4) is the address family: IPv4 / IPv6
    //   - NODE (col 7) is the transport protocol: TCP / UDP
    let mut out = Vec::new();
    let mut lines = output.lines();
    let _ = lines.next(); // skip header
    for line in lines {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 9 {
            continue;
        }
        let command = cols[0].to_string();
        let pid: u32 = match cols[1].parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        // Transport protocol lives in NODE column (index 7), not TYPE (index 4)
        let protocol = match cols[7] {
            "TCP" => Protocol::Tcp,
            "UDP" => Protocol::Udp,
            _ => continue,
        };
        // NAME column is index 8: address:port (e.g., "*:8080" or "10.0.0.1:22->1.2.3.4:54321")
        let name = cols[8];
        // State, if present, is index 9 wrapped in parens (e.g., "(LISTEN)")
        let state = if cols.len() > 9 {
            cols[9].trim_matches(|c| c == '(' || c == ')').to_string()
        } else {
            String::new()
        };
        // For ESTABLISHED lines, name is "local:port->remote:port"; take local port (before ->)
        let address = name.split("->").next().unwrap_or(name);
        let port: u16 = match address.rsplit(':').next().and_then(|p| p.parse().ok()) {
            Some(p) => p,
            None => continue,
        };
        out.push(PortInfo {
            protocol,
            port,
            pid,
            state,
            process_name: Some(command),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_listen() {
        let input = "\
COMMAND     PID   USER   FD   TYPE   DEVICE SIZE/OFF NODE NAME
node      1234   user   22u  IPv4   0x1234      0t0  TCP *:8080 (LISTEN)
";
        let ports = parse_lsof(input).unwrap();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port, 8080);
        assert_eq!(ports[0].pid, 1234);
        assert_eq!(ports[0].state, "LISTEN");
        assert_eq!(ports[0].process_name.as_deref(), Some("node"));
    }

    #[test]
    fn parse_established() {
        let input = "\
COMMAND     PID   USER   FD   TYPE   DEVICE SIZE/OFF NODE NAME
curl      5678   user   5u   IPv4   0x5678      0t0  TCP 192.168.1.1:80->1.2.3.4:443 (ESTABLISHED)
";
        let ports = parse_lsof(input).unwrap();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port, 80);
        assert_eq!(ports[0].pid, 5678);
        assert_eq!(ports[0].state, "ESTABLISHED");
    }

    #[test]
    fn parse_skips_invalid() {
        let input = "\
COMMAND     PID   USER   FD   TYPE   DEVICE SIZE/OFF NODE NAME
weird      abc    user   22u  IPv4   0x1        0t0  TCP *:3000 (LISTEN)
node       42     user   22u  IPv4   0x2        0t0  TCP *:4000 (LISTEN)
";
        let ports = parse_lsof(input).unwrap();
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port, 4000);
    }

    #[test]
    fn parse_empty() {
        let ports = parse_lsof("").unwrap();
        assert_eq!(ports.len(), 0);
    }
}
