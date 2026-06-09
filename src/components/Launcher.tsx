import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { globalRegistry } from '../plugins/registry';
import type { Plugin } from '../plugins/types';
import './Launcher.css';

interface LauncherProps {
  onOpenSettings: () => void;
}

export function Launcher({ onOpenSettings }: LauncherProps) {
  const [opening, setOpening] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const grouped = globalRegistry.byCategory();

  const openTool = async (plugin: Plugin) => {
    const id = plugin.manifest.id;
    setError(null);
    setOpening(id);
    try {
      await invoke('open_tool_window', {
        pluginId: id,
        title: plugin.manifest.name,
        width: plugin.manifest.windowWidth ?? null,
        height: plugin.manifest.windowHeight ?? null,
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setOpening((prev) => (prev === id ? null : prev));
    }
  };

  return (
    <div className="launcher">
      <header className="launcher__header">
        <h1 className="launcher__title">DevToolkit</h1>
        <button
          type="button"
          className="launcher__settings-btn"
          onClick={onOpenSettings}
          aria-label="打开设置"
          title="设置"
        >
          ⚙
        </button>
      </header>

      {error && <div className="launcher__error">打开工具失败: {error}</div>}

      <div className="launcher__grid">
        {Array.from(grouped.entries()).map(([category, plugins]) => (
          <section key={category} className="launcher__section">
            <h2 className="launcher__section-title">{category}</h2>
            <div className="launcher__tiles">
              {plugins.map((plugin) => {
                const isOpening = opening === plugin.manifest.id;
                return (
                  <button
                    key={plugin.manifest.id}
                    type="button"
                    className="launcher__tile"
                    onClick={() => openTool(plugin)}
                    disabled={isOpening}
                    title={plugin.manifest.description}
                  >
                    <span className="launcher__icon" aria-hidden>
                      {plugin.manifest.icon || '•'}
                    </span>
                    <span className="launcher__name">{plugin.manifest.name}</span>
                    <span className="launcher__desc">{plugin.manifest.description}</span>
                  </button>
                );
              })}
            </div>
          </section>
        ))}
      </div>
    </div>
  );
}
