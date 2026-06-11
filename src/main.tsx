import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import "./plugins/builtin";
import { loadUserPlugins } from "./plugins/userLoader";
import App from "./App";
import { ToolWindowRoot } from "./ToolWindowRoot";
import { QuickSwitcherRoot } from "./QuickSwitcherRoot";

const label = getCurrentWebviewWindow().label;
const params = new URLSearchParams(window.location.search);
const windowParam = params.get("window");
const isQuickSwitcher =
  label === "quick-switcher" || windowParam === "quick-switcher";
const isToolWindow = label.startsWith("tool-");

// Block first render until the user-plugins scan completes — by the time
// any root component reads globalRegistry, both built-in and user plugins
// are registered. Scan is local-fs only, typically <100ms.
await loadUserPlugins();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {isQuickSwitcher ? (
      <QuickSwitcherRoot />
    ) : isToolWindow ? (
      <ToolWindowRoot />
    ) : (
      <App />
    )}
  </React.StrictMode>
);
