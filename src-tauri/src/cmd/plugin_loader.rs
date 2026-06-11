//! User plugin loader.
//!
//! Scans `<app_config_dir>/user-plugins/<id>/plugin.json` at startup,
//! reads each entry as a string, and returns the parsed manifest + JS
//! source. The frontend creates blob URLs and dynamic-imports them.
//!
//! This is the runtime counterpart to the compile-time glob in
//! `src/plugins/builtin/index.ts` — both feed the same `globalRegistry`.
//! Restart the app to pick up new user plugins; there is no hot-reload.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Manager};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PluginLoaderError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid JSON in {path}: {err}")]
    BadJson { path: String, err: String },
}

impl From<PluginLoaderError> for String {
    fn from(e: PluginLoaderError) -> Self {
        e.to_string()
    }
}

#[derive(Debug, Serialize)]
pub struct UserPluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub category: String,
    pub icon: Option<String>,
    pub capabilities: Vec<String>,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
    pub source: String,
    pub manifest_path: String,
}

pub fn user_plugins_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("user-plugins"))
}

fn read_plugin_dir(plugin_dir: &Path) -> Option<UserPluginInfo> {
    let manifest_path = plugin_dir.join("plugin.json");
    if !manifest_path.is_file() {
        return None;
    }

    let manifest_raw = match fs::read_to_string(&manifest_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "[plugin-loader] {}: read failed: {}",
                manifest_path.display(),
                e
            );
            return None;
        }
    };
    let manifest: serde_json::Value = match serde_json::from_str(&manifest_raw) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "[plugin-loader] {}: invalid JSON: {}",
                manifest_path.display(),
                e
            );
            return None;
        }
    };

    let id = match manifest.get("id").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => {
            eprintln!(
                "[plugin-loader] {}: missing 'id'",
                manifest_path.display()
            );
            return None;
        }
    };

    let entry_rel = manifest
        .get("entry")
        .and_then(|v| v.as_str())
        .unwrap_or("./index.js");
    let entry_rel = entry_rel.trim_start_matches("./");
    let entry_path = plugin_dir.join(entry_rel);
    let source = match fs::read_to_string(&entry_path) {
        Ok(s) => s,
        Err(_) => {
            eprintln!(
                "[plugin-loader] {}: entry {} not found",
                id,
                entry_path.display()
            );
            return None;
        }
    };

    Some(UserPluginInfo {
        id,
        name: manifest
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        version: manifest
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string(),
        description: manifest
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        author: manifest
            .get("author")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        category: manifest
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("Other")
            .to_string(),
        icon: manifest
            .get("icon")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        capabilities: manifest
            .get("capabilities")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
        window_width: manifest
            .get("windowWidth")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
        window_height: manifest
            .get("windowHeight")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
        source,
        manifest_path: manifest_path.to_string_lossy().into_owned(),
    })
}

pub fn scan_dir(dir: &Path) -> Result<Vec<UserPluginInfo>, PluginLoaderError> {
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if !file_type.is_dir() {
            continue;
        }
        if let Some(info) = read_plugin_dir(&entry.path()) {
            out.push(info);
        }
    }
    Ok(out)
}

#[tauri::command]
pub fn scan_user_plugins(app: AppHandle) -> Result<Vec<UserPluginInfo>, String> {
    let dir = user_plugins_dir(&app)?;
    scan_dir(&dir).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn unique_tmp_dir() -> PathBuf {
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("toolbench-loader-test-{pid}-{n}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(p: &Path) {
        let _ = fs::remove_dir_all(p);
    }

    #[test]
    fn missing_dir_returns_empty() {
        let dir =
            std::env::temp_dir().join(format!("does-not-exist-{}", SEQ.fetch_add(1, Ordering::Relaxed)));
        let r = scan_dir(&dir).unwrap();
        assert!(r.is_empty());
    }

    #[test]
    fn empty_dir_returns_empty() {
        let dir = unique_tmp_dir();
        let r = scan_dir(&dir).unwrap();
        assert!(r.is_empty());
        cleanup(&dir);
    }

    #[test]
    fn scan_finds_valid_plugin() {
        let dir = unique_tmp_dir();
        let p = dir.join("my-plugin");
        fs::create_dir(&p).unwrap();
        fs::write(
            p.join("plugin.json"),
            r#"{"id":"my-plugin","name":"My Plugin","version":"0.1.0","description":"d","author":"a","category":"Other"}"#,
        )
        .unwrap();
        fs::write(p.join("index.js"), "export default {};").unwrap();
        let r = scan_dir(&dir).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].id, "my-plugin");
        assert_eq!(r[0].source, "export default {};");
        cleanup(&dir);
    }

    #[test]
    fn scan_skips_dir_without_plugin_json() {
        let dir = unique_tmp_dir();
        fs::create_dir(dir.join("nope")).unwrap();
        fs::write(dir.join("nope/random.txt"), "x").unwrap();
        let r = scan_dir(&dir).unwrap();
        assert!(r.is_empty());
        cleanup(&dir);
    }

    #[test]
    fn scan_skips_dir_with_invalid_manifest() {
        let dir = unique_tmp_dir();
        let p = dir.join("bad");
        fs::create_dir(&p).unwrap();
        fs::write(p.join("plugin.json"), "not json").unwrap();
        let r = scan_dir(&dir).unwrap();
        assert!(r.is_empty());
        cleanup(&dir);
    }

    #[test]
    fn scan_skips_plugin_missing_entry() {
        let dir = unique_tmp_dir();
        let p = dir.join("missing-entry");
        fs::create_dir(&p).unwrap();
        fs::write(
            p.join("plugin.json"),
            r#"{"id":"missing-entry","name":"x","version":"0.1.0","description":"d","author":"a","category":"Other"}"#,
        )
        .unwrap();
        let r = scan_dir(&dir).unwrap();
        assert!(r.is_empty());
        cleanup(&dir);
    }

    #[test]
    fn scan_skips_plugin_missing_id() {
        let dir = unique_tmp_dir();
        let p = dir.join("no-id");
        fs::create_dir(&p).unwrap();
        fs::write(
            p.join("plugin.json"),
            r#"{"name":"x","version":"0.1.0","description":"d","author":"a","category":"Other"}"#,
        )
        .unwrap();
        fs::write(p.join("index.js"), "x").unwrap();
        let r = scan_dir(&dir).unwrap();
        assert!(r.is_empty());
        cleanup(&dir);
    }

    #[test]
    fn scan_parses_optional_fields() {
        let dir = unique_tmp_dir();
        let p = dir.join("rich");
        fs::create_dir(&p).unwrap();
        fs::write(
            p.join("plugin.json"),
            r#"{
              "id": "rich",
              "name": "Rich Plugin",
              "version": "1.2.3",
              "description": "d",
              "author": "a",
              "category": "System",
              "icon": "Box",
              "capabilities": ["fs:read", "fs:write"],
              "windowWidth": 800,
              "windowHeight": 600,
              "entry": "./dist/main.js"
            }"#,
        )
        .unwrap();
        fs::create_dir(p.join("dist")).unwrap();
        fs::write(p.join("dist/main.js"), "console.log('hi');").unwrap();
        let r = scan_dir(&dir).unwrap();
        assert_eq!(r.len(), 1);
        let info = &r[0];
        assert_eq!(info.id, "rich");
        assert_eq!(info.name, "Rich Plugin");
        assert_eq!(info.version, "1.2.3");
        assert_eq!(info.icon.as_deref(), Some("Box"));
        assert_eq!(info.capabilities, vec!["fs:read", "fs:write"]);
        assert_eq!(info.window_width, Some(800));
        assert_eq!(info.window_height, Some(600));
        assert!(info.source.contains("console.log"));
        cleanup(&dir);
    }

    #[test]
    fn scan_finds_multiple_plugins() {
        let dir = unique_tmp_dir();
        for name in ["alpha", "beta", "gamma"] {
            let p = dir.join(name);
            fs::create_dir(&p).unwrap();
            fs::write(
                p.join("plugin.json"),
                format!(
                    r#"{{"id":"{name}","name":"{name}","version":"0.1.0","description":"d","author":"a","category":"Other"}}"#
                ),
            )
            .unwrap();
            fs::write(p.join("index.js"), format!("// {name}")).unwrap();
        }
        // Add a junk dir that should be skipped.
        fs::create_dir(dir.join("junk")).unwrap();
        fs::write(dir.join("junk/random.txt"), "x").unwrap();

        let r = scan_dir(&dir).unwrap();
        assert_eq!(r.len(), 3);
        let ids: Vec<&str> = r.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"alpha"));
        assert!(ids.contains(&"beta"));
        assert!(ids.contains(&"gamma"));
        cleanup(&dir);
    }
}
