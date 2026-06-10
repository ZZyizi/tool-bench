import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Box, Search, Settings, Zap, type LucideIcon } from 'lucide-react';
import { globalRegistry } from '../plugins/registry';
import { useSettings } from '../settings';
import type { Plugin } from '../plugins/types';
import './Launcher.css';

interface LauncherProps {
  onOpenSettings: () => void;
}

export function Launcher({ onOpenSettings }: LauncherProps) {
  const [settings] = useSettings();
  const [opening, setOpening] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const grouped = globalRegistry.byCategory();

  const openTool = async (plugin: Plugin, useAndGo: boolean) => {
    const id = plugin.manifest.id;
    setError(null);
    setOpening(id);
    try {
      await invoke('open_tool_window', {
        pluginId: id,
        title: plugin.manifest.name,
        width: plugin.manifest.windowWidth ?? null,
        height: plugin.manifest.windowHeight ?? null,
        useAndGo,
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setOpening((prev) => (prev === id ? null : prev));
    }
  };

  const openQuickSwitcher = async () => {
    setError(null);
    try {
      await invoke('open_quick_switcher');
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="launcher">
      <header className="launcher__header">
        <h1 className="launcher__title">DevToolkit</h1>
        <div className="launcher__header-actions">
          <button
            type="button"
            className="launcher__qs-btn"
            onClick={openQuickSwitcher}
            title={`快速启动 (${settings.quickLaunchShortcut})`}
            aria-label="打开快速启动"
          >
            <Search size={16} aria-hidden />
            <span>快速启动</span>
            <kbd className="launcher__kbd">{settings.quickLaunchShortcut}</kbd>
          </button>
          <button
            type="button"
            className="launcher__settings-btn"
            onClick={onOpenSettings}
            aria-label="打开设置"
            title="设置"
          >
            <Settings size={18} aria-hidden />
          </button>
        </div>
      </header>

      {error && <div className="launcher__error">打开工具失败: {error}</div>}

      <div className="launcher__grid">
        {Array.from(grouped.entries()).map(([category, plugins]) => (
          <section key={category} className="launcher__section">
            <h2 className="launcher__section-title">{category}</h2>
            <div className="launcher__tiles">
              {plugins.map((plugin) => {
                const isOpening = opening === plugin.manifest.id;
                const Icon = (plugin.manifest.icon ?? Box) as LucideIcon;
                return (
                  <div key={plugin.manifest.id} className="launcher__tile">
                    <button
                      type="button"
                      className="launcher__tile-main"
                      onClick={() => openTool(plugin, false)}
                      disabled={isOpening}
                      title={plugin.manifest.description}
                    >
                      <span className="launcher__icon" aria-hidden>
                        <Icon size={32} strokeWidth={1.75} />
                      </span>
                      <span className="launcher__name">{plugin.manifest.name}</span>
                      <span className="launcher__desc">{plugin.manifest.description}</span>
                    </button>
                    <button
                      type="button"
                      className="launcher__tile-ephemeral"
                      onClick={() => openTool(plugin, true)}
                      disabled={isOpening}
                      title="即用即走：打开后失焦自动关闭"
                      aria-label="即用即走打开"
                    >
                      <Zap size={14} strokeWidth={2} aria-hidden />
                    </button>
                  </div>
                );
              })}
            </div>
          </section>
        ))}
      </div>
    </div>
  );
}
