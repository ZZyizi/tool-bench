import { Settings, type LucideIcon } from 'lucide-react';
import { globalRegistry } from '../plugins/registry';
import { resolveIcon } from '../plugins/resolveIcon';
import './Sidebar.css';

interface SidebarProps {
  activeId: string | null;
  onSelect: (pluginId: string) => void;
  onOpenSettings: () => void;
}

export function Sidebar({ activeId, onSelect, onOpenSettings }: SidebarProps) {
  const grouped = globalRegistry.byCategory();

  return (
    <aside className="sidebar">
      <div className="sidebar__title">
        <span>DevToolkit</span>
        <button
          type="button"
          className="sidebar__settings-btn"
          onClick={onOpenSettings}
          aria-label="打开设置"
          title="设置"
        >
          <Settings size={14} aria-hidden />
        </button>
      </div>
      {Array.from(grouped.entries()).map(([category, plugins]) => (
        <div key={category} className="sidebar__group">
          <div className="sidebar__group-title">{category}</div>
          {plugins.map((plugin) => {
            const Icon = resolveIcon(plugin.manifest.icon) as LucideIcon;
            return (
              <div
                key={plugin.manifest.id}
                className={`sidebar__item${activeId === plugin.manifest.id ? ' sidebar__item--active' : ''}`}
                onClick={() => onSelect(plugin.manifest.id)}
              >
                <span className="sidebar__icon" aria-hidden>
                  <Icon size={16} strokeWidth={1.75} />
                </span>
                <span>{plugin.manifest.name}</span>
              </div>
            );
          })}
        </div>
      ))}
    </aside>
  );
}
