import { useEffect, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { globalRegistry } from './plugins/registry';
import './plugins/builtin';

interface ToolWindowRootProps {
  pluginId?: string;
}

function readPluginIdFromUrl(): string | null {
  if (typeof window === 'undefined') return null;
  const search = window.location.search;
  if (!search) return null;
  const params = new URLSearchParams(search);
  return params.get('plugin');
}

export function ToolWindowRoot({ pluginId: propId }: ToolWindowRootProps = {}) {
  const pluginId = useMemo(() => {
    if (propId) return propId;
    const fromLabel = getCurrentWebviewWindow().label;
    if (fromLabel.startsWith('tool-')) return fromLabel.slice('tool-'.length);
    return readPluginIdFromUrl();
  }, [propId]);

  // Esc closes the tool window (mirrors the QuickSwitcher behavior). The
  // listener uses the capture phase and a plain `event.key === 'Escape'`
  // check so it works regardless of where focus is inside the tool view —
  // an input field's Esc won't normally bubble, but capture-phase keydown
  // fires on the way down to the target.
  useEffect(() => {
    if (!pluginId) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        invoke('close_tool_window', { pluginId }).catch((err) => {
          console.error('[tool-window] close on Esc failed', err);
        });
      }
    };
    window.addEventListener('keydown', onKeyDown, true);
    return () => window.removeEventListener('keydown', onKeyDown, true);
  }, [pluginId]);

  const plugin = pluginId ? globalRegistry.get(pluginId) : undefined;
  const Component = plugin?.Component;

  if (!plugin) {
    return (
      <div className="tool-window tool-window--missing">
        <h2>工具未找到</h2>
        <p>没有找到 id 为 <code>{pluginId ?? '(空)'}</code> 的工具。</p>
      </div>
    );
  }

  if (!Component) {
    return (
      <div className="tool-window tool-window--missing">
        <h2>{plugin.manifest.name}</h2>
        <p>该工具未注册可视化组件。</p>
      </div>
    );
  }

  return (
    <div className="tool-window">
      <Component />
    </div>
  );
}
