import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export type AppMode = 'embedded' | 'desktop';
export type CloseBehavior = 'quit' | 'hide';

export interface AppSettings {
  mode: AppMode;
  closeBehavior: CloseBehavior;
}

export const DEFAULT_SETTINGS: AppSettings = {
  mode: 'desktop',
  closeBehavior: 'hide',
};

const STORAGE_KEY = 'devtoolkit.settings.v1';
const CHANGE_EVENT = 'devtoolkit-settings-changed';

function isAppSettings(value: unknown): value is AppSettings {
  if (!value || typeof value !== 'object') return false;
  const v = value as Record<string, unknown>;
  return (
    (v.mode === 'embedded' || v.mode === 'desktop') &&
    (v.closeBehavior === 'quit' || v.closeBehavior === 'hide')
  );
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

  const setter: Setter = (next) => {
    saveSettings(next);
    setSettings(next);
    invoke('set_close_behavior', { behavior: next.closeBehavior }).catch((err) => {
      console.error('[settings] failed to sync close behavior to backend', err);
    });
  };

  return [settings, setter];
}
