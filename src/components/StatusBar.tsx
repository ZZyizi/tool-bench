import { useEffect, useState } from 'react';
import { globalRegistry } from '../plugins/registry';

export function StatusBar() {
  const [version, setVersion] = useState<string>('');

  useEffect(() => {
    setVersion('0.1.0');
  }, []);

  const count = globalRegistry.list().length;

  return (
    <footer
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 16,
        padding: '6px 16px',
        borderTop: '1px solid var(--border)',
        background: 'var(--bg-elevated)',
        fontSize: 12,
        color: 'var(--fg-muted)',
      }}
    >
      <span>状态: 就绪</span>
      <span>|</span>
      <span>工具数: {count}</span>
      <span>|</span>
      <span>版本: {version}</span>
    </footer>
  );
}
