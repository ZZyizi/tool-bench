import { useState } from 'react';
import { MessageSquare } from 'lucide-react';
import type { Plugin } from '../../src/plugins/types';
import { echoApi } from '../../src/plugins/api.gen';
import manifestRaw from './plugin.json';

const manifest = {
  ...manifestRaw,
  icon: MessageSquare,
} as const;

function EchoView() {
  const [input, setInput] = useState('hello');
  const [output, setOutput] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  return (
    <div style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 12 }}>
      <h2>Echo 测试</h2>
      <p style={{ color: '#888' }}>输入字符串，调用后端的 echo 命令原样返回。</p>
      <input
        value={input}
        onChange={(e) => setInput(e.target.value)}
        style={{ padding: 8, fontSize: 14 }}
      />
      <button
        onClick={async () => {
          setBusy(true);
          try {
            const r = await echoApi.echo({ message: input });
            setOutput(r.message);
          } finally {
            setBusy(false);
          }
        }}
        disabled={busy}
        style={{ padding: 8 }}
      >
        {busy ? '发送中…' : '回显'}
      </button>
      {output !== null && (
        <pre style={{ background: '#1e1e1e', color: '#9cdcfe', padding: 12 }}>
          {output}
        </pre>
      )}
    </div>
  );
}

export const echoPlugin: Plugin = {
  manifest,
  Component: EchoView,
  activate(ctx) {
    ctx.log('Echo activated');
  },
};

export default echoPlugin;
