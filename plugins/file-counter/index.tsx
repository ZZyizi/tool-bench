// Plugin: file-counter
// Demo: pick a directory via clipboard, call systemApi.fileList, display counts.
// 3 files total (plugin.json + index.tsx + this file). 0 Rust changes.

import { useState } from 'react';
import { Hash } from 'lucide-react';
import type { Plugin } from '../../src/plugins/types';
import { systemApi } from '../../src/plugins/api.gen';
import manifestRaw from './plugin.json';

const manifest = {
  ...manifestRaw,
  icon: Hash,
} as const;

function FileCounterView() {
  const [dir, setDir] = useState('');
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<{
    total: number;
    files: number;
    dirs: number;
  } | null>(null);
  const [error, setError] = useState<string | null>(null);

  const onCount = async () => {
    if (!dir) return;
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      const entries = await systemApi.fileList({ dir });
      const files = entries.filter((e) => !e.is_dir).length;
      const dirs = entries.filter((e) => e.is_dir).length;
      setResult({ total: entries.length, files, dirs });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 12 }}>
      <h2 style={{ margin: 0 }}>File Counter</h2>
      <p style={{ color: '#888', margin: 0 }}>
        输入绝对路径，统计该目录下文件和子目录数量。完全用 <code>systemApi</code> 实现。
      </p>
      <input
        value={dir}
        onChange={(e) => setDir(e.target.value)}
        placeholder="C:\Users\YourName\Documents"
        style={{ padding: 8, fontSize: 14 }}
      />
      <button onClick={onCount} disabled={busy || !dir} style={{ padding: 8 }}>
        {busy ? '统计中…' : '统计'}
      </button>
      {error && <pre style={{ color: '#f88', margin: 0 }}>{error}</pre>}
      {result && (
        <div style={{ background: '#1e1e1e', padding: 12, borderRadius: 4 }}>
          <div>总数: {result.total}</div>
          <div>文件: {result.files}</div>
          <div>目录: {result.dirs}</div>
        </div>
      )}
    </div>
  );
}

export const fileCounterPlugin: Plugin = {
  manifest,
  Component: FileCounterView,
  activate(ctx) {
    ctx.log('File counter activated');
  },
};

export default fileCounterPlugin;
