import { globalRegistry } from '../registry';
import { createPluginContext } from '../context';
import { portManagerPlugin } from './port-manager';
import { envEditorPlugin } from './env-editor';

export const builtinContext = createPluginContext();

globalRegistry.register(portManagerPlugin);
globalRegistry.register(envEditorPlugin);

for (const plugin of globalRegistry.list()) {
  plugin.activate(builtinContext);
}
