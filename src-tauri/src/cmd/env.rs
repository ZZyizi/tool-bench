use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EnvError {
    #[error("PowerShell exited with status {0}: {1}")]
    PowerShellFailed(i32, String),
    #[error("PowerShell produced no output")]
    PowerShellEmpty,
    #[error("PowerShell output is not valid JSON: {0}")]
    PowerShellBadJson(String),
    #[error("preset {preset} detection failed in {dir}: {reason}")]
    PresetDetectionFailed {
        preset: &'static str,
        dir: String,
        reason: String,
    },
    #[error("invalid environment variable name: {0}")]
    InvalidVarName(String),
    #[error("environment variable editor is not supported on this platform")]
    UnsupportedOnThisPlatform,
    #[error("writing to {scope} scope requires administrator privileges")]
    PermissionDenied { scope: &'static str },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<EnvError> for String {
    fn from(e: EnvError) -> Self {
        e.to_string()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VarSource {
    User,
    Process,
    System,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    User,
    System,
}

impl Scope {
    pub fn as_ps_str(self) -> &'static str {
        match self {
            Scope::User => "User",
            Scope::System => "Machine",
        }
    }

    pub fn as_label(self) -> &'static str {
        match self {
            Scope::User => "用户",
            Scope::System => "系统",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
    pub source: VarSource,
    pub scope: Scope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvSnapshot {
    pub vars: Vec<EnvVar>,
    pub path_user: Vec<String>,
    pub path_system: Vec<String>,
    pub warnings: Vec<String>,
    pub captured_at_ms: u128,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PresetKind {
    Java,
    Python,
    Node,
    Go,
    Rust,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVarSpec {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetPlan {
    pub preset: PresetKind,
    pub scope: Scope,
    pub vars: Vec<EnvVarSpec>,
    pub path_prepend: Vec<String>,
    pub path_append: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetResult {
    pub preset: PresetKind,
    pub plan: PresetPlan,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyResult {
    pub applied: Vec<String>,
    pub warnings: Vec<String>,
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

pub fn validate_var_name(name: &str) -> Result<(), EnvError> {
    if name.is_empty() {
        return Err(EnvError::InvalidVarName(
            "variable name cannot be empty".into(),
        ));
    }
    if name.contains('=') {
        return Err(EnvError::InvalidVarName(
            "variable name cannot contain '='".into(),
        ));
    }
    if name.chars().next().unwrap().is_ascii_digit() {
        return Err(EnvError::InvalidVarName(
            "variable name cannot start with a digit".into(),
        ));
    }
    for c in name.chars() {
        if c.is_whitespace() {
            return Err(EnvError::InvalidVarName(
                "variable name cannot contain whitespace".into(),
            ));
        }
    }
    Ok(())
}

// -------------------- Windows-specific impl --------------------

#[cfg(windows)]
fn run_powershell(script: &str) -> Result<String, EnvError> {
    let mut script_path = std::env::temp_dir();
    script_path.push(format!("toolbench-env-{}.ps1", std::process::id()));
    std::fs::write(&script_path, script)?;

    let script_path_str = script_path.to_string_lossy().to_string();
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &script_path_str,
        ])
        .output()?;

    let _ = std::fs::remove_file(&script_path);

    if !output.status.success() {
        return Err(EnvError::PowerShellFailed(
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return Err(EnvError::PowerShellEmpty);
    }
    Ok(trimmed.to_string())
}

#[cfg(windows)]
fn read_env_snapshot() -> Result<EnvSnapshot, EnvError> {
    // Read three sources and merge by name:
    //   process: Get-ChildItem Env: (inherited, includes HKCU-expanded values
    //            already plus any per-process modifications)
    //   user:    HKCU\Environment (persisted user env)
    //   system:  HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment
    //            (persisted system env)
    // Each persisted var wins over the process view; user wins over system
    // when the same name exists in both. PATH is split per scope so the
    // front-end can render/edit User PATH and System PATH independently.
    const SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$process = @{}
$user = @{}
$system = @{}
$warnings = @()

try {
  Get-ChildItem Env: -ErrorAction Stop | ForEach-Object {
    $process[$_.Name] = $_.Value
  }
} catch {
  $warnings += "process env read failed: $_"
}

try {
  $hkcu = Get-ItemProperty -Path 'HKCU:\Environment' -ErrorAction Stop
  $hkcu.PSObject.Properties | Where-Object { $_.MemberType -eq 'NoteProperty' } | ForEach-Object {
    $user[$_.Name] = [string]$_.Value
  }
} catch {
  $warnings += "HKCU\Environment read failed: $_"
}

try {
  $hklm = Get-ItemProperty -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment' -ErrorAction Stop
  $hklm.PSObject.Properties | Where-Object { $_.MemberType -eq 'NoteProperty' } | ForEach-Object {
    $system[$_.Name] = [string]$_.Value
  }
} catch {
  $warnings += "HKLM system env read failed (likely no admin): $_"
}

$vars = @()

# System-persisted first (lowest priority, but listed first in the UI)
foreach ($k in ($system.Keys | Sort-Object)) {
  if (-not $user.ContainsKey($k)) {
    $vars += @{ name = $k; value = $system[$k]; source = 'system'; scope = 'system' }
  }
}

# Process-only (inherited, not persisted) — attributed to user scope
foreach ($k in ($process.Keys | Sort-Object)) {
  if (-not $user.ContainsKey($k) -and -not $system.ContainsKey($k)) {
    $vars += @{ name = $k; value = $process[$k]; source = 'process'; scope = 'user' }
  }
}

# User-persisted wins over system and process
foreach ($k in ($user.Keys | Sort-Object)) {
  $vars += @{ name = $k; value = $user[$k]; source = 'user'; scope = 'user' }
}

function Split-Path-Entries([string]$s) {
  if (-not $s) { return @() }
  return @($s -split ';' | Where-Object { $_ -ne '' })
}

$pathUser = Split-Path-Entries($user['Path'])
$pathSystem = Split-Path-Entries($system['Path'])

$out = @{
  vars = $vars
  path_user = $pathUser
  path_system = $pathSystem
  warnings = $warnings
  captured_at_ms = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
}
$out | ConvertTo-Json -Compress -Depth 5
"#;
    let raw = run_powershell(SCRIPT)?;
    let parsed: RawSnapshot = serde_json::from_str(&raw)
        .map_err(|_| EnvError::PowerShellBadJson(raw.chars().take(200).collect()))?;
    let vars = parsed
        .vars
        .into_iter()
        .filter_map(|v| {
            let (source, scope) = match (v.source.as_str(), v.scope.as_str()) {
                ("user", s) => (VarSource::User, scope_from_str(s)?),
                ("process", s) => (VarSource::Process, scope_from_str(s)?),
                ("system", "system") => (VarSource::System, Scope::System),
                _ => return None,
            };
            Some(EnvVar {
                name: v.name,
                value: v.value,
                source,
                scope,
            })
        })
        .collect();
    Ok(EnvSnapshot {
        vars,
        path_user: parsed.path_user,
        path_system: parsed.path_system,
        warnings: parsed.warnings,
        captured_at_ms: now_ms(),
    })
}

#[cfg(windows)]
fn scope_from_str(s: &str) -> Option<Scope> {
    match s {
        "user" => Some(Scope::User),
        "system" => Some(Scope::System),
        _ => None,
    }
}

#[cfg(windows)]
#[derive(Deserialize)]
struct RawSnapshot {
    vars: Vec<RawVar>,
    path_user: Vec<String>,
    path_system: Vec<String>,
    warnings: Vec<String>,
    #[allow(dead_code)]
    captured_at_ms: u128,
}

#[cfg(windows)]
#[derive(Deserialize)]
struct RawVar {
    name: String,
    value: String,
    source: String,
    scope: String,
}

#[cfg(windows)]
fn set_var(scope: Scope, name: &str, value: &str) -> Result<(), EnvError> {
    validate_var_name(name)?;
    let ps_scope = scope.as_ps_str();
    const SCRIPT: &str = r#"
param([string]$Name, [string]$Value, [string]$Scope)
[Environment]::SetEnvironmentVariable($Name, $Value, $Scope)
"#;
    run_powershell_with_args(SCRIPT, &[name, value, ps_scope])?;
    Ok(())
}

#[cfg(windows)]
fn delete_var(scope: Scope, name: &str) -> Result<(), EnvError> {
    validate_var_name(name)?;
    let ps_scope = scope.as_ps_str();
    const SCRIPT: &str = r#"
param([string]$Name, [string]$Scope)
[Environment]::SetEnvironmentVariable($Name, $null, $Scope)
"#;
    run_powershell_with_args(SCRIPT, &[name, ps_scope])?;
    Ok(())
}

#[cfg(windows)]
fn set_path_entries(scope: Scope, entries: &[String]) -> Result<(), EnvError> {
    let joined = entries.join(";");
    let ps_scope = scope.as_ps_str();
    const SCRIPT: &str = r#"
param([string]$Joined, [string]$Scope)
[Environment]::SetEnvironmentVariable('Path', $Joined, $Scope)
"#;
    run_powershell_with_args(SCRIPT, &[&joined, ps_scope])?;
    Ok(())
}

#[cfg(windows)]
fn run_powershell_with_args(script: &str, args: &[&str]) -> Result<String, EnvError> {
    let mut script_path = std::env::temp_dir();
    script_path.push(format!("toolbench-env-{}.ps1", std::process::id()));
    std::fs::write(&script_path, script)?;

    let script_path_str = script_path.to_string_lossy().to_string();
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &script_path_str,
        ])
        .args(args)
        .output()?;

    let _ = std::fs::remove_file(&script_path);

    if !output.status.success() {
        return Err(EnvError::PowerShellFailed(
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// ---- preset detection ----

fn preset_name(kind: PresetKind) -> &'static str {
    match kind {
        PresetKind::Java => "java",
        PresetKind::Python => "python",
        PresetKind::Node => "node",
        PresetKind::Go => "go",
        PresetKind::Rust => "rust",
    }
}

#[cfg(windows)]
fn detect_preset_inner(kind: PresetKind, dir: &Path) -> Result<PresetPlan, EnvError> {
    match kind {
        PresetKind::Java => detect_java(dir),
        PresetKind::Python => detect_python(dir),
        PresetKind::Node => detect_node(dir),
        PresetKind::Go => detect_go(dir),
        PresetKind::Rust => detect_rust(dir),
    }
}

#[cfg(windows)]
fn detect_java(dir: &Path) -> Result<PresetPlan, EnvError> {
    let bin = resolve_with_bin_descent(dir, "java.exe")?;
    let plan = PresetPlan {
        preset: PresetKind::Java,
        scope: Scope::User,
        vars: vec![EnvVarSpec {
            name: "JAVA_HOME".into(),
            value: bin.parent().unwrap_or(dir).to_string_lossy().to_string(),
        }],
        path_prepend: vec![bin.to_string_lossy().to_string()],
        path_append: vec![],
    };
    Ok(plan)
}

#[cfg(windows)]
fn detect_python(dir: &Path) -> Result<PresetPlan, EnvError> {
    let exe = dir.join("python.exe");
    if !exe.is_file() {
        return Err(EnvError::PresetDetectionFailed {
            preset: "python",
            dir: dir.to_string_lossy().to_string(),
            reason: format!("expected {} to exist", exe.display()),
        });
    }
    Ok(PresetPlan {
        preset: PresetKind::Python,
        scope: Scope::User,
        vars: vec![],
        path_prepend: vec![dir.to_string_lossy().to_string()],
        path_append: vec![],
    })
}

#[cfg(windows)]
fn detect_node(dir: &Path) -> Result<PresetPlan, EnvError> {
    let exe = dir.join("node.exe");
    if !exe.is_file() {
        return Err(EnvError::PresetDetectionFailed {
            preset: "node",
            dir: dir.to_string_lossy().to_string(),
            reason: format!("expected {} to exist", exe.display()),
        });
    }
    Ok(PresetPlan {
        preset: PresetKind::Node,
        scope: Scope::User,
        vars: vec![],
        path_prepend: vec![dir.to_string_lossy().to_string()],
        path_append: vec![],
    })
}

#[cfg(windows)]
fn detect_go(dir: &Path) -> Result<PresetPlan, EnvError> {
    let bin = resolve_with_bin_descent(dir, "go.exe")?;
    let goroot = bin.parent().unwrap_or(dir).to_string_lossy().to_string();
    Ok(PresetPlan {
        preset: PresetKind::Go,
        scope: Scope::User,
        vars: vec![EnvVarSpec {
            name: "GOROOT".into(),
            value: goroot.clone(),
        }],
        path_prepend: vec![bin.to_string_lossy().to_string()],
        path_append: vec![],
    })
}

#[cfg(windows)]
fn detect_rust(dir: &Path) -> Result<PresetPlan, EnvError> {
    let exe = dir.join("cargo.exe");
    if !exe.is_file() {
        return Err(EnvError::PresetDetectionFailed {
            preset: "rust",
            dir: dir.to_string_lossy().to_string(),
            reason: format!("expected {} to exist (please select the .cargo/bin directory)", exe.display()),
        });
    }
    Ok(PresetPlan {
        preset: PresetKind::Rust,
        scope: Scope::User,
        vars: vec![],
        path_prepend: vec![],
        path_append: vec![dir.to_string_lossy().to_string()],
    })
}

/// Try `<dir>/<exe>` first, then `<dir>/bin/<exe>`. Returns the full path
/// of the bin directory that contains the executable.
#[cfg(windows)]
fn resolve_with_bin_descent(dir: &Path, exe: &str) -> Result<PathBuf, EnvError> {
    let direct = dir.join(exe);
    if direct.is_file() {
        return Ok(dir.to_path_buf());
    }
    let in_bin = dir.join("bin").join(exe);
    if in_bin.is_file() {
        return Ok(dir.join("bin"));
    }
    Err(EnvError::PresetDetectionFailed {
        preset: "(java|go)",
        dir: dir.to_string_lossy().to_string(),
        reason: format!(
            "neither {} nor {} exists",
            direct.display(),
            in_bin.display()
        ),
    })
}

#[cfg(windows)]
fn apply_preset_inner(plan: PresetPlan) -> Result<ApplyResult, EnvError> {
    let mut applied: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    for spec in &plan.vars {
        validate_var_name(&spec.name)?;
        set_var(plan.scope, &spec.name, &spec.value)?;
        applied.push(format!("{}={}", spec.name, spec.value));
    }

    // PATH: read current for the chosen scope, prepend/append, write back.
    let current_snapshot = read_env_snapshot()?;
    let current: Vec<String> = match plan.scope {
        Scope::User => current_snapshot.path_user,
        Scope::System => current_snapshot.path_system,
    };
    let current_set: std::collections::HashSet<String> = current
        .iter()
        .map(|s| s.to_lowercase())
        .collect();

    let mut new_entries: Vec<String> = Vec::new();
    for entry in &plan.path_prepend {
        let key = entry.to_lowercase();
        if !current_set.contains(&key) {
            new_entries.push(entry.clone());
        } else {
            warnings.push(format!("PATH 已包含 {}, 跳过", entry));
        }
    }
    new_entries.extend(current.iter().cloned());
    for entry in &plan.path_append {
        let key = entry.to_lowercase();
        if !current_set.contains(&key) {
            new_entries.push(entry.clone());
        } else {
            warnings.push(format!("PATH 已包含 {}, 跳过", entry));
        }
    }

    set_path_entries(plan.scope, &new_entries)?;
    applied.push(format!(
        "Path updated ({} entries, scope={})",
        new_entries.len(),
        plan.scope.as_label()
    ));

    Ok(ApplyResult { applied, warnings })
}

// -------------------- Tauri command wrappers --------------------

#[tauri::command]
pub fn list_env() -> Result<EnvSnapshot, String> {
    #[cfg(windows)]
    {
        read_env_snapshot().map_err(Into::into)
    }
    #[cfg(not(windows))]
    {
        Err(EnvError::UnsupportedOnThisPlatform.into())
    }
}

#[tauri::command]
pub fn set_var_cmd(scope: Scope, name: String, value: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        set_var(scope, &name, &value).map_err(Into::into)
    }
    #[cfg(not(windows))]
    {
        let _ = (scope, name, value);
        Err(EnvError::UnsupportedOnThisPlatform.into())
    }
}

#[tauri::command]
pub fn delete_var_cmd(scope: Scope, name: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        delete_var(scope, &name).map_err(Into::into)
    }
    #[cfg(not(windows))]
    {
        let _ = (scope, name);
        Err(EnvError::UnsupportedOnThisPlatform.into())
    }
}

#[tauri::command]
pub fn set_path_entries_cmd(scope: Scope, entries: Vec<String>) -> Result<(), String> {
    #[cfg(windows)]
    {
        set_path_entries(scope, &entries).map_err(Into::into)
    }
    #[cfg(not(windows))]
    {
        let _ = (scope, entries);
        Err(EnvError::UnsupportedOnThisPlatform.into())
    }
}

#[tauri::command]
pub fn detect_preset_cmd(kind: PresetKind, dir: String) -> Result<PresetResult, String> {
    #[cfg(windows)]
    {
        let path = PathBuf::from(&dir);
        if !path.is_dir() {
            return Err(EnvError::PresetDetectionFailed {
                preset: preset_name(kind),
                dir: dir.clone(),
                reason: "directory does not exist".into(),
            }
            .into());
        }
        let plan = detect_preset_inner(kind, &path)?;
        let warnings = derive_idempotent_warnings(&plan);
        Ok(PresetResult {
            preset: kind,
            plan,
            warnings,
        })
    }
    #[cfg(not(windows))]
    {
        let _ = (kind, dir);
        Err(EnvError::UnsupportedOnThisPlatform.into())
    }
}

#[cfg(windows)]
fn derive_idempotent_warnings(plan: &PresetPlan) -> Vec<String> {
    // Best-effort idempotency hints. Read current snapshot; flag var/path
    // entries that already match in the target scope. This is informational;
    // the apply step also deduplicates.
    let mut warnings = Vec::new();
    let snap = match read_env_snapshot() {
        Ok(s) => s,
        Err(_) => return warnings,
    };
    let current_by_name_scope: std::collections::HashMap<(&str, Scope), &str> = snap
        .vars
        .iter()
        .map(|v| ((v.name.as_str(), v.scope), v.value.as_str()))
        .collect();
    for spec in &plan.vars {
        if let Some(existing) = current_by_name_scope.get(&(spec.name.as_str(), plan.scope)) {
            if existing.eq_ignore_ascii_case(&spec.value) {
                warnings.push(format!(
                    "{} ({}) 已指向相同值, 写入仍是幂等操作",
                    spec.name,
                    plan.scope.as_label()
                ));
            }
        }
    }
    let path_set: std::collections::HashSet<String> = match plan.scope {
        Scope::User => snap.path_user.iter().map(|s| s.to_lowercase()).collect(),
        Scope::System => snap.path_system.iter().map(|s| s.to_lowercase()).collect(),
    };
    for entry in plan.path_prepend.iter().chain(plan.path_append.iter()) {
        if path_set.contains(&entry.to_lowercase()) {
            warnings.push(format!(
                "PATH ({}) 已包含 {}, 跳过",
                plan.scope.as_label(),
                entry
            ));
        }
    }
    warnings
}

#[tauri::command]
pub fn apply_preset_cmd(plan: PresetPlan) -> Result<ApplyResult, String> {
    #[cfg(windows)]
    {
        apply_preset_inner(plan).map_err(Into::into)
    }
    #[cfg(not(windows))]
    {
        let _ = plan;
        Err(EnvError::UnsupportedOnThisPlatform.into())
    }
}

// -------------------- Tests --------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_var_name_rejects_empty() {
        assert!(matches!(
            validate_var_name(""),
            Err(EnvError::InvalidVarName(_))
        ));
    }

    #[test]
    fn validate_var_name_rejects_equals() {
        assert!(matches!(
            validate_var_name("FOO=BAR"),
            Err(EnvError::InvalidVarName(_))
        ));
    }

    #[test]
    fn validate_var_name_rejects_leading_digit() {
        assert!(matches!(
            validate_var_name("1FOO"),
            Err(EnvError::InvalidVarName(_))
        ));
    }

    #[test]
    fn validate_var_name_rejects_whitespace() {
        assert!(matches!(
            validate_var_name("FOO BAR"),
            Err(EnvError::InvalidVarName(_))
        ));
    }

    #[test]
    fn validate_var_name_accepts_underscore_prefix() {
        assert!(validate_var_name("_FOO").is_ok());
    }

    #[test]
    fn validate_var_name_accepts_uppercase() {
        assert!(validate_var_name("JAVA_HOME").is_ok());
    }

    #[test]
    fn validate_var_name_accepts_path_like() {
        assert!(validate_var_name("CARGO_HOME").is_ok());
    }

    // ---- preset detection (mocked filesystem) ----
    // resolve_with_bin_descent and detect_java/go use Path::is_file on
    // real disk. We exercise them with std::env::temp_dir.

    fn make_temp_subdir(label: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "toolbench-env-test-{}-{}-{}",
            label,
            std::process::id(),
            now_ms()
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn resolve_with_bin_descent_finds_direct_exe() {
        let dir = make_temp_subdir("direct");
        std::fs::write(dir.join("java.exe"), b"").unwrap();
        let bin = resolve_with_bin_descent(&dir, "java.exe").unwrap();
        assert_eq!(bin, dir);
    }

    #[test]
    fn resolve_with_bin_descent_finds_exe_in_bin() {
        let dir = make_temp_subdir("bin");
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        std::fs::write(dir.join("bin").join("go.exe"), b"").unwrap();
        let bin = resolve_with_bin_descent(&dir, "go.exe").unwrap();
        assert_eq!(bin, dir.join("bin"));
    }

    #[test]
    fn resolve_with_bin_descent_errors_when_missing() {
        let dir = make_temp_subdir("missing");
        assert!(matches!(
            resolve_with_bin_descent(&dir, "java.exe"),
            Err(EnvError::PresetDetectionFailed { .. })
        ));
    }

    #[test]
    fn detect_java_with_bin_parent() {
        let dir = make_temp_subdir("java-bin");
        std::fs::create_dir_all(dir.join("bin")).unwrap();
        std::fs::write(dir.join("bin").join("java.exe"), b"").unwrap();
        let plan = detect_java(&dir).unwrap();
        assert_eq!(plan.vars.len(), 1);
        assert_eq!(plan.vars[0].name, "JAVA_HOME");
        assert_eq!(plan.vars[0].value, dir.to_string_lossy().to_string());
        assert_eq!(plan.path_prepend.len(), 1);
        assert_eq!(
            plan.path_prepend[0],
            dir.join("bin").to_string_lossy().to_string()
        );
    }

    #[test]
    fn detect_java_with_direct_exe() {
        let dir = make_temp_subdir("java-direct");
        std::fs::write(dir.join("java.exe"), b"").unwrap();
        let plan = detect_java(&dir).unwrap();
        // java.exe directly in dir means the user picked the bin/ dir; JAVA_HOME
        // is the parent of the bin dir, not bin itself.
        assert_eq!(
            plan.vars[0].value,
            dir.parent().unwrap().to_string_lossy().to_string()
        );
        assert_eq!(plan.path_prepend[0], dir.to_string_lossy().to_string());
    }

    #[test]
    fn detect_python_uses_path_prepend_only() {
        let dir = make_temp_subdir("python");
        std::fs::write(dir.join("python.exe"), b"").unwrap();
        let plan = detect_python(&dir).unwrap();
        assert!(plan.vars.is_empty(), "PYTHONHOME must not be set");
        assert_eq!(plan.path_prepend, vec![dir.to_string_lossy().to_string()]);
        assert!(plan.path_append.is_empty());
    }

    #[test]
    fn detect_rust_appends_not_prepends() {
        let dir = make_temp_subdir("rust");
        std::fs::write(dir.join("cargo.exe"), b"").unwrap();
        let plan = detect_rust(&dir).unwrap();
        assert!(plan.vars.is_empty(), "CARGO_HOME must not be set");
        assert!(plan.path_prepend.is_empty(), "rust must APPEND, not prepend");
        assert_eq!(plan.path_append, vec![dir.to_string_lossy().to_string()]);
    }
}
