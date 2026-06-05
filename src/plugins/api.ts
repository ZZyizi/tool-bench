import { invoke } from '@tauri-apps/api/core';
import type { Capabilities, FilteredPorts, KillResult } from '../types';

export const api = {
  listPorts: (query = '') => invoke<FilteredPorts>('list_ports', { query }),
  killPort: (port: number) => invoke<KillResult>('kill_port', { port }),
  listCapabilities: () => invoke<Capabilities>('list_capabilities'),
};
