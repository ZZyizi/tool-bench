import { useEffect, useRef } from 'react';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { QuickSwitcher } from './components/QuickSwitcher';

export function QuickSwitcherRoot() {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    // Pointerdown starts a Tauri-native drag only when the press lands on
    // the chrome (the qs-drag-root or the qs card itself), not on an
    // interactive element. Using Tauri 2's `startDragging()` instead of
    // `-webkit-app-region: drag` is what keeps the drag-aware blur grace
    // in the Rust listener working — the webkit region would route drag
    // through a different path that doesn't emit `Moved` events.
    const onPointerDown = (e: PointerEvent) => {
      const target = e.target as HTMLElement | null;
      if (target && target.closest('.qs__no-drag')) return;
      // Avoid double-handling when the user clicks an element that already
      // captures the pointer for its own logic.
      e.preventDefault();
      void getCurrentWebviewWindow().startDragging();
    };

    el.addEventListener('pointerdown', onPointerDown);
    return () => {
      el.removeEventListener('pointerdown', onPointerDown);
    };
  }, []);

  return (
    <div ref={containerRef} className="qs-drag-root">
      <QuickSwitcher />
    </div>
  );
}
