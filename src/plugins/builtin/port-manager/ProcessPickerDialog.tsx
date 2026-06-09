import { useEffect, useMemo, useRef, useState } from 'react';
import { Eraser, Search } from 'lucide-react';
import './ProcessPickerDialog.css';

interface ProcessPickerDialogProps {
  processNames: string[];
  onClose: () => void;
  onConfirm: (name: string) => void;
}

export function ProcessPickerDialog({ processNames, onClose, onConfirm }: ProcessPickerDialogProps) {
  const [selected, setSelected] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKey);
    searchRef.current?.focus();
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);

  const filteredNames = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return processNames;
    return processNames.filter((n) => n.toLowerCase().includes(q));
  }, [processNames, query]);

  const handleConfirm = () => {
    if (selected) onConfirm(selected);
  };

  // Keep selection valid if the active search hides it.
  const effectiveSelected = selected && filteredNames.includes(selected) ? selected : null;

  return (
    <div className="process-picker__backdrop" onClick={onClose}>
      <div
        className="process-picker"
        role="dialog"
        aria-modal="true"
        aria-labelledby="process-picker-title"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="process-picker__header">
          <h3 id="process-picker-title" className="process-picker__title">
            <Eraser size={16} aria-hidden style={{ verticalAlign: '-3px', marginRight: 6 }} />
            一键清理
          </h3>
          <button
            type="button"
            className="process-picker__close"
            onClick={onClose}
            aria-label="关闭"
          >
            ×
          </button>
        </div>

        <p className="process-picker__hint">
          选择一个进程名，将结束所有同名进程（占用端口的全部 PID）。
        </p>

        <div className="process-picker__search-row">
          <Search size={14} aria-hidden className="process-picker__search-icon" />
          <input
            ref={searchRef}
            type="text"
            className="process-picker__search"
            placeholder="搜索进程名"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            spellCheck={false}
            autoComplete="off"
          />
        </div>

        <div className="process-picker__list" role="listbox" aria-label="可清理的进程名">
          {processNames.length === 0 ? (
            <div className="process-picker__empty">当前列表中没有可清理的进程。</div>
          ) : filteredNames.length === 0 ? (
            <div className="process-picker__empty">没有匹配 "{query}" 的进程</div>
          ) : (
            filteredNames.map((name) => {
              const isSelected = effectiveSelected === name;
              return (
                <button
                  type="button"
                  key={name}
                  role="option"
                  aria-selected={isSelected}
                  className={`process-picker__item${isSelected ? ' process-picker__item--selected' : ''}`}
                  onClick={() => setSelected(name)}
                  title={name}
                >
                  <span className="process-picker__item-name">{name}</span>
                </button>
              );
            })
          )}
        </div>

        <div className="process-picker__actions">
          <button type="button" className="process-picker__btn process-picker__btn--cancel" onClick={onClose}>
            取消
          </button>
          <button
            type="button"
            className="process-picker__btn process-picker__btn--confirm"
            onClick={handleConfirm}
            disabled={!effectiveSelected}
          >
            一键释放
          </button>
        </div>
      </div>
    </div>
  );
}
