import { Settings2 } from 'lucide-react';
import type { Plugin } from '../../src/plugins/types';
import manifestRaw from './plugin.json';
import { EnvEditorView } from './EnvEditorView';

const manifest = {
  ...manifestRaw,
  icon: Settings2,
} as const;

export const envEditorPlugin: Plugin = {
  manifest,
  Component: EnvEditorView,
  activate(ctx) {
    ctx.log('Env editor activated');
  },
};

export default envEditorPlugin;
