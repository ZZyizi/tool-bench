use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppsError {
    #[error("failed to read shortcut {path}: {source}")]
    ShortcutRead {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("PowerShell exited with status {0}: {1}")]
    PowerShellFailed(i32, String),
    #[error("PowerShell produced no output")]
    PowerShellEmpty,
    #[error("PowerShell output is not valid JSON: {0}")]
    PowerShellBadJson(String),
}

impl From<AppsError> for String {
    fn from(e: AppsError) -> Self {
        e.to_string()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct InstalledApp {
    /// Stable id derived from the .lnk path so pinned items survive renames
    /// of the display name (and so a single app appearing in multiple start
    /// menu folders is the same id).
    pub id: String,
    pub name: String,
    /// Resolved executable path, or the raw .lnk path when the target cannot
    /// be resolved (e.g. AppExecutionAlias entries).
    pub target: String,
    pub source: String,
    pub icon_index: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstalledApps {
    pub apps: Vec<InstalledApp>,
    pub scanned_at_ms: u128,
}

#[cfg(windows)]
pub fn scan_installed_apps() -> Result<InstalledApps, AppsError> {
    let dirs: Vec<PathBuf> = windows_start_menu_dirs();

    let mut collected: Vec<(PathBuf, String)> = Vec::new();
    for dir in dirs {
        walk_lnk_dir(&dir, &mut collected, 0);
    }

    // Resolve targets via PowerShell. We pass the list of .lnk paths as
    // arguments so the script reads $args instead of embedding paths in the
    // script body — this sidesteps quote-escaping for paths that contain
    // single or double quotes.
    let resolved: Vec<LnkTarget> = if collected.is_empty() {
        Vec::new()
    } else {
        resolve_targets_powershell(
            &collected
                .iter()
                .map(|(p, _)| p.as_path())
                .collect::<Vec<_>>(),
        )?
    };

    let scanned_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    let mut apps: Vec<InstalledApp> = Vec::with_capacity(collected.len());
    for (i, (path, name)) in collected.into_iter().enumerate() {
        let target = resolved
            .get(i)
            .and_then(|t| t.target.clone())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        let icon_index = resolved.get(i).and_then(|t| t.icon_index);
        let id = stable_id_for(&path);
        apps.push(InstalledApp {
            id,
            name,
            target,
            source: path.to_string_lossy().to_string(),
            icon_index,
        });
    }

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps.dedup_by(|a, b| a.id == b.id);

    Ok(InstalledApps { apps, scanned_at_ms })
}

#[cfg(windows)]
fn windows_start_menu_dirs() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    if let Ok(appdata) = std::env::var("APPDATA") {
        // Per-user Start Menu.
        out.push(PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Ok(progdata) = std::env::var("ProgramData") {
        // All-users Start Menu.
        out.push(PathBuf::from(progdata).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    out
}

#[cfg(not(windows))]
pub fn scan_installed_apps() -> Result<InstalledApps, AppsError> {
    // Unix fallback: scan *.desktop files in standard XDG locations. Not part
    // of v0.1 acceptance (v0.1 is Windows-first), but the quick-switcher
    // shouldn't crash on other platforms.
    let mut apps: Vec<InstalledApp> = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        let candidates = [
            PathBuf::from(&home).join(".local/share/applications"),
            PathBuf::from("/usr/share/applications"),
            PathBuf::from("/var/lib/snapd/desktop/applications"),
            PathBuf::from("/usr/local/share/applications"),
        ];
        for dir in candidates {
            if !dir.exists() {
                continue;
            }
            walk_desktop_dir(&dir, &mut apps, 0);
        }
    }
    let scanned_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps.dedup_by(|a, b| a.id == b.id);
    Ok(InstalledApps { apps, scanned_at_ms })
}

#[cfg(not(windows))]
fn walk_desktop_dir(dir: &Path, out: &mut Vec<InstalledApp>, depth: u32) {
    if depth > 4 {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_desktop_dir(&path, out, depth + 1);
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
            continue;
        }
        let raw = match std::fs::read_to_string(&path) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let name = desktop_field(&raw, "Name")
            .or_else(|| desktop_field(&raw, "Name[en]"))
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Unknown")
                    .to_string()
            });
        let exec = desktop_field(&raw, "Exec").unwrap_or_default();
        out.push(InstalledApp {
            id: stable_id_for(&path),
            name,
            target: exec,
            source: path.to_string_lossy().to_string(),
            icon_index: None,
        });
    }
}

#[cfg(not(windows))]
fn desktop_field(content: &str, key: &str) -> Option<String> {
    let prefix = format!("{}=", key);
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix(&prefix) {
            return Some(rest.trim().to_string());
        }
    }
    None
}

fn walk_lnk_dir(dir: &Path, out: &mut Vec<(PathBuf, String)>, depth: u32) {
    // Start Menu trees can nest a few levels deep (e.g. Accessories\System Tools).
    // Cap at 6 to keep scan latency bounded on machines with huge user folders.
    if depth > 6 {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_lnk_dir(&path, out, depth + 1);
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !ext.eq_ignore_ascii_case("lnk") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();
        out.push((path, name));
    }
}

fn stable_id_for(path: &Path) -> String {
    // Lower-case + forward slashes; Windows paths mix both. Hash via FNV-1a
    // 64 — stable across runs, no external crate.
    let normalized = path.to_string_lossy().replace('\\', "/").to_lowercase();
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in normalized.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("app:{:016x}", hash)
}

#[derive(Default, Debug, Clone, Deserialize)]
struct LnkTarget {
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    icon_index: Option<i32>,
}

#[cfg(windows)]
fn resolve_targets_powershell(paths: &[&Path]) -> Result<Vec<LnkTarget>, AppsError> {
    // IMPORTANT: this is invoked as `powershell -File <script.ps1> <paths...>`,
    // NOT as `powershell -Command <script> <paths...>`. With `-Command` plus
    // multiple trailing arguments, PowerShell 5.1 reparses the whole tail as
    // a single script. Parentheses and `$` characters in path tokens then
    // become (32-bit)-style subexpressions and `$Recycle.Bin`-style variable
    // references, which can be denied by restricted execution policy and
    // surface as `PSSecurityException + InvalidResult`. Using `-File` with a
    // `param([string[]]$Links)` declaration binds args to a parameter, which
    // PowerShell does not reparse as code.
    const SCRIPT: &str = r#"
param([Parameter(ValueFromRemainingArguments=$true)][string[]]$Links)
$OutputEncoding = [System.Text.Encoding]::UTF8
$results = @()
foreach ($p in $Links) {
    $sh = New-Object -ComObject WScript.Shell
    try {
        $lnk = $sh.CreateShortcut($p)
        $target = $lnk.TargetPath
        if ([string]::IsNullOrWhiteSpace($target)) { $target = $null }
        $iconIndex = $lnk.IconLocation
        $idx = $null
        if ($iconIndex) {
            $parts = $iconIndex -split ','
            if ($parts.Length -ge 2) {
                $parsed = 0
                if ([int]::TryParse($parts[1].Trim(), [ref]$parsed)) { $idx = $parsed }
            }
        }
        $results += [pscustomobject]@{ target = $target; iconIndex = $idx }
    } catch {
        $results += [pscustomobject]@{ target = $null; iconIndex = $null }
    }
}
$results | ConvertTo-Json -Compress -Depth 3
"#;

    // Write the script to a temp file. Including the pid in the name keeps
    // concurrent processes (e.g. two DevToolkit instances) from clobbering
    // each other, and the name is predictable enough for cleanup.
    let mut script_path = std::env::temp_dir();
    script_path.push(format!("devtoolkit-resolve-{}.ps1", std::process::id()));
    std::fs::write(&script_path, SCRIPT).map_err(|e| AppsError::ShortcutRead {
        path: script_path.to_string_lossy().to_string(),
        source: e,
    })?;

    let script_path_str = script_path.to_string_lossy().to_string();
    let output_res = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &script_path_str,
        ])
        .args(paths.iter().map(|p| p.to_string_lossy().to_string()))
        .output();

    let _ = std::fs::remove_file(&script_path);
    let output = output_res.map_err(|e| AppsError::ShortcutRead {
        path: paths
            .first()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        source: e,
    })?;

    if !output.status.success() {
        return Err(AppsError::PowerShellFailed(
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Err(AppsError::PowerShellEmpty);
    }

    // ConvertTo-Json -Compress emits either a single object { ... } or an
    // array [{...},{...}]. Normalize single-object output to a one-element
    // array so we can deserialize uniformly.
    let json_text = if trimmed.starts_with('[') {
        trimmed.to_string()
    } else {
        format!("[{}]", trimmed)
    };

    let parsed: Vec<LnkTarget> = match serde_json::from_str(&json_text) {
        Ok(v) => v,
        Err(_) => {
            return Err(AppsError::PowerShellBadJson(
                json_text.chars().take(200).collect(),
            ));
        }
    };
    let mut out = parsed;
    while out.len() < paths.len() {
        out.push(LnkTarget::default());
    }
    Ok(out)
}

#[tauri::command]
pub fn list_installed_apps() -> Result<InstalledApps, String> {
    scan_installed_apps().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn launch_app(target: String) -> Result<(), String> {
    if target.trim().is_empty() {
        return Err("target is empty".into());
    }
    // Use cmd /C start so the launched process detaches from us. Without
    // detaching, the parent process holds a handle to the child and the
    // quick-switcher window's close behavior would still be observable
    // through the child. start "" "<target>" returns immediately.
    let status = Command::new("cmd")
        .args(["/C", "start", "", &target])
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("launcher exited with status {:?}", status.code()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_id_is_deterministic_and_path_separator_agnostic() {
        let p1 = Path::new(r"C:\ProgramData\Microsoft\Windows\Start Menu\Programs\Foo\bar.lnk");
        let p2 = Path::new(r"c:/programdata/microsoft/windows/start menu/programs/foo/bar.lnk");
        assert_eq!(stable_id_for(p1), stable_id_for(p2));
        assert!(stable_id_for(p1).starts_with("app:"));
    }

    #[test]
    fn stable_id_differs_for_different_paths() {
        let a = stable_id_for(Path::new("/x/a.lnk"));
        let b = stable_id_for(Path::new("/x/b.lnk"));
        assert_ne!(a, b);
    }

    #[test]
    fn parse_guid_pads_correctly() {
        // Just exercise the helper module through stable_id_for — the only
        // thing we can reliably test without a real filesystem.
        let id = stable_id_for(Path::new(r"C:\a\b.lnk"));
        assert_eq!(id.len(), "app:".len() + 16);
    }
}
