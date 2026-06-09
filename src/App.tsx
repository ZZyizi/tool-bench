import { useState, useEffect } from 'react';
import { Sidebar } from './components/Sidebar';
import { StatusBar } from './components/StatusBar';
import { Launcher } from './components/Launcher';
import { SettingsPanel } from './components/SettingsPanel';
import { globalRegistry } from './plugins/registry';
import { useSettings } from './settings';
import './plugins/builtin';
import './App.css';

export default function App() {
  const [settings] = useSettings();
  const [settingsOpen, setSettingsOpen] = useState(false);

  return (
    <>
      {settings.mode === 'desktop' ? (
        <Launcher onOpenSettings={() => setSettingsOpen(true)} />
      ) : (
        <EmbeddedApp onOpenSettings={() => setSettingsOpen(true)} />
      )}
      {settingsOpen && <SettingsPanel onClose={() => setSettingsOpen(false)} />}
    </>
  );
}

function EmbeddedApp({ onOpenSettings }: { onOpenSettings: () => void }) {
  const [activeId, setActiveId] = useState<string | null>(null);
  const [renderKey, setRenderKey] = useState(0);

  useEffect(() => {
    const all = globalRegistry.list();
    if (all.length > 0 && activeId === null) {
      setActiveId(all[0].manifest.id);
      setRenderKey((k) => k + 1);
    }
  }, [activeId]);

  const active = activeId ? globalRegistry.get(activeId) : null;
  const ActiveComponent = active?.Component;

  return (
    <div className="app">
      <Sidebar activeId={activeId} onSelect={setActiveId} onOpenSettings={onOpenSettings} />
      <main className="app__main" key={renderKey}>
        {ActiveComponent ? <ActiveComponent /> : <div className="app__empty">选择一个工具开始</div>}
      </main>
      <StatusBar />
    </div>
  );
}
