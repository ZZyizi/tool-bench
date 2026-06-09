use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PinnedApps {
    /// Stable ids of installed apps and tool plugins the user pinned into the
    /// quick-switcher. Ordering is preserved.
    #[serde(default)]
    pub ids: Vec<String>,
}

pub struct PinnedStore {
    file_path: PathBuf,
    cache: Mutex<Option<PinnedApps>>,
}

impl PinnedStore {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            cache: Mutex::new(None),
        }
    }

    pub fn load(&self) -> Result<PinnedApps, String> {
        if let Some(cached) = self.cache.lock().unwrap().clone() {
            return Ok(cached);
        }
        let loaded = match std::fs::read_to_string(&self.file_path) {
            Ok(raw) => serde_json::from_str::<PinnedApps>(&raw).unwrap_or_default(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => PinnedApps::default(),
            Err(e) => return Err(e.to_string()),
        };
        *self.cache.lock().unwrap() = Some(loaded.clone());
        Ok(loaded)
    }

    pub fn save(&self, next: PinnedApps) -> Result<PinnedApps, String> {
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let serialized = serde_json::to_string_pretty(&next).map_err(|e| e.to_string())?;
        // Write to a temp file then rename so a crash mid-write does not leave
        // a truncated pinned.json behind.
        let tmp = self.file_path.with_extension("json.tmp");
        std::fs::write(&tmp, serialized).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.file_path).map_err(|e| e.to_string())?;
        *self.cache.lock().unwrap() = Some(next.clone());
        Ok(next)
    }
}

pub fn default_pinned_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?;
    Ok(dir.join("pinned.json"))
}

#[tauri::command]
pub fn get_pinned_apps(state: State<'_, PinnedStore>) -> Result<PinnedApps, String> {
    state.load()
}

#[tauri::command]
pub fn set_pinned_apps(
    apps: PinnedApps,
    state: State<'_, PinnedStore>,
) -> Result<PinnedApps, String> {
    state.save(apps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn tmp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "devtoolkit-pinned-test-{}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            SEQ.fetch_add(1, Ordering::Relaxed),
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    fn load(store: &PinnedStore) -> PinnedApps {
        store.load().unwrap_or_default()
    }

    #[test]
    fn missing_file_yields_empty_list() {
        let store = PinnedStore::new(tmp_path("pinned.json"));
        assert!(load(&store).ids.is_empty());
    }

    #[test]
    fn save_then_load_round_trips() {
        let path = tmp_path("pinned.json");
        let store = PinnedStore::new(path.clone());
        let mut payload = PinnedApps::default();
        payload.ids = vec!["app:abc".into(), "tool:port-manager".into()];
        store.save(payload.clone()).unwrap();
        let loaded = PinnedStore::new(path).load().unwrap();
        assert_eq!(loaded.ids, payload.ids);
    }

    #[test]
    fn corrupt_file_falls_back_to_empty() {
        let path = tmp_path("pinned.json");
        std::fs::write(&path, "{not valid json").unwrap();
        let store = PinnedStore::new(path);
        assert!(load(&store).ids.is_empty());
    }
}
