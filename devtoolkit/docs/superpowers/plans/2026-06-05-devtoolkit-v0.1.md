# DevToolkit V0.1 MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement DevToolkit v0.1 MVP: port list + kill + base UI + plugin infrastructure (JS/TS plugin model, port-manager as the first built-in plugin).

**Architecture:** Frontend React app with a plugin system (`Plugin` interface, `PluginRegistry`, `PluginManifest`); one built-in plugin `port-manager`. Rust backend exposes system capabilities as Tauri commands (`list_ports`, `kill_port`, `list_capabilities`) consumed by plugins via `invoke()`. Platform-specific port scanning via `PortScanner` trait, implemented separately for Windows (netstat) and Unix (lsof).

**Tech Stack:** Tauri 2.11, React 19, TypeScript 5.8, Vite 7, Rust 1.95, `thiserror` 2, `serde` 1.

**Reference Spec:** `docs/superpowers/specs/2026-06-05-devtoolkit-v0.1-design.md`

**Working Directory:** `d:\project\tool-bench\devtoolkit` (run all commands from here unless stated otherwise)

---

## File Structure (created/modified during this plan)

**Rust (src-tauri/src/):**
- `lib.rs` — modify: Tauri builder, AppState, register commands
- `Cargo.toml` — modify: add `thiserror` dep
- `cmd/mod.rs` — new: module exports
- `cmd/ports.rs` — new: list_ports, kill_port
- `cmd/capabilities.rs` — new: list_capabilities
- `platform/mod.rs` — new: factory + module exports
- `platform/port_scanner.rs` — new: trait + PortInfo + PortError
- `platform/windows.rs` — new: WindowsPortScanner + netstat parser (with tests)
- `platform/unix.rs` — new: UnixPortScanner + lsof parser (with tests)

**Frontend (src/):**
- `App.tsx` — modify: data-driven layout
- `App.css` — modify: CSS variables + dark theme
- `types.ts` — new: shared TS types
- `components/Sidebar.tsx` — new
- `components/Sidebar.css` — new
- `components/StatusBar.tsx` — new
- `components/ConfirmDialog.tsx` — new
- `components/ConfirmDialog.css` — new
- `plugins/types.ts` — new
- `plugins/registry.ts` — new
- `plugins/context.ts` — new
- `plugins/api.ts` — new
- `plugins/builtin/port-manager/plugin.toml` — new
- `plugins/builtin/port-manager/index.ts` — new
- `plugins/builtin/port-manager/PortView.tsx` — new
- `plugins/builtin/port-manager/PortView.css` — new
- `plugins/builtin/index.ts` — new: registers all built-in plugins

---

## Task 1: Add thiserror dependency to Cargo.toml

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add thiserror to dependencies**

In `src-tauri/Cargo.toml`, append inside the existing `[dependencies]` section (after `serde_json = "1"`):

```toml
thiserror = "2"
```

The full `[dependencies]` block should now read:

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
```

- [ ] **Step 2: Verify dependency resolves**

Run: `cd src-tauri && cargo check 2>&1 | tail -5`
Expected: `Finished` line, no errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "build: add thiserror dependency"
```

---

## Task 2: Define PortInfo, PortError, and PortScanner trait

**Files:**
- Create: `src-tauri/src/platform/port_scanner.rs`
- Create: `src-tauri/src/platform/mod.rs` (empty stub — expanded in Task 6)

- [ ] **Step 1: Create port_scanner.rs**

Create `src-tauri/src/platform/port_scanner.rs`:

```rust
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum Protocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortInfo {
    pub protocol: Protocol,
    pub port: u16,
    pub pid: u32,
    pub state: String,
    pub process_name: Option<String>,
}

#[derive(Debug, Error)]
pub enum PortError {
    #[error("command failed: {0}")]
    CommandFailed(String),
    #[error("parse error: {0}")]
    ParseError(String),
    #[error("permission denied")]
    PermissionDenied,
    #[error("port not found")]
    NotFound,
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}

pub trait PortScanner: Send + Sync {
    fn list(&self) -> Result<Vec<PortInfo>, PortError>;
    fn kill(&self, pid: u32) -> Result<(), PortError>;
}
```

- [ ] **Step 2: Create platform/mod.rs stub**

Create `src-tauri/src/platform/mod.rs`:

```rust
pub mod port_scanner;
```

- [ ] **Step 3: Wire platform module into lib.rs**

Open `src-tauri/src/lib.rs`. Replace its current contents (the `run` function boilerplate from the scaffold) with:

```rust
pub mod platform;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Keep the `tauri_plugin_opener::init()` line; remove any other `tauri::generate_context!` or `tauri::Builder::default()` duplicates from the original scaffold.

- [ ] **Step 4: Verify compile**

Run: `cd src-tauri && cargo check 2>&1 | tail -3`
Expected: compiles cleanly.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/platform/ src-tauri/src/lib.rs
git commit -m "feat(platform): define PortInfo, PortError, PortScanner trait"
```

---

## Task 3: Implement Windows netstat parser (TDD)

**Files:**
- Create: `src-tauri/src/platform/windows.rs`
- Modify: `src-tauri/src/platform/mod.rs`

- [ ] **Step 1: Update platform/mod.rs to include windows module**

Replace `src-tauri/src/platform/mod.rs` with:

```rust
pub mod port_scanner;
pub mod windows;
```

- [ ] **Step 2: Create windows.rs with parser + tests**

Create `src-tauri/src/platform/windows.rs`:

```rust
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
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test --lib platform::windows::tests 2>&1 | tail -10`
Expected: 4 tests pass. (On Unix, `impl PortScanner for WindowsPortScanner` will compile because it's not cfg-gated, but the `Command::new("cmd")` runtime is Windows-only — we never invoke it in tests, only `parse_netstat`.)

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/platform/windows.rs src-tauri/src/platform/mod.rs
git commit -m "feat(platform): Windows port scanner with netstat parser"
```

---

## Task 4: Implement Unix lsof parser (TDD)

**Files:**
- Create: `src-tauri/src/platform/unix.rs`
- Modify: `src-tauri/src/platform/mod.rs`

- [ ] **Step 1: Update platform/mod.rs to include unix module**

Replace `src-tauri/src/platform/mod.rs` with:

```rust
pub mod port_scanner;
#[cfg(windows)]
pub mod windows;
#[cfg(unix)]
pub mod unix;
```

- [ ] **Step 2: Create unix.rs with parser + tests**

Create `src-tauri/src/platform/unix.rs`:

```rust
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
        let type_col = cols[3];
        let protocol = match type_col {
            "IPv4" | "IPv6" => Protocol::Tcp,
            _ => continue,
        };
        let name = cols[cols.len() - 1];
        let (port_str, state) = if let Some(paren_start) = name.find('(') {
            let (n, s) = name.split_at(paren_start);
            (n.trim_end_matches(':'), s.trim_matches(|c| c == '(' || c == ')').to_string())
        } else {
            (name.trim_end_matches(':'), String::new())
        };
        let port: u16 = match port_str.rsplit(':').next() {
            Some(p) => match p.parse() {
                Ok(p) => p,
                Err(_) => continue,
            },
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
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test --lib platform::unix::tests 2>&1 | tail -10`
Expected: 4 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/platform/unix.rs src-tauri/src/platform/mod.rs
git commit -m "feat(platform): Unix port scanner with lsof parser"
```

---

## Task 5: Add platform factory and verify all platform tests

**Files:**
- Modify: `src-tauri/src/platform/mod.rs`

- [ ] **Step 1: Add factory to platform/mod.rs**

Replace `src-tauri/src/platform/mod.rs` with:

```rust
pub mod port_scanner;
#[cfg(windows)]
pub mod windows;
#[cfg(unix)]
pub mod unix;

use port_scanner::PortScanner;
use std::sync::Arc;

#[cfg(windows)]
pub fn create_scanner() -> Arc<dyn PortScanner> {
    Arc::new(windows::WindowsPortScanner)
}

#[cfg(unix)]
pub fn create_scanner() -> Arc<dyn PortScanner> {
    Arc::new(unix::UnixPortScanner)
}
```

- [ ] **Step 2: Run all platform tests**

Run: `cd src-tauri && cargo test --lib platform 2>&1 | tail -10`
Expected: 8 tests pass (4 windows on Windows, 4 unix on Unix — only the relevant suite runs per platform).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/platform/mod.rs
git commit -m "feat(platform): add platform-specific scanner factory"
```

---

## Task 6: Implement list_capabilities command

**Files:**
- Create: `src-tauri/src/cmd/capabilities.rs`
- Create: `src-tauri/src/cmd/mod.rs`

- [ ] **Step 1: Create cmd/capabilities.rs**

Create `src-tauri/src/cmd/capabilities.rs`:

```rust
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
```

- [ ] **Step 2: Create cmd/mod.rs**

Create `src-tauri/src/cmd/mod.rs`:

```rust
pub mod capabilities;
pub mod ports;
```

- [ ] **Step 3: Wire cmd module into lib.rs**

In `src-tauri/src/lib.rs`, add `pub mod cmd;` after `pub mod platform;`:

```rust
pub mod platform;
pub mod cmd;
```

- [ ] **Step 4: Verify compile (will fail — cmd/ports.rs doesn't exist yet)**

Run: `cd src-tauri && cargo check 2>&1 | tail -5`
Expected: error about `cmd::ports` module not found. This is expected; Task 7 will create it.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/cmd/capabilities.rs src-tauri/src/cmd/mod.rs src-tauri/src/lib.rs
git commit -m "feat(cmd): add list_capabilities command"
```

---

## Task 7: Implement list_ports and kill_port commands

**Files:**
- Create: `src-tauri/src/cmd/ports.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create cmd/ports.rs**

Create `src-tauri/src/cmd/ports.rs`:

```rust
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
```

- [ ] **Step 2: Define AppState in lib.rs**

Replace `src-tauri/src/lib.rs` with:

```rust
pub mod cmd;
pub mod platform;

use std::sync::Arc;
use tauri::Manager;

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
```

- [ ] **Step 3: Verify compile**

Run: `cd src-tauri && cargo check 2>&1 | tail -3`
Expected: compiles cleanly.

- [ ] **Step 4: Run all tests**

Run: `cd src-tauri && cargo test 2>&1 | tail -10`
Expected: 8 platform tests pass (4 windows or 4 unix depending on host).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/cmd/ports.rs src-tauri/src/lib.rs
git commit -m "feat(cmd): add list_ports and kill_port commands with AppState"
```

---

## Task 8: Define TypeScript types matching Rust structs

**Files:**
- Create: `src/types.ts`

- [ ] **Step 1: Create the file**

Create `src/types.ts`:

```typescript
export type Protocol = 'Tcp' | 'Udp';

export interface PortInfo {
  protocol: Protocol;
  port: number;
  pid: number;
  state: string;
  process_name: string | null;
}

export interface KillResult {
  success: boolean;
  pid: number;
  port: number;
  message: string;
}

export interface Capabilities {
  network_read: boolean;
  process_read: boolean;
  process_kill: boolean;
  dns: boolean;
  file_read: boolean;
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `npx tsc --noEmit 2>&1 | tail -5`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/types.ts
git commit -m "feat(types): add TS types matching Rust serde structs"
```

---

## Task 9: Define Plugin interface and types

**Files:**
- Create: `src/plugins/types.ts`

- [ ] **Step 1: Create the file**

Create `src/plugins/types.ts`:

```typescript
import type { ComponentType } from 'react';
import type { invoke } from '@tauri-apps/api/core';

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  category: string;
  icon?: string;
  entry: string;
  capabilities?: string[];
}

export interface PluginContext {
  invoke: typeof invoke;
  notify: (message: string, type?: 'info' | 'success' | 'error') => void;
  log: (...args: unknown[]) => void;
}

export interface Plugin {
  manifest: PluginManifest;
  Component?: ComponentType;
  activate: (context: PluginContext) => void | Promise<void>;
  deactivate?: () => void | Promise<void>;
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `npx tsc --noEmit 2>&1 | tail -5`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/plugins/types.ts
git commit -m "feat(plugins): define Plugin and PluginManifest interfaces"
```

---

## Task 10: Implement PluginRegistry

**Files:**
- Create: `src/plugins/registry.ts`

- [ ] **Step 1: Create the file**

Create `src/plugins/registry.ts`:

```typescript
import type { Plugin } from './types';

class PluginRegistry {
  private plugins = new Map<string, Plugin>();

  register(plugin: Plugin): void {
    if (this.plugins.has(plugin.manifest.id)) {
      throw new Error(`Plugin "${plugin.manifest.id}" already registered`);
    }
    this.plugins.set(plugin.manifest.id, plugin);
  }

  unregister(id: string): void {
    this.plugins.delete(id);
  }

  list(): Plugin[] {
    return Array.from(this.plugins.values());
  }

  get(id: string): Plugin | undefined {
    return this.plugins.get(id);
  }

  byCategory(): Map<string, Plugin[]> {
    const grouped = new Map<string, Plugin[]>();
    for (const plugin of this.list()) {
      const list = grouped.get(plugin.manifest.category) ?? [];
      list.push(plugin);
      grouped.set(plugin.manifest.category, list);
    }
    return grouped;
  }
}

export const globalRegistry = new PluginRegistry();
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `npx tsc --noEmit 2>&1 | tail -5`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/plugins/registry.ts
git commit -m "feat(plugins): add PluginRegistry with globalRegistry singleton"
```

---

## Task 11: Implement PluginContext and system API

**Files:**
- Create: `src/plugins/api.ts`
- Create: `src/plugins/context.ts`

- [ ] **Step 1: Create plugins/api.ts**

Create `src/plugins/api.ts`:

```typescript
import { invoke } from '@tauri-apps/api/core';
import type { Capabilities, KillResult, PortInfo } from '../types';

export const api = {
  listPorts: () => invoke<PortInfo[]>('list_ports'),
  killPort: (port: number) => invoke<KillResult>('kill_port', { port }),
  listCapabilities: () => invoke<Capabilities>('list_capabilities'),
};
```

- [ ] **Step 2: Create plugins/context.ts**

Create `src/plugins/context.ts`:

```typescript
import { invoke } from '@tauri-apps/api/core';
import type { PluginContext } from './types';

export const createPluginContext = (): PluginContext => ({
  invoke,
  notify: (message, type = 'info') => {
    const event = new CustomEvent('plugin-notify', { detail: { message, type } });
    window.dispatchEvent(event);
    console.log(`[plugin notify ${type}]`, message);
  },
  log: (...args) => console.log('[plugin]', ...args),
});
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `npx tsc --noEmit 2>&1 | tail -5`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/plugins/api.ts src/plugins/context.ts
git commit -m "feat(plugins): add system API wrapper and PluginContext factory"
```

---

## Task 12: Create port-manager plugin manifest and entry

**Files:**
- Create: `src/plugins/builtin/port-manager/plugin.toml`
- Create: `src/plugins/builtin/port-manager/index.ts`

- [ ] **Step 1: Create plugin.toml**

Create `src/plugins/builtin/port-manager/plugin.toml`:

```toml
[plugin]
id = "port-manager"
name = "端口管理"
version = "0.1.0"
description = "查看和释放系统占用的端口"
author = "DevToolkit Team"
category = "Network"
icon = "🔌"
entry = "./index.ts"

[plugin.capabilities]
required = ["network:read", "process:read", "process:kill"]
```

- [ ] **Step 2: Create index.ts**

Create `src/plugins/builtin/port-manager/index.ts`:

```typescript
import type { Plugin, PluginManifest } from '../../types';
import { PortView } from './PortView';

const manifest: PluginManifest = {
  id: 'port-manager',
  name: '端口管理',
  version: '0.1.0',
  description: '查看和释放系统占用的端口',
  author: 'DevToolkit Team',
  category: 'Network',
  icon: '🔌',
  entry: './index.ts',
  capabilities: ['network:read', 'process:read', 'process:kill'],
};

export const portManagerPlugin: Plugin = {
  manifest,
  Component: PortView,
  activate(ctx) {
    ctx.log('Port manager activated');
  },
};

export default portManagerPlugin;
```

Note: We duplicate manifest in TS (not import the .toml) because V0.1 statically imports plugins. V0.3+ will load manifests from disk.

- [ ] **Step 3: Verify TypeScript compiles (will fail — PortView not yet created)**

Run: `npx tsc --noEmit 2>&1 | tail -5`
Expected: error about `PortView` not existing. This is expected; proceed to Task 13.

- [ ] **Step 4: Commit**

```bash
git add src/plugins/builtin/port-manager/plugin.toml src/plugins/builtin/port-manager/index.ts
git commit -m "feat(plugins): add port-manager plugin manifest and entry"
```

---

## Task 13: Create ConfirmDialog component

**Files:**
- Create: `src/components/ConfirmDialog.css`
- Create: `src/components/ConfirmDialog.tsx`

- [ ] **Step 1: Create components/ConfirmDialog.css**

Create `src/components/ConfirmDialog.css`:

```css
.confirm-dialog__backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.confirm-dialog {
  background: var(--bg-elevated);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 20px 24px;
  min-width: 320px;
  max-width: 480px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
}

.confirm-dialog__title {
  margin: 0 0 12px;
  font-size: 16px;
  font-weight: 600;
}

.confirm-dialog__message {
  margin: 0 0 20px;
  font-size: 14px;
  line-height: 1.5;
  color: var(--fg-muted);
}

.confirm-dialog__actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}

.confirm-dialog__btn {
  padding: 6px 14px;
  border: 1px solid var(--border);
  border-radius: 4px;
  cursor: pointer;
  font-size: 13px;
}

.confirm-dialog__btn--cancel {
  background: transparent;
  color: var(--fg);
}

.confirm-dialog__btn--confirm {
  background: var(--danger);
  color: white;
  border-color: var(--danger);
}
```

- [ ] **Step 2: Create components/ConfirmDialog.tsx**

Create `src/components/ConfirmDialog.tsx`:

```tsx
import './ConfirmDialog.css';

export interface ConfirmDialogProps {
  title: string;
  message: string;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDialog({ title, message, confirmLabel, onConfirm, onCancel }: ConfirmDialogProps) {
  return (
    <div className="confirm-dialog__backdrop" onClick={onCancel}>
      <div className="confirm-dialog" onClick={(e) => e.stopPropagation()}>
        <h3 className="confirm-dialog__title">{title}</h3>
        <p className="confirm-dialog__message">{message}</p>
        <div className="confirm-dialog__actions">
          <button className="confirm-dialog__btn confirm-dialog__btn--cancel" onClick={onCancel}>
            取消
          </button>
          <button className="confirm-dialog__btn confirm-dialog__btn--confirm" onClick={onConfirm}>
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `npx tsc --noEmit 2>&1 | tail -5`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/components/ConfirmDialog.tsx src/components/ConfirmDialog.css
git commit -m "feat(ui): add reusable ConfirmDialog component"
```

---

## Task 14: Implement PortView component

**Files:**
- Create: `src/plugins/builtin/port-manager/PortView.css`
- Create: `src/plugins/builtin/port-manager/PortView.tsx`

- [ ] **Step 1: Create PortView.css**

Create `src/plugins/builtin/port-manager/PortView.css`:

```css
.port-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 16px;
  gap: 12px;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
}

.port-view__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.port-view__title {
  font-size: 18px;
  font-weight: 600;
  margin: 0;
}

.port-view__refresh {
  padding: 6px 12px;
  background: var(--accent);
  color: var(--accent-fg);
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 13px;
}

.port-view__refresh:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.port-view__table {
  flex: 1;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 4px;
}

.port-view__table table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.port-view__table th,
.port-view__table td {
  text-align: left;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border);
}

.port-view__table th {
  background: var(--bg-elevated);
  position: sticky;
  top: 0;
  font-weight: 600;
  z-index: 1;
}

.port-view__row {
  cursor: pointer;
}

.port-view__row:hover {
  background: var(--row-hover);
}

.port-view__row--selected {
  background: var(--row-selected);
}

.port-view__empty,
.port-view__error {
  padding: 24px;
  text-align: center;
  color: var(--fg-muted);
}

.port-view__error {
  color: var(--error);
}

.port-view__footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-top: 8px;
  border-top: 1px solid var(--border);
  font-size: 13px;
}

.port-view__selection {
  color: var(--fg-muted);
}

.port-view__kill {
  padding: 6px 16px;
  background: var(--danger);
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 13px;
}

.port-view__kill:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
```

- [ ] **Step 2: Create PortView.tsx**

Create `src/plugins/builtin/port-manager/PortView.tsx`:

```tsx
import { useState, useEffect, useCallback } from 'react';
import { api } from '../../api';
import { ConfirmDialog } from '../../../components/ConfirmDialog';
import type { PortInfo } from '../../../types';
import './PortView.css';

export function PortView() {
  const [ports, setPorts] = useState<PortInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<PortInfo | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [actionMessage, setActionMessage] = useState<{ kind: 'success' | 'error'; text: string } | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await api.listPorts();
      setPorts(list);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const handleKill = useCallback(async () => {
    if (!selected) return;
    setConfirming(false);
    setActionMessage(null);
    try {
      const result = await api.killPort(selected.port);
      if (result.success) {
        setActionMessage({ kind: 'success', text: result.message });
      } else {
        setActionMessage({ kind: 'error', text: result.message });
      }
      setSelected(null);
      await refresh();
    } catch (e) {
      setActionMessage({ kind: 'error', text: String(e) });
    }
  }, [selected, refresh]);

  return (
    <div className="port-view">
      <div className="port-view__header">
        <h2 className="port-view__title">端口占用列表</h2>
        <button className="port-view__refresh" onClick={refresh} disabled={loading}>
          {loading ? '刷新中…' : '刷新'}
        </button>
      </div>

      {error && <div className="port-view__error">加载失败: {error}</div>}
      {actionMessage && (
        <div className={actionMessage.kind === 'error' ? 'port-view__error' : 'port-view__empty'}>
          {actionMessage.text}
        </div>
      )}

      <div className="port-view__table">
        {ports.length === 0 && !loading && !error ? (
          <div className="port-view__empty">没有检测到端口占用</div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>协议</th>
                <th>端口</th>
                <th>进程</th>
                <th>PID</th>
                <th>状态</th>
              </tr>
            </thead>
            <tbody>
              {ports.map((p) => {
                const isSelected = selected?.port === p.port && selected?.pid === p.pid;
                return (
                  <tr
                    key={`${p.protocol}-${p.port}-${p.pid}`}
                    className={`port-view__row${isSelected ? ' port-view__row--selected' : ''}`}
                    onClick={() => setSelected(p)}
                  >
                    <td>{p.protocol.toUpperCase()}</td>
                    <td>{p.port}</td>
                    <td>{p.process_name ?? '—'}</td>
                    <td>{p.pid}</td>
                    <td>{p.state || (p.protocol === 'Udp' ? '*' : '')}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </div>

      <div className="port-view__footer">
        <span className="port-view__selection">
          {selected
            ? `已选: ${selected.protocol.toUpperCase()} ${selected.port} (PID ${selected.pid})`
            : '点击行选择端口'}
        </span>
        <button
          className="port-view__kill"
          disabled={!selected}
          onClick={() => setConfirming(true)}
        >
          释放端口
        </button>
      </div>

      {confirming && selected && (
        <ConfirmDialog
          title="确认释放端口"
          message={`确定要结束占用端口 ${selected.port} 的进程 (PID ${selected.pid}) 吗？此操作不可撤销。`}
          confirmLabel="确认释放"
          onConfirm={handleKill}
          onCancel={() => setConfirming(false)}
        />
      )}
    </div>
  );
}
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `npx tsc --noEmit 2>&1 | tail -5`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/plugins/builtin/port-manager/PortView.tsx src/plugins/builtin/port-manager/PortView.css
git commit -m "feat(plugins): add PortView component with table and confirm flow"
```

---

## Task 15: Implement Sidebar with data-driven plugin list

**Files:**
- Create: `src/components/Sidebar.tsx`
- Create: `src/components/Sidebar.css`

- [ ] **Step 1: Create components/Sidebar.css**

Create `src/components/Sidebar.css`:

```css
.sidebar {
  width: 220px;
  background: var(--bg-elevated);
  border-right: 1px solid var(--border);
  padding: 16px 0;
  overflow-y: auto;
}

.sidebar__title {
  padding: 0 16px 12px;
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--fg-muted);
  border-bottom: 1px solid var(--border);
  margin: 0 0 8px;
}

.sidebar__group {
  margin-bottom: 12px;
}

.sidebar__group-title {
  padding: 6px 16px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  color: var(--fg-muted);
}

.sidebar__item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  cursor: pointer;
  font-size: 14px;
  border-left: 2px solid transparent;
}

.sidebar__item:hover {
  background: var(--row-hover);
}

.sidebar__item--active {
  background: var(--row-selected);
  border-left-color: var(--accent);
  font-weight: 500;
}

.sidebar__icon {
  font-size: 16px;
}
```

- [ ] **Step 2: Create components/Sidebar.tsx**

Create `src/components/Sidebar.tsx`:

```tsx
import { globalRegistry } from '../plugins/registry';
import './Sidebar.css';

interface SidebarProps {
  activeId: string | null;
  onSelect: (pluginId: string) => void;
}

export function Sidebar({ activeId, onSelect }: SidebarProps) {
  const grouped = globalRegistry.byCategory();

  return (
    <aside className="sidebar">
      <h2 className="sidebar__title">DevToolkit</h2>
      {Array.from(grouped.entries()).map(([category, plugins]) => (
        <div key={category} className="sidebar__group">
          <div className="sidebar__group-title">{category}</div>
          {plugins.map((plugin) => (
            <div
              key={plugin.manifest.id}
              className={`sidebar__item${activeId === plugin.manifest.id ? ' sidebar__item--active' : ''}`}
              onClick={() => onSelect(plugin.manifest.id)}
            >
              {plugin.manifest.icon && <span className="sidebar__icon">{plugin.manifest.icon}</span>}
              <span>{plugin.manifest.name}</span>
            </div>
          ))}
        </div>
      ))}
    </aside>
  );
}
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `npx tsc --noEmit 2>&1 | tail -5`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/components/Sidebar.tsx src/components/Sidebar.css
git commit -m "feat(ui): add data-driven Sidebar rendering plugins by category"
```

---

## Task 16: Implement StatusBar

**Files:**
- Create: `src/components/StatusBar.tsx`

- [ ] **Step 1: Create the file**

Create `src/components/StatusBar.tsx`:

```tsx
import { useEffect, useState } from 'react';
import { globalRegistry } from '../plugins/registry';

export function StatusBar() {
  const [version, setVersion] = useState<string>('');

  useEffect(() => {
    setVersion('0.1.0');
  }, []);

  const count = globalRegistry.list().length;

  return (
    <footer
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 16,
        padding: '6px 16px',
        borderTop: '1px solid var(--border)',
        background: 'var(--bg-elevated)',
        fontSize: 12,
        color: 'var(--fg-muted)',
      }}
    >
      <span>状态: 就绪</span>
      <span>|</span>
      <span>工具数: {count}</span>
      <span>|</span>
      <span>版本: {version}</span>
    </footer>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `npx tsc --noEmit 2>&1 | tail -5`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/components/StatusBar.tsx
git commit -m "feat(ui): add StatusBar with plugin count and version"
```

---

## Task 17: Build the builtin plugin registry and wire App.tsx

**Files:**
- Create: `src/plugins/builtin/index.ts`
- Modify: `src/App.tsx`

- [ ] **Step 1: Create plugins/builtin/index.ts**

Create `src/plugins/builtin/index.ts`:

```typescript
import { globalRegistry } from '../registry';
import { createPluginContext } from '../context';
import { portManagerPlugin } from './port-manager';

export const builtinContext = createPluginContext();

globalRegistry.register(portManagerPlugin);

for (const plugin of globalRegistry.list()) {
  plugin.activate(builtinContext);
}
```

- [ ] **Step 2: Replace App.tsx with data-driven layout**

Replace `src/App.tsx` entirely with:

```tsx
import { useState, useEffect } from 'react';
import { Sidebar } from './components/Sidebar';
import { StatusBar } from './components/StatusBar';
import { globalRegistry } from './plugins/registry';
import './plugins/builtin';
import './App.css';

export default function App() {
  const [activeId, setActiveId] = useState<string | null>(null);
  const [renderKey, setRenderKey] = useState(0);

  useEffect(() => {
    const all = globalRegistry.list();
    if (all.length > 0 && activeId === null) {
      setActiveId(all[0].manifest.id);
      setRenderKey((k) => k + 1);
    }
  }, [activeId]);

  const active = activeId ? globalRegistry.get(activeId) : null;
  const ActiveComponent = active?.Component;

  return (
    <div className="app">
      <Sidebar activeId={activeId} onSelect={setActiveId} />
      <main className="app__main" key={renderKey}>
        {ActiveComponent ? <ActiveComponent /> : <div className="app__empty">选择一个工具开始</div>}
      </main>
      <StatusBar />
    </div>
  );
}
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `npx tsc --noEmit 2>&1 | tail -5`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/plugins/builtin/index.ts src/App.tsx
git commit -m "feat(app): wire data-driven layout with plugin registry"
```

---

## Task 18: Replace App.css with CSS Variables dark theme

**Files:**
- Modify: `src/App.css`

- [ ] **Step 1: Replace the file**

Replace `src/App.css` entirely with:

```css
:root {
  --bg: #1a1a1a;
  --bg-elevated: #242424;
  --fg: #e0e0e0;
  --fg-muted: #888;
  --border: #333;
  --accent: #4a9eff;
  --accent-fg: #ffffff;
  --row-hover: #2a2a2a;
  --row-selected: #1e3550;
  --danger: #d44;
  --error: #f55;
}

* {
  box-sizing: border-box;
}

html, body, #root {
  height: 100%;
  margin: 0;
  padding: 0;
  background: var(--bg);
  color: var(--fg);
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  font-size: 14px;
  overflow: hidden;
}

.app {
  display: grid;
  grid-template-columns: auto 1fr;
  grid-template-rows: 1fr auto;
  height: 100%;
}

.app__main {
  grid-column: 2;
  grid-row: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.app__empty {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--fg-muted);
}
```

- [ ] **Step 2: Verify Vite build succeeds**

Run: `npm run build 2>&1 | tail -10`
Expected: build completes with `dist/` directory created, no errors.

- [ ] **Step 3: Commit**

```bash
git add src/App.css
git commit -m "feat(ui): add dark theme with CSS variables and grid layout"
```

---

## Task 19: Final verification — full test pass and Tauri dev startup

**Files:**
- No new files. Verification only.

- [ ] **Step 1: Run all Rust tests**

Run: `cd src-tauri && cargo test 2>&1 | tail -10`
Expected: 8 platform tests pass (4 windows on Windows, 4 unix on Unix).

- [ ] **Step 2: Run full frontend type-check + build**

Run: `npx tsc --noEmit 2>&1 | tail -5` and `npm run build 2>&1 | tail -5`
Expected: both commands succeed with no errors.

- [ ] **Step 3: Start a test port for manual verification**

In a separate terminal:

```bash
python -m http.server 8765
```

- [ ] **Step 4: Launch Tauri dev (verify startup)**

Run: `npm run tauri dev 2>&1 | head -30`
Expected: Tauri window opens within ~10 seconds. The window should show:
- Sidebar on the left with "Network" group and "端口管理" item
- Clicking it shows the port table; 8765 should appear (from the test server)
- No JS errors in the dev console

After verifying, kill the dev process and the python server.

- [ ] **Step 5: Commit a verification log**

Create `docs/superpowers/plans/2026-06-05-devtoolkit-v0.1-verification.md` with:

```markdown
# V0.1 Implementation Verification

**Date:** 2026-06-05

## Tests
- [x] `cargo test` — 8 platform tests pass
- [x] `npx tsc --noEmit` — no type errors
- [x] `npm run build` — frontend builds

## Manual verification (Tauri dev)
- [x] App launches
- [x] Sidebar shows "Network" → "端口管理"
- [x] Port table loads; test server (port 8765) appears
- [ ] Kill port flow works end-to-end
- [ ] Error handling for protected processes

## Notes
[Any deviations or issues found during verification]
```

Adjust checkboxes based on what you actually verified.

- [ ] **Step 6: Final commit**

```bash
git add docs/superpowers/plans/2026-06-05-devtoolkit-v0.1-verification.md
git commit -m "docs: add v0.1 implementation verification log"
```

---

## Definition of Done

- All checkboxes in this plan are checked
- `cargo test` green
- `npm run build` succeeds
- `npm run tauri dev` launches and the port list + kill flow work
- Plugin infrastructure (`Plugin`, `PluginRegistry`, `PluginManifest`) is in place — adding a second built-in plugin requires only: create a new folder under `src/plugins/builtin/<name>/` with a `plugin.toml`, `index.ts`, `<Name>View.tsx`, and add one line to `src/plugins/builtin/index.ts`. No changes to `App.tsx`, `Sidebar.tsx`, or any component in `components/`.
- Verification log committed

## Out of Scope (later milestones)

- V0.2: search/filter, additional built-in plugins (base64, dns, json)
- V0.3: dynamic plugin loading from `~/.devtoolkit/plugins/`, local browse
- V0.4: remote marketplace, signature verification, capability enforcement
