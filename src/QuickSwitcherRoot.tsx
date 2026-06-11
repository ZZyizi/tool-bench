import { useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { QuickSwitcher } from './components/QuickSwitcher';

function isEscape(e: KeyboardEvent): boolean {
  return e.key === 'Escape' || e.key === 'Esc' || e.keyCode === 27;
}

export function QuickSwitcherRoot() {
  const hideWindow = useCallback(async () => {
    try {
      await getCurrentWebviewWindow().hide();
    } catch (e) {
      console.error('[qs-root] hide failed', e);
    }
  }, []);

  useEffect(() => {
    // Expose diagnostic: in DevTools console, type __hookDiag() to see LL hook state.
    (window as any).__hookDiag = async () => {
      const diag = await invoke('get_hook_diagnostics');
      console.log('[qs-root] LL hook diagnostics:', diag);
      return diag;
    };

    const onKey = (e: KeyboardEvent) => {
      if (isEscape(e)) {
        e.preventDefault();
        e.stopPropagation();
        void hideWindow();
      }
    };
    window.addEventListener('keydown', onKey, { capture: true });
    document.addEventListener('keydown', onKey, { capture: true });
    window.addEventListener('keyup', onKey, { capture: true });
    return () => {
      delete (window as any).__hookDiag;
      window.removeEventListener('keydown', onKey, { capture: true });
      document.removeEventListener('keydown', onKey, { capture: true });
      window.removeEventListener('keyup', onKey, { capture: true });
    };
  }, [hideWindow]);

  return (
    <div className="qs-drag-root">
      <QuickSwitcher />
    </div>
  );
}
