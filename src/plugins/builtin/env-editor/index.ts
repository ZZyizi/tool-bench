import type { Plugin, PluginManifest } from '../../types';
import { Settings2 } from 'lucide-react';
import { EnvEditorView } from './EnvEditorView';

const manifest: PluginManifest = {
  id: 'env-editor',
  name: '环境变量',
  version: '0.1.0',
  description: '查看、修改用户环境变量；一键配置 Java / Python / Node / Go / Rust',
  author: 'DevToolkit Team',
  category: 'System',
  icon: Settings2,
  entry: './index.ts',
  capabilities: ['env:read', 'env:write'],
  windowWidth: 800,
  windowHeight: 560,
};

export const envEditorPlugin: Plugin = {
  manifest,
  Component: EnvEditorView,
  activate(ctx) {
    ctx.log('Env editor activated');
  },
};

export default envEditorPlugin;
