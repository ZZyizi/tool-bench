import { globalRegistry } from '../registry';
import { createPluginContext } from '../context';
import { portManagerPlugin } from './port-manager';

export const builtinContext = createPluginContext();

globalRegistry.register(portManagerPlugin);

for (const plugin of globalRegistry.list()) {
  plugin.activate(builtinContext);
}
