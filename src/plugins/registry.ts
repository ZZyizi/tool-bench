import type { Plugin } from './types';

class PluginRegistry {
  private plugins = new Map<string, Plugin>();

  register(plugin: Plugin): void {
    if (this.plugins.has(plugin.manifest.id)) {
      throw new Error(`Plugin "${plugin.manifest.id}" already registered`);
    }
    this.plugins.set(plugin.manifest.id, plugin);
  }

  unregister(id: string): void {
    this.plugins.delete(id);
  }

  list(): Plugin[] {
    return Array.from(this.plugins.values());
  }

  get(id: string): Plugin | undefined {
    return this.plugins.get(id);
  }

  byCategory(): Map<string, Plugin[]> {
    const grouped = new Map<string, Plugin[]>();
    for (const plugin of this.list()) {
      const list = grouped.get(plugin.manifest.category) ?? [];
      list.push(plugin);
      grouped.set(plugin.manifest.category, list);
    }
    return grouped;
  }
}

export const globalRegistry = new PluginRegistry();
