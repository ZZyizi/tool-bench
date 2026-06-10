import { useEffect, useRef, useState } from 'react';
import { useSettings, type AppMode, type CloseBehavior } from '../settings';
import './SettingsPanel.css';

interface SettingsPanelProps {
  onClose: () => void;
}

interface RadioOption<T extends string> {
  value: T;
  label: string;
  description: string;
}

const MODE_OPTIONS: RadioOption<AppMode>[] = [
  {
    value: 'desktop',
    label: '桌面模式',
    description: '主窗口变为启动器，点击工具图标打开独立窗口。',
  },
  {
    value: 'embedded',
    label: '嵌入式模式',
    description: '保留经典的侧边栏 + 主区布局，所有工具在主窗口内切换。',
  },
];

const CLOSE_OPTIONS: RadioOption<CloseBehavior>[] = [
  {
    value: 'hide',
    label: '隐藏到托盘',
    description: '关闭主窗口后进程继续在托盘运行，可从托盘恢复。',
  },
  {
    value: 'quit',
    label: '直接退出',
    description: '关闭主窗口即退出整个应用（托盘也会一起消失）。',
  },
];

/** Strip the `Key`/`Digit` prefix from KeyboardEvent.code for display. */
function formatKeyDisplay(code: string): string {
  if (code.startsWith('Key')) return code.slice(3);
  if (code.startsWith('Digit')) return code.slice(5);
  if (code === 'Space') return 'Space';
  if (code === 'ArrowUp') return '↑';
  if (code === 'ArrowDown') return '↓';
  if (code === 'ArrowLeft') return '←';
  if (code === 'ArrowRight') return '→';
  return code;
}

/**
 * A small button that toggles into "recording" mode, captures a key combo,
 * and calls `onChange` with a Shortcut-parser-compatible string.
 */
function ShortcutRecorder({
  value,
  onChange,
}: {
  value: string;
  onChange: (shortcut: string) => void;
}) {
  const [recording, setRecording] = useState(false);
  const [pending, setPending] = useState('');
  const btnRef = useRef<HTMLButtonElement>(null);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLButtonElement>) => {
    e.preventDefault();
    e.stopPropagation();

    if (e.key === 'Escape') {
      setRecording(false);
      setPending('');
      return;
    }

    const parts: string[] = [];
    if (e.ctrlKey) parts.push('Ctrl');
    if (e.altKey) parts.push('Alt');
    if (e.shiftKey) parts.push('Shift');
    if (e.metaKey) parts.push('Meta');

    const modKeys = new Set(['Control', 'Alt', 'Shift', 'Meta']);
    if (modKeys.has(e.key)) {
      // Only modifiers pressed so far — show preview
      setPending(parts.join('+') + '+');
      return;
    }

    // Must have at least one modifier
    if (parts.length === 0) return;

    parts.push(e.code);
    const shortcut = parts.join('+');
    onChange(shortcut);
    setRecording(false);
    setPending('');
  };

  useEffect(() => {
    if (recording) {
      btnRef.current?.focus();
    }
  }, [recording]);

  if (recording) {
    return (
      <button
        type="button"
        className="settings-panel__shortcut-btn settings-panel__shortcut-btn--recording"
        ref={btnRef}
        onKeyDown={handleKeyDown}
        onBlur={() => { setRecording(false); setPending(''); }}
        tabIndex={0}
      >
        {pending || '按下快捷键...'}
      </button>
    );
  }

  // Build a display-friendly version of the stored shortcut
  const displayValue = value
    .split('+')
    .map((part, i, arr) => (i === arr.length - 1 ? formatKeyDisplay(part) : part))
    .join('+');

  return (
    <button
      type="button"
      className="settings-panel__shortcut-btn"
      onClick={() => setRecording(true)}
      title="点击修改快捷键"
    >
      <kbd>{displayValue}</kbd>
      <span className="settings-panel__shortcut-hint">点击修改</span>
    </button>
  );
}

export function SettingsPanel({ onClose }: SettingsPanelProps) {
  const [settings, setSettings] = useSettings();
  const panelRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKey);
    panelRef.current?.focus();
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);

  const updateMode = (mode: AppMode) => {
    if (mode === settings.mode) return;
    setSettings({ ...settings, mode });
  };

  const updateClose = (closeBehavior: CloseBehavior) => {
    if (closeBehavior === settings.closeBehavior) return;
    setSettings({ ...settings, closeBehavior });
  };

  return (
    <div className="settings-panel__backdrop" onClick={onClose}>
      <div
        className="settings-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-panel-title"
        ref={panelRef}
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
      >
        <div className="settings-panel__header">
          <h3 id="settings-panel-title" className="settings-panel__title">
            设置
          </h3>
          <button
            type="button"
            className="settings-panel__close"
            onClick={onClose}
            aria-label="关闭设置"
          >
            ×
          </button>
        </div>

        <section className="settings-panel__section">
          <h4 className="settings-panel__section-title">交互模式</h4>
          <div className="settings-panel__options">
            {MODE_OPTIONS.map((opt) => {
              const active = settings.mode === opt.value;
              return (
                <label
                  key={opt.value}
                  className={`settings-panel__option${active ? ' settings-panel__option--active' : ''}`}
                >
                  <input
                    type="radio"
                    name="mode"
                    value={opt.value}
                    checked={active}
                    onChange={() => updateMode(opt.value)}
                  />
                  <span className="settings-panel__option-label">{opt.label}</span>
                  <span className="settings-panel__option-desc">{opt.description}</span>
                </label>
              );
            })}
          </div>
        </section>

        <section className="settings-panel__section">
          <h4 className="settings-panel__section-title">关闭主窗口时</h4>
          <div className="settings-panel__options">
            {CLOSE_OPTIONS.map((opt) => {
              const active = settings.closeBehavior === opt.value;
              return (
                <label
                  key={opt.value}
                  className={`settings-panel__option${active ? ' settings-panel__option--active' : ''}`}
                >
                  <input
                    type="radio"
                    name="closeBehavior"
                    value={opt.value}
                    checked={active}
                    onChange={() => updateClose(opt.value)}
                  />
                  <span className="settings-panel__option-label">{opt.label}</span>
                  <span className="settings-panel__option-desc">{opt.description}</span>
                </label>
              );
            })}
          </div>
        </section>

        <section className="settings-panel__section">
          <h4 className="settings-panel__section-title">全局快捷键</h4>
          <p className="settings-panel__section-desc">
            设置打开快速启动面板的全局快捷键。点击下方按钮，然后按下你想要的组合键。
          </p>
          <ShortcutRecorder
            value={settings.quickLaunchShortcut}
            onChange={(shortcut) => setSettings({ ...settings, quickLaunchShortcut: shortcut })}
          />
        </section>

        <div className="settings-panel__actions">
          <button type="button" className="settings-panel__btn" onClick={onClose}>
            完成
          </button>
        </div>
      </div>
    </div>
  );
}
