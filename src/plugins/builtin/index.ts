import { globalRegistry } from '../registry';
import { createPluginContext } from '../context';
import type { Plugin } from '../types';

const builtinContext = createPluginContext();

// Vite 编译期 glob：自动扫项目根目录的 plugins/*/index.tsx。
// 新增插件只需在 plugins/ 下建文件夹 + 写 plugin.json + index.tsx，
// 不再需要改本文件。
const modules = import.meta.glob<{ default: Plugin }>(
  '../../../plugins/*/index.tsx',
  { eager: true },
);

for (const [path, mod] of Object.entries(modules)) {
  const plugin = mod.default;
  if (!plugin) {
    console.error(`[plugins] ${path}: missing default export`);
    continue;
  }
  if (!plugin.manifest?.id) {
    console.error(`[plugins] ${path}: manifest.id is required`);
    continue;
  }
  globalRegistry.register(plugin);
  plugin.activate(builtinContext);
}
