import { invoke } from '@tauri-apps/api/core';
import type { Capabilities, KillResult, PortInfo } from '../types';

export const api = {
  listPorts: () => invoke<PortInfo[]>('list_ports'),
  killPort: (port: number) => invoke<KillResult>('kill_port', { port }),
  listCapabilities: () => invoke<Capabilities>('list_capabilities'),
};
