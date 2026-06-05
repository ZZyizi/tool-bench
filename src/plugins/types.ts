import type { ComponentType } from 'react';
import type { invoke } from '@tauri-apps/api/core';

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  category: string;
  icon?: string;
  entry: string;
  capabilities?: string[];
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
