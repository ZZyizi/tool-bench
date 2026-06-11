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
