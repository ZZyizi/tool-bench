import type { ComponentType } from 'react';
import type { invoke } from '@tauri-apps/api/core';

// icon 可以是 lucide 字符串（从 plugin.json 读出）或 ComponentType（在
// index.tsx 里手动 import 后赋值）。两种来源都允许。
export type IconRef = string | ComponentType;

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  category: string;
  icon?: IconRef;
  entry?: string;
  capabilities?: string[];
  windowWidth?: number;
  windowHeight?: number;
}

export interface PluginContext {
  invoke: typeof invoke;
  notify: (message: string, type?: 'info' | 'success' | 'error') => void;
  log: (...args: unknown[]) => void;
}

export interface Plugin {
  manifest: PluginManifest;
  Component?: ComponentType;
  activate: (context: PluginContext) => void | Promise<void>;
  deactivate?: () => void | Promise<void>;
}
