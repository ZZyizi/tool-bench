import { Plug } from 'lucide-react';
import type { Plugin } from '../../src/plugins/types';
import manifestRaw from './plugin.json';
import { PortView } from './PortView';

// plugin.json can't carry a React component, so resolve the icon here.
const manifest = {
  ...manifestRaw,
  icon: Plug,
} as const;

export const portManagerPlugin: Plugin = {
  manifest,
  Component: PortView,
  activate(ctx) {
    ctx.log('Port manager activated');
  },
};

export default portManagerPlugin;
