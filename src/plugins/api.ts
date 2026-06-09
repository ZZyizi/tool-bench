import { invoke } from '@tauri-apps/api/core';
import type { Capabilities, FilteredPorts, KillByNameResult, KillResult } from '../types';

export const api = {
  listPorts: (query = '') => invoke<FilteredPorts>('list_ports', { query }),
  killPort: (port: number) => invoke<KillResult>('kill_port', { port }),
  killByProcessName: (name: string) => invoke<KillByNameResult>('kill_by_process_name', { name }),
  listCapabilities: () => invoke<Capabilities>('list_capabilities'),
};
