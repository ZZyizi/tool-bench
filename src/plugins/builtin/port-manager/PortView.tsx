import { useState, useEffect, useCallback } from 'react';
import { api } from '../../api';
import { ConfirmDialog } from '../../../components/ConfirmDialog';
import type { PortInfo } from '../../../types';
import './PortView.css';

export function PortView() {
  const [ports, setPorts] = useState<PortInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<PortInfo | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [actionMessage, setActionMessage] = useState<{ kind: 'success' | 'error'; text: string } | null>(null);
  const [query, setQuery] = useState('');

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await api.listPorts(query);
      setPorts(list);
      // Backend now does the filtering; if the active selection falls out
      // of the result set, clear it so the footer doesn't claim a row the
      // user can't see.
      setSelected((prev) =>
        prev && !list.some((p) => p.port === prev.port && p.pid === prev.pid) ? null : prev,
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [query]);

  useEffect(() => {
    // Debounce: avoid hammering netstat/lsof on every keystroke.
    const timer = setTimeout(refresh, 300);
    return () => clearTimeout(timer);
  }, [query, refresh]);

  const handleKill = useCallback(async () => {
    if (!selected) return;
    setConfirming(false);
    setActionMessage(null);
    try {
      const result = await api.killPort(selected.port);
      if (result.success) {
        setActionMessage({ kind: 'success', text: result.message });
      } else {
        setActionMessage({ kind: 'error', text: result.message });
      }
      setSelected(null);
      await refresh();
    } catch (e) {
      setActionMessage({ kind: 'error', text: String(e) });
    }
  }, [selected, refresh]);

  return (
    <div className="port-view">
      <div className="port-view__header">
        <h2 className="port-view__title">端口占用列表</h2>
        <div className="port-view__header-actions">
          <input
            className="port-view__search"
            type="text"
            placeholder="搜索端口号或进程名"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <button className="port-view__refresh" onClick={refresh} disabled={loading}>
            {loading ? '刷新中…' : '刷新'}
          </button>
        </div>
      </div>

      {error && <div className="port-view__error">加载失败: {error}</div>}
      {actionMessage && (
        <div className={actionMessage.kind === 'error' ? 'port-view__error' : 'port-view__empty'}>
          {actionMessage.text}
        </div>
      )}

      <div className="port-view__table">
        {ports.length === 0 && !loading && !error ? (
          <div className="port-view__empty">
            {query ? `没有匹配 "${query}" 的端口` : '没有检测到端口占用'}
          </div>
        ) : (
          <table>
            <thead>
              <tr>
                <th>协议</th>
                <th>端口</th>
                <th>进程</th>
                <th>PID</th>
                <th>状态</th>
              </tr>
            </thead>
            <tbody>
              {ports.map((p) => {
                const isSelected = selected?.port === p.port && selected?.pid === p.pid;
                return (
                  <tr
                    key={`${p.protocol}-${p.port}-${p.pid}`}
                    className={`port-view__row${isSelected ? ' port-view__row--selected' : ''}`}
                    onClick={() => setSelected(p)}
                  >
                    <td>{p.protocol.toUpperCase()}</td>
                    <td>{p.port}</td>
                    <td>{p.process_name ?? '—'}</td>
                    <td>{p.pid}</td>
                    <td>{p.state || (p.protocol === 'Udp' ? '*' : '')}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </div>

      <div className="port-view__footer">
        <span className="port-view__selection">
          {selected
            ? `已选: ${selected.protocol.toUpperCase()} ${selected.port} (PID ${selected.pid})`
            : '点击行选择端口'}
        </span>
        <button
          className="port-view__kill"
          disabled={!selected}
          onClick={() => setConfirming(true)}
        >
          释放端口
        </button>
      </div>

      {confirming && selected && (
        <ConfirmDialog
          title="确认释放端口"
          message={`确定要结束占用端口 ${selected.port} 的进程 (PID ${selected.pid}) 吗？此操作不可撤销。`}
          confirmLabel="确认释放"
          onConfirm={handleKill}
          onCancel={() => setConfirming(false)}
        />
      )}
    </div>
  );
}
