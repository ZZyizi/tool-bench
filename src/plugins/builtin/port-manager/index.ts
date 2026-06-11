import type { Plugin, PluginManifest } from '../../types';
import { Plug } from 'lucide-react';
import { PortView } from './PortView';

const manifest: PluginManifest = {
  id: 'port-manager',
  name: '端口管理',
  version: '0.1.0',
  description: '查看和释放系统占用的端口',
  author: 'DevToolkit Team',
  category: 'Network',
  icon: Plug,
  entry: './index.ts',
  capabilities: ['network:read', 'process:read', 'process:kill'],
};

export const portManagerPlugin: Plugin = {
  manifest,
  Component: PortView,
  activate(ctx) {
    ctx.log('Port manager activated');
  },
};

export default portManagerPlugin;
