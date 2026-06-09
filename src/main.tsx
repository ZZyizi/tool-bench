import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import App from "./App";
import { ToolWindowRoot } from "./ToolWindowRoot";

const label = getCurrentWebviewWindow().label;
const isToolWindow = label.startsWith("tool-");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {isToolWindow ? <ToolWindowRoot /> : <App />}
  </React.StrictMode>
);
