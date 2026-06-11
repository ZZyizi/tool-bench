# toolBench

> 面向开发者的轻量级桌面工具箱 —— 全局快捷键一拉即出、点完即收,**离线、零配置、可扩展**。
>
> npm 包名：`tool-bench` ｜ Rust crate：`toolBench` ｜ Tauri identifier：`com.toolBench.app`

[![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8D8?logo=tauri)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.8-3178C6?logo=typescript)](https://www.typescriptlang.org/)
[![Vite](https://img.shields.io/badge/Vite-7-646CFF?logo=vite)](https://vitejs.dev/)
[![Rust](https://img.shields.io/badge/Rust-stable-DEA584?logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/平台-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)](#跨平台支持)
[![License](https://img.shields.io/badge/license-待定-lightgrey)](#许可证)

---

## 项目简介

**toolBench** 是一款使用 **Tauri 2 + React 19 + TypeScript 5.8 + Vite 7** 构建的跨平台桌面工具箱。它不是"一个应用"，而是"一个常驻启动器 + 一组独立小工具"：

- 一个**主窗口**（Launcher）展示已安装的内置/三方工具
- 一个**快速启动器**（Quick Switcher）—— 通过 `Ctrl+Space` 任意位置呼出，搜/钉/启动工具
- 每个**工具**在自己的 webview 窗口中运行，按 `Esc` 或失去焦点即可关闭
- 托盘常驻，关闭主窗不退出进程

核心理念：

- **快**：启动 < 1s；快捷键到工具可见 < 200ms（快速启动器窗口已预热）
- **轻**：< 50 MB 内存，单文件运行
- **准**：输出精确，无需解读
- **离线**：完全本地运行，不上传任何数据
- **扩展**：插件化架构，按需添加工具（v0.3+ 支持动态加载）

### 目标用户

- 后端 / 全栈 / Node 开发者（频繁切工具、读环境变量）
- DevOps 工程师（端口/进程/环境变量高频场景）
- 调试本地环境的运维人员

---

## 核心特性

| 特性 | 说明 |
|------|------|
| **全局快速启动器** | `Ctrl+Space` 任意位置呼出，模糊搜索、Pin 常用、即开即用 |
| **多窗口隔离** | 每个工具独立 webview；按 `Esc` 关工具；失去焦点自动收起 |
| **系统托盘** | 关闭主窗默认隐藏到托盘；可改为"真退出"；托盘菜单含显示主窗/快速启动/退出 |
| **可配置全局快捷键** | 通过设置面板录制新快捷键（仅 `Ctrl+Space` 为默认推荐） |
| **应用扫描** | Windows：递归扫描开始菜单 `.lnk`，用 PowerShell 解析 target/icon；Unix：扫 `*.desktop` 兜底 |
| **环境变量编辑** | 读/写/删 user 与 system 作用域；PATH 单独编辑；Java / Python / Node / Go / Rust 一键检测并配置 |
| **端口管理** | 端口列表、按 PID 释放、按进程名批量结束 |
| **设置持久化** | `app_config_dir/settings.json`（原子写：`.json.tmp` → rename） |

### 内置工具

| 工具 id | 名称 | 分类 | 主要能力 |
|---------|------|------|----------|
| `port-manager` | 端口管理 | Network | 端口列表、按端口号释放、按进程名批量结束 |
| `env-editor` | 环境变量 | System | 查看/新增/修改/删除 user & system 环境变量、PATH 编辑、Java/Python/Node/Go/Rust 一键配置 |

> **添加一个新工具** = 在 `src/plugins/builtin/<id>/` 下放 `index.ts` + `<Name>View.tsx`，然后在 `src/plugins/builtin/index.ts` 中**加一行**注册。**无需改动** `App.tsx` / `Launcher` / `QuickSwitcher` —— 它们都是从 `globalRegistry` 数据驱动渲染的。

---

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面壳 | Tauri 2（Rust） |
| 前端框架 | React 19 + TypeScript 5.8 |
| 构建工具 | Vite 7 |
| 跨平台 | Windows / macOS / Linux（**Windows 体验最完整**：含 PowerShell 解析 `.lnk`、WH_KEYBOARD_LL 钩子、Alt+Space 系统菜单吞咽） |
| 状态管理 | React `useState` + `useSettings` Hook（设置走 Rust 持久化） |
| 样式 | 原生 CSS + CSS Variables（深色主题） |
| 图标 | `lucide-react` |
| IPC | `@tauri-apps/api` 的 `invoke()` |
| Rust 生态 | `tauri`、`tauri-plugin-opener`、`tauri-plugin-dialog`、`tauri-plugin-global-shortcut`、`serde`、`serde_json`、`thiserror` |

> **依赖原则**：不引入 `sysinfo` / `netstat2` 等第三方 Rust crate —— 平台层直接解析 `netstat` / `lsof` 的 CLI 输出。Windows 应用扫描走 `WScript.Shell`（PowerShell 解析 `.lnk`），不依赖 `windows` crate 的非稳定 API。

---

## 快速开始

### 前置要求

- **Node.js** ≥ 18
- **Rust** stable（[rustup](https://rustup.rs)）
- 平台构建工具：
  - **Windows**：Microsoft Visual Studio C++ Build Tools + WebView2
  - **macOS**：Xcode Command Line Tools
  - **Linux**：`webkit2gtk-4.1`、`libayatana-appindicator3-dev` 等（参考 [Tauri 文档](https://tauri.app/start/prerequisites/)）

### 安装与运行

```bash
# 1. 安装前端依赖
npm install

# 2. 启动开发模式（同时启动 Vite + Tauri 原生窗口）
npm run tauri dev
```

首次启动会比较慢（Tauri 需要编译 Rust 依赖），后续启动约 10 秒。

### 仅前端开发（不打开原生窗口）

```bash
npm run dev          # 启动 Vite 开发服务器（端口 1420）
```

> Vite 端口固定为 `1420`（`strictPort: true`），Tauri 期望这个端口，**不要修改**。HMR 走 `1421`（`TAURI_DEV_HOST` 设置时启用）。

### 构建生产包

```bash
npm run build        # tsc + vite build（输出到 dist/）
npm run tauri build  # 打包成各平台原生安装包
```

---

## 开发命令速查

### 前端（项目根目录）

| 命令 | 说明 |
|------|------|
| `npm run dev` | Vite 开发服务器 |
| `npm run build` | 类型检查 + 生产构建 |
| `npm run preview` | 预览构建产物 |
| `npx tsc --noEmit` | 仅类型检查 |

### Tauri / Rust（`src-tauri/` 目录）

| 命令 | 说明 |
|------|------|
| `npm run tauri` | 等价于 `tauri dev` |
| `cargo check` | 快速类型检查 |
| `cargo test` | 运行所有 Rust 单元测试 |
| `cargo test --lib platform::windows::tests` | 运行单个测试模块 |
| `cargo test --lib cmd::apps` | 应用扫描单元测试（`stable_id_for` 等） |

---

## 项目架构

toolBench 采用 **两层职责分离 + 多窗口协议** 的设计 —— **Rust 只做"系统能力提供者"**，**所有 Tool/Plugin 逻辑都在前端**；前端又把"启动器 UI"和"工具 UI"分到不同的 webview 窗口里，靠 URL 参数和 webview label 协议路由。

```
┌────────────────────────────────────────────────────────────────────────────┐
│  Frontend (React, src/)                                                    │
│                                                                            │
│   main.tsx (URL 路由)                                                      │
│     ├── webview label "main"      → App.tsx → Launcher / EmbeddedApp       │
│     ├── webview label "quick-..." → QuickSwitcherRoot → QuickSwitcher     │
│     └── webview label "tool-..."  → ToolWindowRoot   → plugin.Component   │
│                                                                            │
│   公共能力                                                                  │
│     ├── Plugin System: Plugin / PluginManifest / PluginRegistry /          │
│     │                 PluginContext (with globalRegistry)                  │
│     ├── System API Bridge → api.ts (类型化的 invoke 包装)                  │
│     ├── useSettings() Hook → get/set settings + 跨实例广播                 │
│     └── 平台桥 (Windows): 通过 windows_hook 吞 Alt+Space/Esc              │
└────────────────────────────┬───────────────────────────────────────────────┘
                             │ IPC
┌────────────────────────────┴───────────────────────────────────────────────┐
│  Backend (Rust, src-tauri/src/)                                            │
│                                                                            │
│   lib.rs — Tauri builder、AppState、托盘、关窗 handler、注册所有命令       │
│   ├── cmd/                                                                 │
│   │   ├── ports.rs        list_ports / kill_port / kill_by_process_name   │
│   │   ├── env.rs          list_env / set_var / delete_var / set_path_...   │
│   │   │                  detect_preset / apply_preset                     │
│   │   ├── apps.rs         list_installed_apps / launch_app                │
│   │   │                  (Windows: .lnk + PowerShell 解析)                │
│   │   ├── quick_switcher.rs  open_quick_switcher                          │
│   │   ├── settings.rs     get_settings / set_settings / set_recording_mode│
│   │   ├── windows.rs      open_tool_window / close_tool_window            │
│   │   └── capabilities.rs list_capabilities                               │
│   ├── platform/        PortScanner trait + Windows/Unix 实现              │
│   └── windows_hook.rs  WH_KEYBOARD_LL 钩子 + 窗口子类化（仅 Windows）     │
└────────────────────────────────────────────────────────────────────────────┘
```

**关键设计点**：

1. **Rust 不定义 Tool 抽象**。所有"工具"都是前端的 React 组件。Rust 提供原子化的系统能力（`list_ports`、`set_var` 等）。
2. **每个工具独立 webview**（`tool-<id>` 标签，URL `?plugin=<id>`）。崩溃隔离，状态隔离。
3. **快速启动器预热**：`lib.rs::setup()` 启动时即预创建 `quick-switcher` 窗口（隐藏），第一次 `Ctrl+Space` 几乎零延迟。
4. **设置是单例**：`useSettings()` Hook 内部去后端 `get_settings` 拉初始值，`setter` 走乐观更新 + `devtoolkit-settings-changed` 自定义事件广播给同进程的其它实例，再 fire-and-forget 持久化。
5. **Windows 钩子只在 Windows 编译**：`#[cfg(windows)]` 的 `windows_hook.rs` 用 `WH_KEYBOARD_LL` 吞 Alt+Space 系统菜单、Esc 关快速启动器；其它平台用普通事件。

### 目录结构

```
.
├── src/                                # 前端 (React + TS)
│   ├── main.tsx                        # 入口：根据 webview label 选 Root
│   ├── App.tsx                         # 主窗 Root（按 mode 选 Launcher / EmbeddedApp）
│   ├── App.css                         # CSS Variables + 布局
│   ├── QuickSwitcherRoot.tsx           # 快速启动器窗 Root
│   ├── ToolWindowRoot.tsx              # 工具窗 Root（按 ?plugin= 选组件）
│   ├── ToolWindowRoot.css
│   ├── Launcher.tsx / .css             # 主窗启动器 UI（默认 mode）
│   ├── settings.ts                     # AppSettings 类型 + useSettings() Hook
│   ├── types.ts                        # 共享 TS 类型（PortInfo、EnvSnapshot、PresetPlan...）
│   ├── components/
│   │   ├── Sidebar.tsx / .css          # embedded 模式侧栏
│   │   ├── StatusBar.tsx               # embedded 模式底栏
│   │   ├── ConfirmDialog.tsx / .css    # 二次确认弹窗（端口释放等）
│   │   ├── QuickSwitcher.tsx / .css    # 全局快速启动器
│   │   └── SettingsPanel.tsx / .css    # 设置弹窗（含 ShortcutRecorder）
│   └── plugins/
│       ├── types.ts                    # Plugin / PluginManifest / PluginContext
│       ├── registry.ts                 # PluginRegistry + globalRegistry
│       ├── context.ts                  # createPluginContext()
│       ├── api.ts                      # 类型化的 invoke 包装
│       └── builtin/
│           ├── index.ts                # 注册所有内置插件（按 id 静态导入）
│           ├── port-manager/           # 端口管理工具
│           │   ├── index.ts
│           │   ├── PortView.tsx
│           │   └── PortView.css
│           └── env-editor/             # 环境变量编辑器
│               ├── index.ts
│               ├── EnvEditorView.tsx
│               └── EnvEditorView.css
│
├── src-tauri/                          # 后端 (Rust)
│   └── src/
│       ├── main.rs                     # 调用 tool_bench_lib::run()
│       ├── lib.rs                      # Tauri builder + AppState + 托盘 + 关窗
│       ├── windows_hook.rs             # WH_KEYBOARD_LL 钩子 + 窗口子类化（Windows）
│       ├── cmd/
│       │   ├── mod.rs
│       │   ├── ports.rs                # list_ports / kill_port / kill_by_process_name
│       │   ├── env.rs                  # list_env / set_var / delete_var / set_path_entries
│       │   │                           # detect_preset / apply_preset
│       │   ├── apps.rs                 # list_installed_apps / launch_app
│       │   ├── quick_switcher.rs       # open_quick_switcher（+ 预创建 + 居中 + Esc 收起）
│       │   ├── settings.rs             # get_settings / set_settings / set_recording_mode
│       │   ├── windows.rs              # open_tool_window / close_tool_window
│       │   └── capabilities.rs         # list_capabilities
│       └── platform/
│           ├── mod.rs                  # 工厂方法
│           ├── port_scanner.rs         # PortScanner trait / PortInfo / PortError
│           ├── windows.rs              # netstat -ano 解析 + 测试（#[cfg(windows)]）
│           └── unix.rs                 # lsof -i 解析 + 测试（#[cfg(unix)]）
│
├── docs/
│   ├── PRD.md                          # 产品需求文档
│   ├── tools-planning.md               # 工具候选清单
│   └── superpowers/
│       ├── specs/                      # 设计文档
│       └── plans/                      # 任务分解与进度跟踪
│
├── package.json                        # npm 包（tool-bench）
├── vite.config.ts                      # Vite 配置（端口 1420，HMR 1421）
├── tauri.conf.json                     # Tauri 应用配置
├── PRD.md
├── CLAUDE.md                           # Claude Code 工作指引
└── README.md
```

---

## 暴露给前端的 Tauri 命令

所有命令在 IPC 边界统一为 `Result<T, String>`（stringified error）；内部错误由 `thiserror` 定义（如 `PortError` / `AppsError` / `EnvError`）。

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `list_ports` | `query: String` | `FilteredPorts` | 平台工厂执行 `netstat`/`lsof`；`PermissionDenied` 透传 |
| `kill_port` | `port: u16` | `KillResult` | UI 二次确认；后端 `taskkill /F` 或 `kill -9` |
| `kill_by_process_name` | `name: String` | `KillByNameResult` | 按名批量结束所有匹配 PID（v0.2+） |
| `list_capabilities` | — | `Capabilities` | 后端能力声明（V0.3+ 用于权限校验） |
| `list_installed_apps` | — | `InstalledApps` | Windows 扫开始菜单 `.lnk` + PowerShell 解析；Unix 扫 `.desktop` |
| `launch_app` | `target: String` | `void` | `cmd /C start "" "<target>"`（detach） |
| `open_quick_switcher` | — | `void` | 切换或创建快速启动器窗口；居中屏幕 |
| `open_tool_window` | `pluginId, title?, width?, height?, useAndGo?` | `void` | 创建/聚焦 `tool-<id>` webview 窗口 |
| `close_tool_window` | `pluginId` | `void` | 关闭指定工具窗 |
| `get_settings` | — | `AppSettings` | 从 `app_config_dir/settings.json` 读取（缓存） |
| `set_settings` | `AppSettings` | `AppSettings` | 原子写 + 同步 `close_behavior` + 重注册全局快捷键 |
| `set_recording_mode` | `recording: bool` | `void` | 录制时关闭钩子吞咽 + 注销旧快捷键；结束时恢复 |
| `list_env` | — | `EnvSnapshot` | user + system 环境变量 + 解析后的 PATH 列表 + warnings |
| `set_var` (Tauri 名 `set_var_cmd`) | `scope, name, value` | `void` | 写单条变量 |
| `delete_var` (Tauri 名 `delete_var_cmd`) | `scope, name` | `void` | 删除变量 |
| `set_path_entries` (Tauri 名 `set_path_entries_cmd`) | `scope, entries[]` | `void` | 整体替换 PATH 列表 |
| `detect_preset` (Tauri 名 `detect_preset_cmd`) | `kind, dir` | `PresetResult` | 检测 Java/Python/Node/Go/Rust 安装目录，生成 plan |
| `apply_preset` (Tauri 名 `apply_preset_cmd`) | `PresetPlan` | `ApplyResult` | 应用 plan：写变量 + 改 PATH |
| `get_hook_diagnostics` | — | JSON | Windows 钩子诊断计数（hits / esc / suppress / QS hwnd） |

---

## 添加新工具（插件）

1. 在 [src/plugins/builtin/](src/plugins/builtin/) 下创建新目录，例如 `<id>/`
2. 准备以下文件：
   - `index.ts` —— 导出 `Plugin` 对象（manifest + Component + activate）
   - `<Name>View.tsx` + `<Name>View.css` —— 工具视图
   - 可选：自定义窗口尺寸 `manifest.windowWidth/Height`
3. 在 [src/plugins/builtin/index.ts](src/plugins/builtin/index.ts) 中 **加一行** `globalRegistry.register(...)` 并触发 `activate`
4. 如果插件需要新的后端能力：
   - 在 `src-tauri/src/cmd/<name>.rs` 添加命令
   - 在 [src-tauri/capabilities/default.json](src-tauri/capabilities/default.json) 注册前端权限
5. 重新启动 `npm run tauri dev`

> **无需** 改动 `App.tsx` / `Launcher.tsx` / `QuickSwitcher.tsx` —— 它们都是 `globalRegistry.list()` 数据驱动的。
>
> 验收：以上 5 步做完后，启动应用，新工具自动出现在主窗 Launcher 与快速启动器中。

### 工具 ID 命名空间

- 内置工具：`tool:<id>` —— `<id>` 即 `manifest.id`
- 安装的应用：`app:<fnv64-of-normalized-path>` —— FNV-1a 64 对归一化（小写 + 正斜杠）的 `.lnk`/`.desktop` 路径哈希，跨路径分隔符稳定
- 这两类 id 都可以在快速启动器中**搜索/Pin**

---

## 多窗口 / 快速启动器协议

```
┌──────────────────┐                          ┌──────────────────┐
│  main 窗         │  onClick                  │  tool-<id> 窗   │
│  (Launcher)      │  invoke('open_tool_window',│  ToolWindowRoot  │
│  webview label=  │   { pluginId, ... })      │  URL: ?plugin=  │
│  "main"          │ ────────────────────────▶ │  Esc 关闭        │
└──────────────────┘                          └──────────────────┘
         │                                              ▲
         │ invoke('open_quick_switcher')                 │
         ▼                                              │
┌──────────────────┐    按 Pin/Enter 后                  │
│  quick-switcher  │    invoke('open_tool_window', ...)  │
│  webview label=  │ ───────────────────────────────────┘
│  "quick-..."     │
│  precreated 预热 │
│  Esc / blur 收起 │
└──────────────────┘
```

- **URL 路由**：[src/main.tsx](src/main.tsx) 读 `getCurrentWebviewWindow().label` 和 `?window=` / `?plugin=` 决定渲染哪个 Root
- **预热**：[lib.rs::setup()](src-tauri/src/lib.rs) 启动时即 `quick_switcher::precreate(...)`，webview 预渲染好
- **Esc 关闭**：QS 自身 + ToolWindowRoot 都监听 capture-phase `keydown`，按 Esc 隐藏/关闭
- **use-and-go**：打开工具时 `useAndGo=true` 注册一个 `WindowEvent::Focused(false)` 监听器；200ms grace 后若未重新获焦则自动关工具窗（适用于"打开就用一下就关"的场景）

---

## 跨平台支持

| 平台 | 端口查询 | 进程终止 | 应用扫描 | 环境变量 | 全局快捷键 | 钩子 |
|------|----------|----------|----------|----------|------------|------|
| **Windows** | `chcp 65001 && netstat -ano`（带 `-b` 解析 `[exe.exe]` trailer） | `taskkill /F /PID xxx` | 开始菜单 `.lnk` + PowerShell `WScript.Shell` | 注册表 / `SetX` | ✅ | `WH_KEYBOARD_LL` + 窗口子类化（吞 Alt+Space） |
| **macOS** | `lsof -i -P -n` | `kill -9 PID` | `*.desktop` 兜底（不完整） | `launchctl setenv` 等 | ✅ | ❌ |
| **Linux** | `lsof -i -P -n` | `kill -9 PID` | `*.desktop` 兜底（不完整） | `~/.profile` / `/etc/environment` | ✅ | ❌ |

**权限错误识别**（在 stderr 上做子串匹配）：

- Windows：`"Access is denied"` → `PortError::PermissionDenied`
- Unix：`"Operation not permitted"` → `PortError::PermissionDenied`

> 注意：`PortInfo.process_name` 是 `Option<String>`。Windows `netstat` 在非管理员下大多返回 `None`，UI 在此场景下只显示 PID；高权限运行（管理员）时 `-b` 标志会附加 `[process.exe]` trailer，由 `parse_netstat` 回填。

### Windows 平台细节

- **`WH_KEYBOARD_LL` 钩子**（[windows_hook.rs](src-tauri/src/windows_hook.rs)）：安装到主线程，`Alt+Space` 触发的系统菜单（`SC_KEYMENU`）被吞掉，保留给用户自定义快捷键。`Esc` 在快速启动器可见时由钩子直接发 `WM_CLOSE`，避免与 webview 焦点状态竞争。
- **窗口子类化**：每个 webview 窗口（主窗、QS、工具窗）通过 `SetWindowLongPtrW` 替换 `WNDPROC`，在子类回调里继续吞 `WM_SYSCOMMAND` / `SC_KEYMENU`。幂等（用 `GetPropW` 标记）。
- **PowerShell 解析 `.lnk`**：走 `powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File <script.ps1> <paths...>`（不是 `-Command`），避免路径里的 `$` / 括号被当成代码段。脚本写到 `temp_dir/devtoolkit-resolve-<pid>.ps1`，用 pid 区分并发实例。

---

## 关键约定

- **数据契约**：所有 Tauri 命令返回的 `T` 必须是 `serde::Serialize` 的；前端 `src/types.ts` 是事实上的类型契约，新增/修改字段必须同步两侧。
- **`process_name` 是 `Option<String>`**：Windows netstat 不保证能拿到；UI 必须容忍 `null`。
- **`chcp 65001 > nul` 不可省**：Windows 控制台默认编码（非英文系统下）会破坏 netstat `-b` 的 `[exe]` 行解析。
- **设置原子写**：`settings.json.tmp` → `rename`。崩溃中段不会留下半截 JSON。
- **全局快捷键变更**：必须先 `unregister_all()` 再 `on_shortcut(new, ...)`，避免重复触发。
- **快捷键录制期间**（`set_recording_mode(true)`）：钩子 `SUPPRESS=false` 让 Alt+Space 等键能透传到 webview 让用户录制；结束时 `SUPPRESS=true` 并重新应用保存的快捷键。
- **没有第三方 Rust crate** 用于"枚举系统信息"——按设计原则解析 CLI / `WScript.Shell` 输出。

---

## 测试策略

- **Rust 单元测试** 覆盖：
  - `parse_netstat` / `parse_lsof`：4+ 个用例覆盖 LISTEN/ESTABLISHED/UDP/无效行/去重
  - `kill_port`：边界场景（端口无占用 / 权限拒绝 / 多个 PID）
  - `cmd/apps`：`stable_id_for` 对路径分隔符大小写不敏感；不同路径得到不同 id
  - `cmd/settings`：默认值、JSON 序列化往返
- **手动验证** 是验证 IPC + UI 完整流程的唯一方式：
  - **端口**：`python -m http.server 8765` 启动一个测试服务，运行应用，验证 8765 出现在列表中
  - **环境变量**：用 env-editor 添加一条测试变量，重新打开"新 cmd"验证生效
  - **快速启动器**：`Ctrl+Space` 任意位置呼出，输入工具名 / Pin / Enter 打开
  - **托盘**：关闭主窗 → 托盘图标仍在 → 右键 → 退出
- **不引入** 前端测试框架（Vitest / RTL 等）—— 路线图外。

---

## 路线图

| 版本 | 范围 | 状态 |
|------|------|------|
| **v0.1** | 端口列表 / 端口释放 / 基础 UI / 插件基础设施（port-manager） | ✅ 已完成 |
| **v0.2** | 快速启动器 / 工具矩阵 / 设置 / 托盘 / Windows 钩子 / env-editor / kill_by_process_name | ✅ 已完成（当前 main） |
| **v0.3** | 文件系统动态加载插件 / `manifest.capabilities` 权限强制 / 工具市场本地浏览 | 📋 规划中 |
| **v0.4** | 远程插件市场 / 安装 / 签名校验 | 📋 规划中 |
| **v1.0** | 打包发布 / 性能优化 / 全平台 polish | 📋 规划中 |

未来内置工具候选：[docs/tools-planning.md](docs/tools-planning.md) 列出了进程查看器、系统概览、DNS 解析、Hosts 管理、文件差异对比等。

详细设计：[docs/superpowers/specs/](docs/superpowers/specs/) ｜ 计划：[docs/superpowers/plans/](docs/superpowers/plans/) ｜ PRD：[docs/PRD.md](docs/PRD.md)

---

## 贡献

欢迎 PR！但请先阅读：

1. [CLAUDE.md](CLAUDE.md) —— 项目约定、命名规范、目录结构、命令清单
2. [docs/PRD.md](docs/PRD.md) —— 产品定位与功能边界
3. 相关版本的设计文档（[docs/superpowers/specs/](docs/superpowers/specs/)）与执行计划（[docs/superpowers/plans/](docs/superpowers/plans/)）

### 当前不在范围内（暂不接受）

- 工具之间共享数据
- 工具访问互联网（保持离线）
- 主题切换（v0.1 假设深色主题为唯一主题）
- Linux/macOS 的 `.desktop` 应用扫描完整性（v0.2 兜底即可；完整解析是 v0.3+）
- 打包发布（v1.0）

---

## 资源链接

- **产品文档**：[docs/PRD.md](docs/PRD.md)
- **工具候选清单**：[docs/tools-planning.md](docs/tools-planning.md)
- **v0.1 设计**：[docs/superpowers/specs/2026-06-05-devtoolkit-v0.1-design.md](docs/superpowers/specs/2026-06-05-devtoolkit-v0.1-design.md)
- **v0.1 计划**：[docs/superpowers/plans/2026-06-05-devtoolkit-v0.1.md](docs/superpowers/plans/2026-06-05-devtoolkit-v0.1.md)
- **Tauri 2 文档**：https://tauri.app/develop/
