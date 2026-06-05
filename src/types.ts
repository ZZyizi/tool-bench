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

export interface Capabilities {
  network_read: boolean;
  process_read: boolean;
  process_kill: boolean;
  dns: boolean;
  file_read: boolean;
}
