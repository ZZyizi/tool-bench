import { useState, useEffect, useCallback, useMemo } from 'react';
import { Eraser } from 'lucide-react';
import { api } from '../../api';
import { ConfirmDialog } from '../../../components/ConfirmDialog';
import type { PortInfo } from '../../../types';
import { ProcessPickerDialog } from './ProcessPickerDialog';
import './PortView.css';

export function PortView() {
  const [ports, setPorts] = useState<PortInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<PortInfo | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [actionMessage, setActionMessage] = useState<{ kind: 'success' | 'error'; text: string } | null>(null);
  const [query, setQuery] = useState('');
  const [hiddenSystemCount, setHiddenSystemCount] = useState(0);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [pendingName, setPendingName] = useState<string | null>(null);
  const [bulkKilling, setBulkKilling] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await api.listPorts(query);
      setPorts(data.ports);
      setHiddenSystemCount(data.hidden_system);
      setSelected((prev) =>
        prev && !data.ports.some((p) => p.port === prev.port && p.pid === prev.pid) ? null : prev,
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [query]);

  useEffect(() => {
    const timer = setTimeout(refresh, 300);
    return () => clearTimeout(timer);
  }, [query, refresh]);

  const processNames = useMemo(() => {
    const set = new Set<string>();
    for (const p of ports) {
      if (p.process_name) set.add(p.process_name);
    }
    return Array.from(set).sort((a, b) => a.localeCompare(b));
  }, [ports]);

  const handleKill = useCallback(async () => {
    if (!selected) return;
    setConfirming(false);
    setActionMessage(null);
    try {
      const result = await api.killPort(selected.port);
      setActionMessage({
        kind: result.success ? 'success' : 'error',
        text: result.message,
      });
      setSelected(null);
      await refresh();
    } catch (e) {
      setActionMessage({ kind: 'error', text: String(e) });
    }
  }, [selected, refresh]);

  const handleBulkKill = useCallback(async () => {
    if (!pendingName) return;
    setBulkKilling(true);
    setActionMessage(null);
    try {
      const result = await api.killByProcessName(pendingName);
      setActionMessage({
        kind: result.success ? 'success' : 'error',
        text: result.message,
      });
    } catch (e) {
      setActionMessage({ kind: 'error', text: String(e) });
    } finally {
      setBulkKilling(false);
      setPendingName(null);
      await refresh();
    }
  }, [pendingName, refresh]);

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
          <button
            className="port-view__bulk-clean"
            onClick={() => setPickerOpen(true)}
            disabled={processNames.length === 0}
            title="按进程名一键清理"
          >
            <Eraser size={14} aria-hidden style={{ verticalAlign: '-2px', marginRight: 4 }} />
            一键清理
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
        {hiddenSystemCount > 0 && (
          <span className="port-view__hidden">已隐藏 {hiddenSystemCount} 个系统进程</span>
        )}
        <button
          className="port-view__kill"
          disabled={!selected}
          onClick={() => setConfirming(true)}
        >
          释放端口
        </button>
      </div>

      {pickerOpen && (
        <ProcessPickerDialog
          processNames={processNames}
          onClose={() => setPickerOpen(false)}
          onConfirm={(name) => {
            setPickerOpen(false);
            setPendingName(name);
          }}
        />
      )}

      {confirming && selected && (
        <ConfirmDialog
          title="确认释放端口"
          message={`确定要结束占用端口 ${selected.port} 的进程 (PID ${selected.pid}) 吗？此操作不可撤销。`}
          confirmLabel="确认释放"
          onConfirm={handleKill}
          onCancel={() => setConfirming(false)}
        />
      )}

      {pendingName && (
        <ConfirmDialog
          title="一键释放确认"
          message={`确定要结束所有名为 "${pendingName}" 的进程吗？此操作不可撤销。`}
          confirmLabel={bulkKilling ? '释放中…' : '一键释放'}
          onConfirm={handleBulkKill}
          onCancel={() => setPendingName(null)}
        />
      )}
    </div>
  );
}
