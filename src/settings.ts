import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export type AppMode = 'embedded' | 'desktop';
export type CloseBehavior = 'quit' | 'hide';

export interface AppSettings {
  mode: AppMode;
  closeBehavior: CloseBehavior;
  /// Stable ids (`app:<hash>` for installed apps, `tool:<id>` for built-in
  /// tools) of items pinned into the quick-switcher. Order is preserved.
  pinnedApps: string[];
}

export const DEFAULT_SETTINGS: AppSettings = {
  mode: 'desktop',
  closeBehavior: 'hide',
  pinnedApps: [],
};

const STORAGE_KEY = 'devtoolkit.settings.v1';
const CHANGE_EVENT = 'devtoolkit-settings-changed';

function isAppSettings(value: unknown): value is AppSettings {
  if (!value || typeof value !== 'object') return false;
  const v = value as Record<string, unknown>;
  if (!(v.mode === 'embedded' || v.mode === 'desktop')) return false;
  if (!(v.closeBehavior === 'quit' || v.closeBehavior === 'hide')) return false;
  if (!Array.isArray(v.pinnedApps)) return false;
  if (!v.pinnedApps.every((id) => typeof id === 'string')) return false;
  return true;
}

export function loadSettings(): AppSettings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_SETTINGS };
    const parsed = JSON.parse(raw);
    if (isAppSettings(parsed)) return parsed;
  } catch {
    // fall through
  }
  return { ...DEFAULT_SETTINGS };
}

export function saveSettings(next: AppSettings): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
  window.dispatchEvent(new CustomEvent<AppSettings>(CHANGE_EVENT, { detail: next }));
}

type Setter = (next: AppSettings) => void;

export function useSettings(): [AppSettings, Setter] {
  const [settings, setSettings] = useState<AppSettings>(loadSettings);

  useEffect(() => {
    const onChange = (event: Event) => {
      const detail = (event as CustomEvent<AppSettings>).detail;
      if (isAppSettings(detail)) {
        setSettings(detail);
      }
    };
    const onStorage = (event: StorageEvent) => {
      if (event.key !== STORAGE_KEY || !event.newValue) return;
      try {
        const parsed = JSON.parse(event.newValue);
        if (isAppSettings(parsed)) setSettings(parsed);
      } catch {
        // ignore
      }
    };
    window.addEventListener(CHANGE_EVENT, onChange);
    window.addEventListener('storage', onStorage);
    return () => {
      window.removeEventListener(CHANGE_EVENT, onChange);
      window.removeEventListener('storage', onStorage);
    };
  }, []);

  useEffect(() => {
    // Sync the persisted close behavior to the Rust side on first mount so the
    // atomic matches localStorage (the Rust default is `hide` and would otherwise
    // override a previously-saved `quit` setting on app restart).
    invoke('set_close_behavior', { behavior: settings.closeBehavior }).catch((err) => {
      console.error('[settings] failed to sync initial close behavior to backend', err);
    });
  }, []);

  // Pull the server-side pinned list once on mount, then keep it in sync. The
  // backend is the source of truth so the same pinned set is visible in the
  // quick-switcher and in the settings panel.
  useEffect(() => {
    let cancelled = false;
    invoke<{ ids: string[] }>('get_pinned_apps')
      .then((server) => {
        if (cancelled) return;
        if (
          server.ids.length !== settings.pinnedApps.length ||
          server.ids.some((id, i) => id !== settings.pinnedApps[i])
        ) {
          setSettings({ ...settings, pinnedApps: server.ids });
        }
      })
      .catch(() => {
        // Backend may not be available in non-Tauri contexts; ignore.
      });
    return () => {
      cancelled = true;
    };
    // We intentionally only sync on mount — subsequent edits go through the
    // setter, which writes to the backend.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const setter: Setter = (next) => {
    saveSettings(next);
    setSettings(next);
    invoke('set_close_behavior', { behavior: next.closeBehavior }).catch((err) => {
      console.error('[settings] failed to sync close behavior to backend', err);
    });
    invoke('set_pinned_apps', { apps: { ids: next.pinnedApps } }).catch((err) => {
      console.error('[settings] failed to sync pinned apps to backend', err);
    });
  };

  return [settings, setter];
}
