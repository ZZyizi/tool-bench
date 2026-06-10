import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export type AppMode = 'embedded' | 'desktop';
export type CloseBehavior = 'quit' | 'hide';

export interface AppSettings {
  mode: AppMode;
  closeBehavior: CloseBehavior;
  /// Stable ids (`app:<hash>` for installed apps, `tool:<id>` for built-in
  /// tools) of items pinned into the quick-switcher. Order is preserved.
  pinnedApps: string[];
  /// Configurable global shortcut for opening the quick launcher, e.g. "Alt+Space".
  quickLaunchShortcut: string;
}

export const DEFAULT_SETTINGS: AppSettings = {
  mode: 'desktop',
  closeBehavior: 'hide',
  pinnedApps: [],
  quickLaunchShortcut: 'Alt+Space',
};

type Setter = (next: AppSettings) => void;

const CHANGE_EVENT = 'devtoolkit-settings-changed';

export function useSettings(): [AppSettings, Setter] {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const loaded = useRef(false);

  // Load settings from the Rust backend on first mount.
  useEffect(() => {
    if (loaded.current) return;
    loaded.current = true;
    invoke<AppSettings>('get_settings')
      .then((server) => {
        setSettings(server);
      })
      .catch((err) => {
        console.error('[settings] failed to load from backend, using defaults', err);
      });
  }, []);

  // Sync with other useSettings() instances via custom event.
  useEffect(() => {
    const onChange = (event: Event) => {
      const detail = (event as CustomEvent<AppSettings>).detail;
      setSettings(detail);
    };
    window.addEventListener(CHANGE_EVENT, onChange as EventListener);
    return () => window.removeEventListener(CHANGE_EVENT, onChange as EventListener);
  }, []);

  // Optimistic setter: update local state + broadcast to other instances,
  // then persist to backend.
  const setter: Setter = useCallback((next) => {
    setSettings(next);
    window.dispatchEvent(new CustomEvent<AppSettings>(CHANGE_EVENT, { detail: next }));
    invoke('set_settings', { settings: next }).catch((err) => {
      console.error('[settings] failed to persist to backend', err);
    });
  }, []);

  return [settings, setter];
}
