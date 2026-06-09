import { useEffect, useRef } from 'react';
import { QuickSwitcher } from './components/QuickSwitcher';

export function QuickSwitcherRoot() {
  const containerRef = useRef<HTMLDivElement>(null);

  // Tauri honors `-webkit-app-region: drag` on frameless windows. Apply it
  // to the chrome (everywhere except interactive controls) by walking the
  // descendants of the QuickSwitcher: the input and the action buttons
  // opt out via the `qs__no-drag` class.
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    el.classList.add('qs-drag-root');
    const stops = el.querySelectorAll<HTMLElement>(
      'input, button, .qs__item--active, .qs__pin, .qs__clear, .qs__hint',
    );
    stops.forEach((node) => node.classList.add('qs__no-drag'));
  }, []);

  return (
    <div ref={containerRef} className="qs-drag-root">
      <QuickSwitcher />
    </div>
  );
}
