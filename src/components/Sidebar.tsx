import { globalRegistry } from '../plugins/registry';
import './Sidebar.css';

interface SidebarProps {
  activeId: string | null;
  onSelect: (pluginId: string) => void;
}

export function Sidebar({ activeId, onSelect }: SidebarProps) {
  const grouped = globalRegistry.byCategory();

  return (
    <aside className="sidebar">
      <h2 className="sidebar__title">DevToolkit</h2>
      {Array.from(grouped.entries()).map(([category, plugins]) => (
        <div key={category} className="sidebar__group">
          <div className="sidebar__group-title">{category}</div>
          {plugins.map((plugin) => (
            <div
              key={plugin.manifest.id}
              className={`sidebar__item${activeId === plugin.manifest.id ? ' sidebar__item--active' : ''}`}
              onClick={() => onSelect(plugin.manifest.id)}
            >
              {plugin.manifest.icon && <span className="sidebar__icon">{plugin.manifest.icon}</span>}
              <span>{plugin.manifest.name}</span>
            </div>
          ))}
        </div>
      ))}
    </aside>
  );
}
