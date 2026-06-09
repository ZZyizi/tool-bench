import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Box, Pin, PinOff, Search, X, Zap } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { api } from '../plugins/api';
import { globalRegistry } from '../plugins/registry';
import { useSettings } from '../settings';
import type { InstalledApp } from '../types';
import './QuickSwitcher.css';

type Item =
  | {
      kind: 'tool';
      id: string;
      name: string;
      description: string;
      icon: LucideIcon;
      pluginId: string;
    }
  | {
      kind: 'app';
      id: string;
      name: string;
      target: string;
      icon: LucideIcon;
    };

const TOOL_PREFIX = 'tool:';
const CELL_PX = 96; // width of one cell — keep in sync with .qs__cell CSS

function buildToolItems(): Item[] {
  return globalRegistry.list().map((plugin) => ({
    kind: 'tool' as const,
    id: `${TOOL_PREFIX}${plugin.manifest.id}`,
    pluginId: plugin.manifest.id,
    name: plugin.manifest.name,
    description: plugin.manifest.description,
    icon: (plugin.manifest.icon ?? Box) as LucideIcon,
  }));
}

function buildAppItems(apps: InstalledApp[]): Item[] {
  return apps.map((app) => ({
    kind: 'app' as const,
    id: app.id,
    name: app.name,
    target: app.target,
    icon: Box,
  }));
}

function score(query: string, name: string): number {
  // Lower is better. Used to rank search results.
  const q = query.toLowerCase();
  const n = name.toLowerCase();
  if (n === q) return 0;
  if (n.startsWith(q)) return 1;
  const idx = n.indexOf(q);
  if (idx >= 0) return 2 + idx;
  return Number.POSITIVE_INFINITY;
}

export function QuickSwitcher() {
  const [settings, setSettings] = useSettings();
  const [query, setQuery] = useState('');
  const [toolItems] = useState<Item[]>(() => buildToolItems());
  const [installedApps, setInstalledApps] = useState<InstalledApp[] | null>(null);
  const [activeIndex, setActiveIndex] = useState(0);
  const [colCount, setColCount] = useState(1);
  const [error, setError] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const gridRef = useRef<HTMLDivElement>(null);

  // Focus the search field on mount. The window is always-on-top and
  // decorations=false, so users expect to start typing the moment Alt+Space
  // is released.
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // Lazy-scan installed apps on first use. The quick-switcher is the
  // entrypoint for the user, so we don't pay the cost at app startup.
  useEffect(() => {
    let cancelled = false;
    api
      .listInstalledApps()
      .then((result) => {
        if (cancelled) return;
        setInstalledApps(result.apps);
      })
      .catch((e) => {
        if (cancelled) return;
        setError(String(e));
        setInstalledApps([]);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // Track the grid's actual width so keyboard navigation can convert
  // a 1-D index into (row, col) correctly. The grid uses `auto-fill` so the
  // column count depends on the current window width.
  useEffect(() => {
    const el = gridRef.current;
    if (!el) return;
    const ro = new ResizeObserver(([entry]) => {
      const w = entry.contentRect.width;
      // Subtract the inline gap (4px) by floor() on (w + gap) / (cell + gap).
      setColCount(Math.max(1, Math.floor((w + 4) / CELL_PX)));
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  // Build the visible list. When the search box is empty, show pinned items
  // in their pinned order. When non-empty, replace the view with search
  // results across both tools and installed apps, ranked by score.
  const visible: Item[] = useMemo(() => {
    const appItems = installedApps ? buildAppItems(installedApps) : [];
    if (query.trim() === '') {
      const pinnedById = new Map<string, Item>();
      for (const t of toolItems) pinnedById.set(t.id, t);
      for (const a of appItems) pinnedById.set(a.id, a);
      return settings.pinnedApps
        .map((id) => pinnedById.get(id))
        .filter((x): x is Item => Boolean(x));
    }
    const q = query.trim();
    const candidates = [...toolItems, ...appItems];
    const scored = candidates
      .map((item) => ({ item, s: score(q, item.name) }))
      .filter((x) => x.s !== Number.POSITIVE_INFINITY)
      .sort((a, b) => a.s - b.s);
    return scored.slice(0, 64).map((x) => x.item);
  }, [query, toolItems, installedApps, settings.pinnedApps]);

  // Reset the active index whenever the visible list changes shape.
  useEffect(() => {
    setActiveIndex((i) => (i >= visible.length ? 0 : i));
  }, [visible.length]);

  const closeWindow = useCallback(async () => {
    try {
      await getCurrentWebviewWindow().hide();
    } catch (e) {
      console.error('[qs] failed to hide window', e);
    }
  }, []);

  const launchItem = useCallback(
    async (item: Item, useAndGo: boolean) => {
      setError(null);
      try {
        if (item.kind === 'tool') {
          const plugin = globalRegistry.get(item.pluginId);
          if (!plugin) throw new Error(`tool "${item.pluginId}" not registered`);
          await invoke('open_tool_window', {
            pluginId: item.pluginId,
            title: plugin.manifest.name,
            width: plugin.manifest.windowWidth ?? null,
            height: plugin.manifest.windowHeight ?? null,
            useAndGo,
          });
        } else {
          await api.launchApp(item.target);
        }
        if (useAndGo) {
          await closeWindow();
        }
      } catch (e) {
        setError(String(e));
      }
    },
    [closeWindow],
  );

  const togglePin = useCallback(
    (item: Item) => {
      const ids = settings.pinnedApps;
      if (ids.includes(item.id)) {
        setSettings({
          ...settings,
          pinnedApps: ids.filter((x) => x !== item.id),
        });
      } else {
        setSettings({
          ...settings,
          pinnedApps: [...ids, item.id],
        });
      }
    },
    [settings, setSettings],
  );

  const moveActive = useCallback(
    (deltaRow: number, deltaCol: number) => {
      if (visible.length === 0) return;
      setActiveIndex((i) => {
        const row = Math.floor(i / colCount);
        const col = i % colCount;
        const newRow = Math.max(0, Math.min(Math.floor((visible.length - 1) / colCount), row + deltaRow));
        const newCol = Math.max(0, Math.min(colCount - 1, col + deltaCol));
        const next = newRow * colCount + newCol;
        return Math.min(next, visible.length - 1);
      });
    },
    [colCount, visible.length],
  );

  const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      void closeWindow();
      return;
    }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      moveActive(1, 0);
      return;
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      moveActive(-1, 0);
      return;
    }
    if (e.key === 'ArrowRight') {
      e.preventDefault();
      moveActive(0, 1);
      return;
    }
    if (e.key === 'ArrowLeft') {
      e.preventDefault();
      moveActive(0, -1);
      return;
    }
    if (e.key === 'Enter') {
      e.preventDefault();
      const item = visible[activeIndex];
      if (item) void launchItem(item, false);
      return;
    }
  };

  const clearQuery = () => {
    setQuery('');
    inputRef.current?.focus();
  };

  const isPinned = (id: string) => settings.pinnedApps.includes(id);
  const showEmptyHint = query.trim() === '' && visible.length === 0;
  const showNoMatch = query.trim() !== '' && visible.length === 0;

  return (
    <div className="qs">
      <div className="qs__row qs__row--input">
        <Search size={16} className="qs__search-icon" aria-hidden />
        <input
          ref={inputRef}
          className="qs__input"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder={
            query.trim() === ''
              ? '搜索应用或工具...  (↑↓←→ 移动 · Enter 打开 · Esc 关闭)'
              : '输入中...  (✕ 清空恢复已固定)'
          }
          spellCheck={false}
          autoComplete="off"
        />
        {query && (
          <button
            type="button"
            className="qs__clear"
            onClick={clearQuery}
            aria-label="清空搜索"
            title="清空"
          >
            <X size={14} aria-hidden />
          </button>
        )}
      </div>

      <div className="qs__row qs__row--result" ref={gridRef} role="listbox" aria-label="搜索结果">
        {visible.length > 0 ? (
          <div className="qs__grid">
            {visible.map((item, i) => {
              const Icon = item.icon;
              const active = i === activeIndex;
              const pinned = isPinned(item.id);
              return (
                <div
                  key={item.id}
                  className={`qs__cell${active ? ' qs__cell--active' : ''}`}
                  role="option"
                  aria-selected={active}
                  tabIndex={-1}
                  onMouseEnter={() => setActiveIndex(i)}
                  onClick={(e) => {
                    // Clicks on the action buttons are caught by their own
                    // onClick + stopPropagation, so a click on the cell body
                    // here means "open this item".
                    if (e.defaultPrevented) return;
                    void launchItem(item, false);
                  }}
                >
                  <div className="qs__cell-actions">
                    <button
                      type="button"
                      className="qs__cell-action"
                      onClick={(e) => {
                        e.stopPropagation();
                        togglePin(item);
                      }}
                      aria-label={pinned ? '取消固定' : '固定'}
                      title={pinned ? '取消固定' : '固定'}
                    >
                      {pinned ? <PinOff size={12} aria-hidden /> : <Pin size={12} aria-hidden />}
                    </button>
                    <button
                      type="button"
                      className="qs__cell-action"
                      onClick={(e) => {
                        e.stopPropagation();
                        void launchItem(item, true);
                      }}
                      aria-label="即开即用"
                      title="即开即用：打开后自动关闭窗口"
                    >
                      <Zap size={12} aria-hidden />
                    </button>
                  </div>
                  <div className="qs__cell-icon" aria-hidden>
                    <Icon size={32} strokeWidth={1.5} />
                  </div>
                  <div className="qs__cell-name" title={item.name}>
                    {item.name}
                  </div>
                </div>
              );
            })}
          </div>
        ) : showNoMatch ? (
          <div className="qs__hint">没有匹配的结果</div>
        ) : showEmptyHint ? (
          <div className="qs__hint">还没有固定任何项。搜索一个结果后用 📌 固定。</div>
        ) : null}
      </div>

      {error && <div className="qs__error">{error}</div>}
    </div>
  );
}
