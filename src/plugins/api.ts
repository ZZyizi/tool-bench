import { invoke } from '@tauri-apps/api/core';
import type {
  ApplyResult,
  Capabilities,
  EnvSnapshot,
  FilteredPorts,
  InstalledApps,
  KillByNameResult,
  KillResult,
  PinnedApps,
  PresetKind,
  PresetPlan,
  PresetResult,
  Scope,
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

  listEnv: () => invoke<EnvSnapshot>('list_env'),
  setVar: (scope: Scope, name: string, value: string) =>
    invoke<void>('set_var', { scope, name, value }),
  deleteVar: (scope: Scope, name: string) =>
    invoke<void>('delete_var', { scope, name }),
  setPathEntries: (scope: Scope, entries: string[]) =>
    invoke<void>('set_path_entries', { scope, entries }),
  detectPreset: (kind: PresetKind, dir: string) =>
    invoke<PresetResult>('detect_preset', { kind, dir }),
  applyPreset: (plan: PresetPlan) =>
    invoke<ApplyResult>('apply_preset', { plan }),
};
