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

export type VarSource = 'user' | 'process' | 'system';
export type Scope = 'user' | 'system';

export interface EnvVar {
  name: string;
  value: string;
  source: VarSource;
  scope: Scope;
}

export interface EnvSnapshot {
  vars: EnvVar[];
  path_user: string[];
  path_system: string[];
  warnings: string[];
  captured_at_ms: number;
}

export type PresetKind = 'java' | 'python' | 'node' | 'go' | 'rust';

export interface EnvVarSpec {
  name: string;
  value: string;
}

export interface PresetPlan {
  preset: PresetKind;
  scope: Scope;
  vars: EnvVarSpec[];
  path_prepend: string[];
  path_append: string[];
}

export interface PresetResult {
  preset: PresetKind;
  plan: PresetPlan;
  warnings: string[];
}

export interface ApplyResult {
  applied: string[];
  warnings: string[];
}

// ---- dispatch arg shapes ----
//
// These are the JSON payloads sent to the `dispatch` Tauri command. Each
// plugin command in plugin.json references one of these via argsRef; the
// Rust wrapper deserializes the same shape on the other side.

export interface ListPortsArgs {
  query?: string;
}

export interface KillPortArgs {
  port: number;
}

export interface KillByNameArgs {
  name: string;
}

export interface SetUserVarArgs {
  scope: Scope;
  name: string;
  value: string;
}

export interface DeleteUserVarArgs {
  scope: Scope;
  name: string;
}

export interface SetPathEntriesArgs {
  scope: Scope;
  entries: string[];
}

export interface DetectPresetArgs {
  kind: PresetKind;
  dir: string;
}

// ---- echo plugin (Phase 1 verification) ----

export interface EchoArgs {
  message: string;
}

export interface EchoResult {
  message: string;
}

// ---- system toolbox ----
//
// 8 generic primitives (file / clipboard) backed by src-tauri/src/cmd/system.rs.
// Composed by 3rd-party plugins to build sticky-notes / todo / snippet-manager
// without writing any Rust. See docs/plugin-author-guide.md §3.

export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}

// Result type aliases — codegen's `returnsRef` is matched against
// `export (interface|type) Name` in this file, so builtins like
// `string` / `boolean` need to be re-exported with a name.
export type FileReadResult = string;
export type FileListResult = FileEntry[];
export type FileExistsResult = boolean;

export interface FileReadArgs {
  path: string;
}
export interface FileWriteArgs {
  path: string;
  content: string;
}
export interface FileListArgs {
  dir: string;
}
export interface FileDeleteArgs {
  path: string;
}
export interface DirEnsureArgs {
  dir: string;
}
export interface FileExistsArgs {
  path: string;
}
export interface ClipboardWriteArgs {
  text: string;
}
