import { useMemo } from 'react';
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
