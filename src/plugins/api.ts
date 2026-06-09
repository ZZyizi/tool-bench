import { invoke } from '@tauri-apps/api/core';
import type {
  Capabilities,
  FilteredPorts,
  InstalledApps,
  KillByNameResult,
  KillResult,
  PinnedApps,
} from '../types';

export const api = {
  listPorts: (query = '') => invoke<FilteredPorts>('list_ports', { query }),
  killPort: (port: number) => invoke<KillResult>('kill_port', { port }),
  killByProcessName: (name: string) =>
    invoke<KillByNameResult>('kill_by_process_name', { name }),
  listCapabilities: () => invoke<Capabilities>('list_capabilities'),

  listInstalledApps: () => invoke<InstalledApps>('list_installed_apps'),
  launchApp: (target: string) => invoke<void>('launch_app', { target }),

  getPinnedApps: () => invoke<PinnedApps>('get_pinned_apps'),
  setPinnedApps: (apps: PinnedApps) =>
    invoke<PinnedApps>('set_pinned_apps', { apps }),

  openQuickSwitcher: () => invoke<void>('open_quick_switcher'),
};
