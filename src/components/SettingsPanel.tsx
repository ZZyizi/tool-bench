import { useEffect, useRef } from 'react';
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

        <div className="settings-panel__actions">
          <button type="button" className="settings-panel__btn" onClick={onClose}>
            完成
          </button>
        </div>
      </div>
    </div>
  );
}
