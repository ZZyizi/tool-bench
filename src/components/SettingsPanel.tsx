import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
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

/** Map `KeyboardEvent.code` → the token tauri's Shortcut parser accepts. */
function codeToShortcutToken(code: string): string | null {
  if (code.startsWith('Key')) return code.slice(3);
  if (code.startsWith('Digit')) return code.slice(5);
  // Tauri's parser uses the literal name for these.
  if (code === 'Space') return 'Space';
  if (code === 'Enter') return 'Enter';
  if (code === 'Tab') return 'Tab';
  if (code === 'Backspace') return 'Backspace';
  if (code === 'Delete') return 'Delete';
  if (code === 'Insert') return 'Insert';
  if (code === 'Home') return 'Home';
  if (code === 'End') return 'End';
  if (code === 'PageUp') return 'PageUp';
  if (code === 'PageDown') return 'PageDown';
  if (code === 'ArrowUp') return 'Up';
  if (code === 'ArrowDown') return 'Down';
  if (code === 'ArrowLeft') return 'Left';
  if (code === 'ArrowRight') return 'Right';
  if (code === 'Escape') return 'Escape';
  if (code.startsWith('F') && /^F\d{1,2}$/.test(code)) return code;
  // Unknown / unsupported — caller decides what to do.
  return null;
}

const MODIFIER_KEYS = new Set([
  'Control',
  'Alt',
  'AltGraph',
  'Shift',
  'Meta',
  'OS',
]);

/**
 * A button that toggles into "recording" mode and captures a key combo via a
 * window-level keydown listener. We can't use the button's own `onKeyDown`
 * because pressing Space on a focused button triggers the browser's native
 * activation handling and steals the event — that's why Space couldn't be
 * bound before.
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

  useEffect(() => {
    if (!recording) return;

    // Tell the backend to step aside: unregister global shortcuts and stop
    // swallowing Alt+Space. Otherwise Ctrl+Space / Alt+Space never reach
    // this window.
    invoke('set_recording_mode', { recording: true }).catch((err) => {
      console.error('[shortcut] failed to enter recording mode', err);
    });

    const handleKeyDown = (e: KeyboardEvent) => {
      // Stop the event from reaching the focused button (or anything else).
      e.preventDefault();
      e.stopPropagation();

      if (e.key === 'Escape') {
        setRecording(false);
        setPending('');
        return;
      }

      const modifiers: string[] = [];
      if (e.ctrlKey) modifiers.push('Ctrl');
      if (e.altKey) modifiers.push('Alt');
      if (e.shiftKey) modifiers.push('Shift');
      if (e.metaKey) modifiers.push('Super');

      // Modifier-only keystroke — show a preview but don't commit yet.
      if (MODIFIER_KEYS.has(e.key)) {
        setPending(modifiers.length ? modifiers.join('+') + '+' : '');
        return;
      }

      // Must have at least one modifier — a bare Space (or letter) would
      // hijack typing globally.
      if (modifiers.length === 0) {
        setPending('需要先按下 Ctrl / Alt / Shift / Win');
        return;
      }

      const token = codeToShortcutToken(e.code);
      if (!token) {
        setPending(`不支持的键: ${e.code}`);
        return;
      }

      const shortcut = [...modifiers, token].join('+');
      onChange(shortcut);
      setRecording(false);
      setPending('');
    };

    // `capture: true` so we beat any other handler (including the
    // settings-panel Escape handler).
    window.addEventListener('keydown', handleKeyDown, { capture: true });
    return () => {
      window.removeEventListener('keydown', handleKeyDown, { capture: true });
      // Restore: re-register the saved shortcut and re-enable Alt+Space
      // suppression. set_settings (called from onChange→useSettings) also
      // re-registers, so this is safe to invoke either way.
      invoke('set_recording_mode', { recording: false }).catch((err) => {
        console.error('[shortcut] failed to exit recording mode', err);
      });
    };
  }, [recording, onChange]);

  if (recording) {
    return (
      <button
        type="button"
        className="settings-panel__shortcut-btn settings-panel__shortcut-btn--recording"
        onClick={(e) => {
          e.preventDefault();
          setRecording(false);
          setPending('');
        }}
        tabIndex={-1}
      >
        {pending || '按下快捷键...(Esc 取消)'}
      </button>
    );
  }

  // Build a display-friendly version of the stored shortcut.
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
            设置打开快速启动面板的全局快捷键。注意：Alt+Space 可能会被 Windows 系统拦截，建议使用 Ctrl+Space 或其他组合。
          </p>
          <label className="settings-panel__shortcut-label">快速启动快捷键</label>
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
