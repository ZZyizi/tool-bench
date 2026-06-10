use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::cmd::windows::CLOSE_HIDE;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub close_behavior: String,
    #[serde(default)]
    pub pinned_apps: Vec<String>,
    #[serde(default = "default_shortcut")]
    pub quick_launch_shortcut: String,
}

fn default_shortcut() -> String {
    "Ctrl+Space".to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            mode: "desktop".to_string(),
            close_behavior: "hide".to_string(),
            pinned_apps: Vec::new(),
            quick_launch_shortcut: "Ctrl+Space".to_string(),
        }
    }
}

pub struct SettingsStore {
    file_path: PathBuf,
    cache: Mutex<Option<AppSettings>>,
}

impl SettingsStore {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            cache: Mutex::new(None),
        }
    }

    pub fn load(&self) -> Result<AppSettings, String> {
        if let Some(cached) = self.cache.lock().unwrap().clone() {
            return Ok(cached);
        }
        let loaded = match std::fs::read_to_string(&self.file_path) {
            Ok(raw) => serde_json::from_str::<AppSettings>(&raw).unwrap_or_default(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => AppSettings::default(),
            Err(e) => return Err(e.to_string()),
        };
        *self.cache.lock().unwrap() = Some(loaded.clone());
        Ok(loaded)
    }

    pub fn save(&self, next: AppSettings) -> Result<AppSettings, String> {
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let serialized = serde_json::to_string_pretty(&next).map_err(|e| e.to_string())?;
        let tmp = self.file_path.with_extension("json.tmp");
        std::fs::write(&tmp, serialized).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.file_path).map_err(|e| e.to_string())?;
        *self.cache.lock().unwrap() = Some(next.clone());
        Ok(next)
    }
}

pub fn default_settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("settings.json"))
}

#[tauri::command]
pub fn get_settings(state: State<'_, SettingsStore>) -> Result<AppSettings, String> {
    state.load()
}

#[tauri::command]
pub fn set_settings(
    app: AppHandle,
    settings: AppSettings,
    state: State<'_, SettingsStore>,
    close_behavior: State<'_, Arc<AtomicU8>>,
) -> Result<AppSettings, String> {
    // 1. Save to file
    let result = state.save(settings)?;

    // 2. Apply close_behavior
    let behavior_value = match result.close_behavior.as_str() {
        "hide" => CLOSE_HIDE,
        _ => 0, // CLOSE_QUIT
    };
    close_behavior.store(behavior_value, Ordering::Relaxed);

    // 3. Re-register global shortcut
    if let Err(e) = apply_shortcut(&app, &result.quick_launch_shortcut) {
        eprintln!("[toolBench] failed to re-register shortcut: {e}");
    }

    Ok(result)
}

fn apply_shortcut(app: &tauri::AppHandle, shortcut_str: &str) -> Result<(), String> {
    let new_shortcut: Shortcut = shortcut_str
        .parse()
        .map_err(|_| format!("无效快捷键: {}", shortcut_str))?;

    // Unregister ALL existing global shortcuts (we only have one)
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| e.to_string())?;

    // Register the new one
    let app_handle = app.clone();
    app.global_shortcut()
        .on_shortcut(new_shortcut, move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                let _ = crate::cmd::quick_switcher::open_quick_switcher(app_handle.clone());
            }
        })
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Called from lib.rs::setup() to register the initial shortcut from saved settings.
pub fn apply_shortcut_from_settings(app: &tauri::AppHandle) -> Result<(), String> {
    let store = app.state::<SettingsStore>();
    let settings = store.load()?;
    apply_shortcut(app, &settings.quick_launch_shortcut)
}

/// Toggle "shortcut recording" mode.
///
/// While recording:
/// - unregister all global shortcuts so combos like Ctrl+Space reach the webview
///   instead of being consumed by the global handler
/// - on Windows, stop swallowing Alt+Space so the recorder can capture it too
///
/// When recording ends, re-register from saved settings and re-enable the
/// Alt+Space suppression.
#[tauri::command]
pub fn set_recording_mode(app: AppHandle, recording: bool) -> Result<(), String> {
    if recording {
        app.global_shortcut()
            .unregister_all()
            .map_err(|e| e.to_string())?;
        #[cfg(windows)]
        crate::windows_hook::set_suppress(false);
    } else {
        #[cfg(windows)]
        crate::windows_hook::set_suppress(true);
        // Re-apply whatever shortcut is currently saved. If the user changed
        // it during recording, set_settings already re-registered it; calling
        // here is a no-op-ish reapply.
        apply_shortcut_from_settings(&app)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn tmp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "devtoolkit-settings-test-{}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            SEQ.fetch_add(1, Ordering::Relaxed),
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    fn load(store: &SettingsStore) -> AppSettings {
        store.load().unwrap_or_default()
    }

    #[test]
    fn missing_file_yields_defaults() {
        let store = SettingsStore::new(tmp_path("settings.json"));
        let s = load(&store);
        assert_eq!(s.mode, "desktop");
        assert_eq!(s.close_behavior, "hide");
        assert!(s.pinned_apps.is_empty());
        assert_eq!(s.quick_launch_shortcut, "Ctrl+Space");
    }

    #[test]
    fn save_then_load_round_trips() {
        let path = tmp_path("settings.json");
        let store = SettingsStore::new(path.clone());
        let settings = AppSettings {
            mode: "embedded".into(),
            close_behavior: "quit".into(),
            pinned_apps: vec!["app:abc".into(), "tool:port-manager".into()],
            quick_launch_shortcut: "Ctrl+Shift+P".into(),
        };
        store.save(settings.clone()).unwrap();
        let loaded = SettingsStore::new(path).load().unwrap();
        assert_eq!(loaded.mode, settings.mode);
        assert_eq!(loaded.close_behavior, settings.close_behavior);
        assert_eq!(loaded.pinned_apps, settings.pinned_apps);
        assert_eq!(loaded.quick_launch_shortcut, settings.quick_launch_shortcut);
    }

    #[test]
    fn corrupt_file_falls_back_to_defaults() {
        let path = tmp_path("settings.json");
        std::fs::write(&path, "{not valid json").unwrap();
        let store = SettingsStore::new(path);
        let s = load(&store);
        assert_eq!(s.mode, "desktop");
    }
}
