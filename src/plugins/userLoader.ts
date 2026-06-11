// User plugin loader.
//
// Scans `~/.tool-bench/user-plugins/<id>/` at startup (via the
// `scan_user_plugins` Tauri command), reads each entry's JS source,
// and dynamic-imports it through a blob URL. Each loaded module
// must `export default` a `Plugin` object — same shape as built-in
// plugins. Conflicting ids are skipped with a console warning.
//
// User plugins are pre-built (Vite/esbuild output, no bare imports).
// The plugin-author-guide.md has the workflow.

import { invoke } from '@tauri-apps/api/core';
import { globalRegistry } from './registry';
import { createPluginContext } from './context';
import type { Plugin } from './types';

interface UserPluginInfo {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  category: string;
  icon: string | null;
  capabilities: string[];
  windowWidth: number | null;
  windowHeight: number | null;
  source: string;
  manifestPath: string;
}

const ctx = createPluginContext();
let initPromise: Promise<void> | null = null;

export function loadUserPlugins(): Promise<void> {
  if (!initPromise) {
    initPromise = doScan();
  }
  return initPromise;
}

async function doScan() {
  let infos: UserPluginInfo[];
  try {
    infos = await invoke<UserPluginInfo[]>('scan_user_plugins');
  } catch (e) {
    console.error('[user-plugins] scan failed:', e);
    return;
  }
  if (infos.length === 0) return;

  for (const info of infos) {
    if (globalRegistry.get(info.id)) {
      console.warn(
        `[user-plugins] ${info.id}: id conflicts with existing plugin, skipping`,
      );
      continue;
    }
    try {
      const blob = new Blob([info.source], { type: 'application/javascript' });
      const url = URL.createObjectURL(blob);
      try {
        const mod = await import(/* @vite-ignore */ url);
        const plugin = mod.default as Plugin | undefined;
        if (!plugin?.manifest?.id) {
          console.warn(`[user-plugins] ${info.id}: no default Plugin export`);
          continue;
        }
        globalRegistry.register(plugin);
        plugin.activate(ctx);
        console.log(`[user-plugins] loaded: ${plugin.manifest.id}`);
      } finally {
        URL.revokeObjectURL(url);
      }
    } catch (e) {
      console.warn(`[user-plugins] ${info.id}: load failed:`, e);
    }
  }
}
