export type Protocol = 'Tcp' | 'Udp';

export interface PortInfo {
  protocol: Protocol;
  port: number;
  pid: number;
  state: string;
  process_name: string | null;
}

export interface KillResult {
  success: boolean;
  pid: number;
  port: number;
  message: string;
}

export interface KillByNameResult {
  success: boolean;
  name: string;
  killed: number;
  failed: number;
  message: string;
}

export interface Capabilities {
  network_read: boolean;
  process_read: boolean;
  process_kill: boolean;
  dns: boolean;
  file_read: boolean;
}

export interface FilteredPorts {
  ports: PortInfo[];
  hidden_system: number;
}

export interface InstalledApp {
  id: string;
  name: string;
  target: string;
  source: string;
  icon_index: number | null;
}

export interface InstalledApps {
  apps: InstalledApp[];
  scanned_at_ms: number;
}

export interface PinnedApps {
  ids: string[];
}
