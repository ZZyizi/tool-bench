import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { ComponentType } from 'react';
import {
  AlertCircle,
  Box,
  Check,
  ChevronDown,
  ChevronUp,
  Code,
  Cog,
  Coffee,
  Edit3,
  FolderPlus,
  Hexagon,
  Loader2,
  Plus,
  RefreshCw,
  Search,
  Sparkles,
  Trash2,
  X,
} from 'lucide-react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { envEditorApi } from '../../src/plugins/api.gen';
import { ConfirmDialog } from '../../src/components/ConfirmDialog';
import type {
  EnvSnapshot,
  EnvVar,
  PresetKind,
  PresetResult,
  Scope,
} from '../../src/types';
import './EnvEditorView.css';

type Tab = 'env' | 'path' | 'presets';
type Toast = { kind: 'success' | 'error'; text: string } | null;

const PRESET_META: Array<{
  kind: PresetKind;
  name: string;
  Icon: ComponentType<{ size?: number }>;
  hint: string;
}> = [
  { kind: 'java', name: 'Java', Icon: Coffee, hint: '选择 JDK 安装目录 (e.g. D:\\jdk-17)' },
  { kind: 'python', name: 'Python', Icon: Code, hint: '选择 Python 安装根目录 (含 python.exe)' },
  { kind: 'node', name: 'Node.js', Icon: Hexagon, hint: '选择 Node 安装目录 (含 node.exe)' },
  { kind: 'go', name: 'Go', Icon: Box, hint: '选择 Go 安装目录 (含 bin\\go.exe)' },
  { kind: 'rust', name: 'Rust', Icon: Cog, hint: '选择 .cargo\\bin 目录 (含 cargo.exe)' },
];

const VAR_NAME_RE = /^[A-Za-z_][A-Za-z0-9_()]*$/;

function isValidVarName(name: string): boolean {
  if (!name) return false;
  if (name.includes('=')) return false;
  return VAR_NAME_RE.test(name);
}

export function EnvEditorView() {
  const [tab, setTab] = useState<Tab>('env');
  const [snapshot, setSnapshot] = useState<EnvSnapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [toast, setToast] = useState<Toast>(null);
  const toastTimer = useRef<number | null>(null);

  const showToast = useCallback((kind: 'success' | 'error', text: string) => {
    setToast({ kind, text });
    if (toastTimer.current) {
      window.clearTimeout(toastTimer.current);
    }
    toastTimer.current = window.setTimeout(() => setToast(null), 3000);
  }, []);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const data = await envEditorApi.listEnv();
      setSnapshot(data);
      if (data.warnings.length > 0) {
        showToast('error', data.warnings.join('；'));
      }
    } catch (e) {
      showToast('error', `加载失败: ${String(e)}`);
    } finally {
      setLoading(false);
    }
  }, [showToast]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  useEffect(() => {
    return () => {
      if (toastTimer.current) {
        window.clearTimeout(toastTimer.current);
      }
    };
  }, []);

  return (
    <div className="env-editor">
      <div className="env-editor__header">
        <h2 className="env-editor__title">环境变量编辑器</h2>
        <div className="env-editor__tab-bar">
          <TabButton active={tab === 'env'} onClick={() => setTab('env')}>
            全部变量
          </TabButton>
          <TabButton active={tab === 'path'} onClick={() => setTab('path')}>
            PATH
          </TabButton>
          <TabButton active={tab === 'presets'} onClick={() => setTab('presets')}>
            一键配置
          </TabButton>
        </div>
        <button
          className="env-editor__refresh"
          onClick={refresh}
          disabled={loading}
          title="刷新"
        >
          {loading ? <Loader2 size={14} className="spin" /> : <RefreshCw size={14} />}
          刷新
        </button>
      </div>

      {snapshot && tab === 'env' && (
        <EnvTab
          vars={snapshot.vars}
          onChanged={refresh}
          showToast={showToast}
        />
      )}
      {snapshot && tab === 'path' && (
        <PathTab
          pathUser={snapshot.path_user}
          pathSystem={snapshot.path_system}
          onChanged={refresh}
          showToast={showToast}
        />
      )}
      {tab === 'presets' && <PresetsTab showToast={showToast} />}

      {toast && (
        <div className={`env-editor__toast env-editor__toast--${toast.kind}`}>
          {toast.kind === 'error' ? (
            <AlertCircle size={16} />
          ) : (
            <Check size={16} />
          )}
          <span>{toast.text}</span>
        </div>
      )}
    </div>
  );
}

function TabButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      className={`env-editor__tab${active ? ' env-editor__tab--active' : ''}`}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

// -------------------- Env Tab --------------------

function EnvTab({
  vars,
  onChanged,
  showToast,
}: {
  vars: EnvVar[];
  onChanged: () => void | Promise<void>;
  showToast: (kind: 'success' | 'error', text: string) => void;
}) {
  const [query, setQuery] = useState('');
  const [editing, setEditing] = useState<
    { var: EnvVar } | { isNew: true; defaultScope: Scope } | null
  >(null);
  const [pendingDelete, setPendingDelete] = useState<EnvVar | null>(null);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return vars;
    return vars.filter(
      (v) =>
        v.name.toLowerCase().includes(q) ||
        v.value.toLowerCase().includes(q),
    );
  }, [vars, query]);

  const handleDelete = useCallback(
    async (v: EnvVar) => {
      try {
        await envEditorApi.deleteUserVar({ scope: v.scope, name: v.name });
        showToast('success', `已删除 ${v.name} (${v.scope === 'user' ? '用户' : '系统'})`);
        setPendingDelete(null);
        await onChanged();
      } catch (e) {
        showToast('error', `删除失败: ${String(e)}`);
      }
    },
    [onChanged, showToast],
  );

  return (
    <div className="env-editor__panel">
      <div className="env-editor__toolbar">
        <div className="env-editor__search">
          <Search size={14} />
          <input
            type="text"
            placeholder="搜索变量名或值"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>
        <button
          className="env-editor__btn env-editor__btn--primary"
          onClick={() => setEditing({ isNew: true, defaultScope: 'user' })}
        >
          <Plus size={14} /> 新建
        </button>
      </div>

      <div className="env-editor__table">
        {filtered.length === 0 ? (
          <div className="env-editor__empty">
            {query ? `没有匹配 "${query}" 的变量` : '没有环境变量'}
          </div>
        ) : (
          <table>
            <thead>
              <tr>
                <th style={{ width: '24%' }}>名称</th>
                <th>值</th>
                <th style={{ width: '90px' }}>作用域</th>
                <th style={{ width: '110px' }}>操作</th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((v) => {
                const key = `${v.scope}-${v.name}`;
                return (
                  <tr key={key}>
                    <td className="env-editor__cell-name">{v.name}</td>
                    <td className="env-editor__cell-value" title={v.value}>
                      {v.value}
                    </td>
                    <td>
                      <span
                        className={`env-editor__tag env-editor__tag--${v.scope}`}
                        title={v.source === 'process' ? '进程继承 (未持久化)' : ''}
                      >
                        {v.scope === 'user' ? '用户' : '系统'}
                        {v.source === 'process' && ' · 继承'}
                      </span>
                    </td>
                    <td className="env-editor__cell-actions">
                      <button
                        className="env-editor__icon-btn"
                        title={v.scope === 'system' ? '系统变量需管理员权限' : '编辑'}
                        onClick={() => setEditing({ var: v })}
                      >
                        <Edit3 size={14} />
                      </button>
                      <button
                        className="env-editor__icon-btn env-editor__icon-btn--danger"
                        title="删除"
                        onClick={() => setPendingDelete(v)}
                      >
                        <Trash2 size={14} />
                      </button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </div>

      {editing && (
        <AddEditDialog
          initial={'var' in editing ? editing.var : null}
          defaultScope={'var' in editing ? editing.var.scope : editing.defaultScope}
          onClose={() => setEditing(null)}
          onSaved={async () => {
            setEditing(null);
            await onChanged();
          }}
          showToast={showToast}
        />
      )}

      {pendingDelete && (
        <ConfirmDialog
          title={`确认删除${pendingDelete.scope === 'user' ? '用户' : '系统'}环境变量`}
          message={`确定要从 ${pendingDelete.scope === 'user' ? 'HKCU' : 'HKLM'} 移除 "${pendingDelete.name}" 吗？此操作会从注册表删除，开新 cmd 进程后生效。`}
          confirmLabel="确认删除"
          onConfirm={() => handleDelete(pendingDelete)}
          onCancel={() => setPendingDelete(null)}
        />
      )}
    </div>
  );
}

function AddEditDialog({
  initial,
  defaultScope,
  onClose,
  onSaved,
  showToast,
}: {
  initial: EnvVar | null;
  defaultScope: Scope;
  onClose: () => void;
  onSaved: () => void | Promise<void>;
  showToast: (kind: 'success' | 'error', text: string) => void;
}) {
  const isNew = initial === null;
  const [name, setName] = useState(initial?.name ?? '');
  const [value, setValue] = useState(initial?.value ?? '');
  const [scope, setScope] = useState<Scope>(defaultScope);
  const [saving, setSaving] = useState(false);

  const nameValid = isValidVarName(name) || (!isNew && name === initial?.name);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);

  const handleSave = useCallback(async () => {
    if (!isValidVarName(name)) {
      showToast('error', '变量名非法');
      return;
    }
    setSaving(true);
    try {
      await envEditorApi.setUserVar({ scope, name, value });
      showToast(
        'success',
        isNew
          ? `已新建 ${name} (${scope === 'user' ? '用户' : '系统'})`
          : `已更新 ${name} (${scope === 'user' ? '用户' : '系统'})`,
      );
      await onSaved();
    } catch (e) {
      showToast('error', `保存失败: ${String(e)}`);
    } finally {
      setSaving(false);
    }
  }, [name, value, scope, isNew, showToast, onSaved]);

  return (
    <div className="env-editor__dialog-backdrop" onMouseDown={onClose}>
      <div
        className="env-editor__dialog"
        onMouseDown={(e) => e.stopPropagation()}
      >
        <div className="env-editor__dialog-header">
          <h3>{isNew ? '新建环境变量' : `编辑 ${initial?.name}`}</h3>
          <button className="env-editor__icon-btn" onClick={onClose}>
            <X size={16} />
          </button>
        </div>
        <div className="env-editor__dialog-body">
          <div className="env-editor__field">
            <span>作用域</span>
            <div className="env-editor__radio-group">
              <label className="env-editor__radio">
                <input
                  type="radio"
                  name="scope"
                  value="user"
                  checked={scope === 'user'}
                  onChange={() => setScope('user')}
                />
                <span>用户 (HKCU)</span>
              </label>
              <label className="env-editor__radio">
                <input
                  type="radio"
                  name="scope"
                  value="system"
                  checked={scope === 'system'}
                  onChange={() => setScope('system')}
                />
                <span>系统 (HKLM, 需管理员)</span>
              </label>
            </div>
          </div>
          <label className="env-editor__field">
            <span>名称</span>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="如 JAVA_HOME"
              className={name && !nameValid ? 'is-invalid' : ''}
              autoFocus
            />
            {name && !nameValid && (
              <small className="env-editor__hint-error">
                名称只能包含字母、数字、下划线，且不能以数字开头
              </small>
            )}
          </label>
          <label className="env-editor__field">
            <span>值</span>
            <textarea
              value={value}
              onChange={(e) => setValue(e.target.value)}
              placeholder="变量值（可空）"
              rows={5}
            />
          </label>
        </div>
        <div className="env-editor__dialog-footer">
          <button className="env-editor__btn" onClick={onClose}>
            取消
          </button>
          <button
            className="env-editor__btn env-editor__btn--primary"
            disabled={!nameValid || saving}
            onClick={handleSave}
          >
            {saving ? <Loader2 size={14} className="spin" /> : <Check size={14} />}
            保存
          </button>
        </div>
      </div>
    </div>
  );
}

// -------------------- PATH Tab --------------------

function PathTab({
  pathUser,
  pathSystem,
  onChanged,
  showToast,
}: {
  pathUser: string[];
  pathSystem: string[];
  onChanged: () => void | Promise<void>;
  showToast: (kind: 'success' | 'error', text: string) => void;
}) {
  const [scope, setScope] = useState<Scope>('user');
  const [entries, setEntries] = useState<string[]>(pathUser);
  const [saving, setSaving] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [highlightDups, setHighlightDups] = useState(false);

  // When scope changes, reset entries from the corresponding snapshot
  useEffect(() => {
    setEntries(scope === 'user' ? pathUser : pathSystem);
    setDirty(false);
  }, [scope, pathUser, pathSystem]);

  const dupSet = useMemo(() => {
    const seen = new Set<string>();
    const dups = new Set<string>();
    for (const e of entries) {
      const k = e.toLowerCase();
      if (seen.has(k)) dups.add(e);
      else seen.add(k);
    }
    return dups;
  }, [entries]);

  const moveUp = useCallback((idx: number) => {
    if (idx <= 0) return;
    setEntries((prev) => {
      const next = prev.slice();
      [next[idx - 1], next[idx]] = [next[idx], next[idx - 1]];
      return next;
    });
    setDirty(true);
  }, []);

  const moveDown = useCallback((idx: number) => {
    setEntries((prev) => {
      if (idx >= prev.length - 1) return prev;
      const next = prev.slice();
      [next[idx], next[idx + 1]] = [next[idx + 1], next[idx]];
      return next;
    });
    setDirty(true);
  }, []);

  const removeAt = useCallback((idx: number) => {
    setEntries((prev) => prev.filter((_, i) => i !== idx));
    setDirty(true);
  }, []);

  const addDirectory = useCallback(async () => {
    try {
      const selected = await openDialog({ directory: true, multiple: false });
      if (typeof selected === 'string' && selected) {
        setEntries((prev) => {
          const k = selected.toLowerCase();
          if (prev.some((p) => p.toLowerCase() === k)) {
            showToast('error', `${selected} 已在 PATH 中`);
            return prev;
          }
          return [...prev, selected];
        });
        setDirty(true);
      }
    } catch (e) {
      showToast('error', `选择目录失败: ${String(e)}`);
    }
  }, [showToast]);

  const dedup = useCallback(() => {
    setEntries((prev) => {
      const seen = new Set<string>();
      const out: string[] = [];
      for (const e of prev) {
        const k = e.toLowerCase();
        if (!seen.has(k)) {
          seen.add(k);
          out.push(e);
        }
      }
      return out;
    });
    setDirty(true);
  }, []);

  const save = useCallback(async () => {
    setSaving(true);
    try {
      await envEditorApi.setPathEntries({ scope, entries });
      showToast('success', `PATH (${scope === 'user' ? '用户' : '系统'}) 已保存 (${entries.length} 项)`);
      await onChanged();
    } catch (e) {
      showToast('error', `保存失败: ${String(e)}`);
    } finally {
      setSaving(false);
    }
  }, [scope, entries, onChanged, showToast]);

  return (
    <div className="env-editor__panel">
      <div className="env-editor__toolbar">
        <div className="env-editor__radio-group env-editor__radio-group--inline">
          <label className="env-editor__radio">
            <input
              type="radio"
              name="path-scope"
              value="user"
              checked={scope === 'user'}
              onChange={() => setScope('user')}
            />
            <span>用户 PATH ({pathUser.length})</span>
          </label>
          <label className="env-editor__radio">
            <input
              type="radio"
              name="path-scope"
              value="system"
              checked={scope === 'system'}
              onChange={() => setScope('system')}
            />
            <span>系统 PATH ({pathSystem.length}, 需管理员)</span>
          </label>
        </div>
        <div className="env-editor__toolbar-spacer" />
        <button className="env-editor__btn" onClick={addDirectory}>
          <FolderPlus size={14} /> 添加目录
        </button>
        <button
          className="env-editor__btn"
          onClick={() => setHighlightDups((v) => !v)}
          disabled={dupSet.size === 0}
        >
          {highlightDups ? '隐藏' : '高亮'}重复 ({dupSet.size})
        </button>
        <button
          className="env-editor__btn"
          onClick={dedup}
          disabled={dupSet.size === 0}
        >
          一键去重
        </button>
        <button
          className="env-editor__btn env-editor__btn--primary"
          onClick={save}
          disabled={!dirty || saving}
        >
          {saving ? <Loader2 size={14} className="spin" /> : <Check size={14} />}
          保存顺序
        </button>
      </div>

      <div className="env-editor__path-list">
        {entries.length === 0 ? (
          <div className="env-editor__empty">
            {scope === 'user' ? '用户' : '系统'} PATH 为空
          </div>
        ) : (
          <ol>
            {entries.map((entry, idx) => {
              const isDup = highlightDups && dupSet.has(entry);
              return (
                <li
                  key={`${idx}-${entry}`}
                  className={isDup ? 'env-editor__path-item--dup' : ''}
                >
                  <span className="env-editor__path-idx">{idx + 1}</span>
                  <span className="env-editor__path-value" title={entry}>
                    {entry}
                  </span>
                  <span className="env-editor__path-actions">
                    <button
                      className="env-editor__icon-btn"
                      title="上移"
                      onClick={() => moveUp(idx)}
                      disabled={idx === 0}
                    >
                      <ChevronUp size={14} />
                    </button>
                    <button
                      className="env-editor__icon-btn"
                      title="下移"
                      onClick={() => moveDown(idx)}
                      disabled={idx === entries.length - 1}
                    >
                      <ChevronDown size={14} />
                    </button>
                    <button
                      className="env-editor__icon-btn env-editor__icon-btn--danger"
                      title="删除"
                      onClick={() => removeAt(idx)}
                    >
                      <Trash2 size={14} />
                    </button>
                  </span>
                </li>
              );
            })}
          </ol>
        )}
      </div>
    </div>
  );
}

// -------------------- Presets Tab --------------------

function PresetsTab({
  showToast,
}: {
  showToast: (kind: 'success' | 'error', text: string) => void;
}) {
  const [picking, setPicking] = useState<PresetKind | null>(null);
  const [preview, setPreview] = useState<PresetResult | null>(null);
  const [scope, setScope] = useState<Scope>('user');
  const [confirming, setConfirming] = useState(false);
  const [applying, setApplying] = useState(false);

  const pickDirectory = useCallback(
    async (kind: PresetKind) => {
      setPicking(kind);
      setPreview(null);
      try {
        const selected = await openDialog({ directory: true, multiple: false });
        if (typeof selected !== 'string' || !selected) {
          setPicking(null);
          return;
        }
        const result = await envEditorApi.detectPreset({ kind, dir: selected });
        setPreview(result);
      } catch (e) {
        showToast('error', `探测失败: ${String(e)}`);
      } finally {
        setPicking(null);
      }
    },
    [showToast],
  );

  const apply = useCallback(async () => {
    if (!preview) return;
    setApplying(true);
    try {
      const plan = { ...preview.plan, scope };
      const r = await envEditorApi.applyPreset(plan);
      showToast('success', `已应用 (${r.applied.length} 项)`);
      setConfirming(false);
      setPreview(null);
    } catch (e) {
      showToast('error', `应用失败: ${String(e)}`);
    } finally {
      setApplying(false);
    }
  }, [preview, scope, showToast]);

  return (
    <div className="env-editor__panel env-editor__panel--presets">
      <div className="env-editor__presets-hint">
        <Sparkles size={14} />
        为常用语言/运行环境一键配置 <code>JAVA_HOME</code> / <code>GOROOT</code> 等变量，并 prepend 到 PATH。
        写入注册表后，<strong>开新的 cmd 进程即可生效</strong>。
      </div>

      <div className="env-editor__preset-grid">
        {PRESET_META.map((p) => {
          const isPicking = picking === p.kind;
          const matched = preview?.preset === p.kind;
          const Icon = p.Icon;
          return (
            <div
              key={p.kind}
              className={`env-editor__preset-card${matched ? ' env-editor__preset-card--matched' : ''}`}
            >
              <div className="env-editor__preset-icon">
                <Icon size={32} />
              </div>
              <div className="env-editor__preset-name">{p.name}</div>
              <div className="env-editor__preset-hint">{p.hint}</div>
              <button
                className="env-editor__btn env-editor__btn--primary"
                onClick={() => pickDirectory(p.kind)}
                disabled={isPicking}
              >
                {isPicking ? <Loader2 size={14} className="spin" /> : <FolderPlus size={14} />}
                选择目录
              </button>
            </div>
          );
        })}
      </div>

      {preview && (
        <div className="env-editor__preset-preview">
          <h4>预览: {PRESET_META.find((p) => p.kind === preview.preset)?.name}</h4>
          <div className="env-editor__field">
            <span>应用到作用域</span>
            <div className="env-editor__radio-group">
              <label className="env-editor__radio">
                <input
                  type="radio"
                  name="preset-scope"
                  value="user"
                  checked={scope === 'user'}
                  onChange={() => setScope('user')}
                />
                <span>用户 (HKCU)</span>
              </label>
              <label className="env-editor__radio">
                <input
                  type="radio"
                  name="preset-scope"
                  value="system"
                  checked={scope === 'system'}
                  onChange={() => setScope('system')}
                />
                <span>系统 (HKLM, 需管理员)</span>
              </label>
            </div>
          </div>
          {preview.warnings.length > 0 && (
            <ul className="env-editor__warnings">
              {preview.warnings.map((w, i) => (
                <li key={i}>{w}</li>
              ))}
            </ul>
          )}
          {preview.plan.vars.length > 0 && (
            <>
              <div className="env-editor__preview-label">将设置/覆盖的变量:</div>
              <ul className="env-editor__preview-list">
                {preview.plan.vars.map((v) => (
                  <li key={v.name}>
                    <code>{v.name}</code> = <code>{v.value}</code>
                  </li>
                ))}
              </ul>
            </>
          )}
          {preview.plan.path_prepend.length > 0 && (
            <>
              <div className="env-editor__preview-label">将 prepend 到 PATH:</div>
              <ul className="env-editor__preview-list">
                {preview.plan.path_prepend.map((p, i) => (
                  <li key={`p-${i}`}>
                    <code>{p}</code>
                  </li>
                ))}
              </ul>
            </>
          )}
          {preview.plan.path_append.length > 0 && (
            <>
              <div className="env-editor__preview-label">将 append 到 PATH:</div>
              <ul className="env-editor__preview-list">
                {preview.plan.path_append.map((p, i) => (
                  <li key={`a-${i}`}>
                    <code>{p}</code>
                  </li>
                ))}
              </ul>
            </>
          )}
          <div className="env-editor__preview-actions">
            <button
              className="env-editor__btn"
              onClick={() => setPreview(null)}
            >
              取消
            </button>
            <button
              className="env-editor__btn env-editor__btn--primary"
              onClick={() => setConfirming(true)}
            >
              <Check size={14} /> 应用
            </button>
          </div>
        </div>
      )}

      {confirming && preview && (
        <ConfirmDialog
          title="确认应用 preset"
          message={`将${scope === 'user' ? '写入 HKCU' : '写入 HKLM (需管理员)'}，修改 ${preview.plan.vars.length} 个环境变量并改写 PATH。修改会立即写入注册表，开新 cmd 生效。是否继续？`}
          confirmLabel={applying ? '应用中…' : '确认应用'}
          onConfirm={apply}
          onCancel={() => setConfirming(false)}
        />
      )}
    </div>
  );
}
