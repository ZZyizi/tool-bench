# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> **全局使用中文回答。**

---

## Project: toolBench

A lightweight **developer desktop toolbox** built on **Tauri 2 + React 19 + TypeScript 5.8 + Vite 7**. Targets Windows / macOS / Linux, < 1s startup, < 50 MB RAM, fully offline. **v0.1 已完成**,v0.2 的快速启动器 / 工具矩阵 / 设置 / 托盘 / Windows 钩子 / env-editor / 批量杀进程也已落地。当前在向 v0.3（动态加载插件 / 权限强制）演进。

### 命名规范（这些名字在很多地方被引用，**必须保持一致**）

- npm 包：`tool-bench`（[package.json](package.json)）
- Rust crate：`toolBench`（[src-tauri/Cargo.toml](src-tauri/Cargo.toml)）
- Rust lib：`tool_bench_lib`（[src-tauri/src/main.rs](src-tauri/src/main.rs) 调用 `tool_bench_lib::run()`）
- Tauri identifier：`com.toolBench.app`（[src-tauri/tauri.conf.json](src-tauri/tauri.conf.json)）
- 显示名 / 托盘 tooltip：`toolBench`

### 关键决策的 Why（首次修改前必读）

| 决策 | Why |
|------|-----|
| Rust 不定义 Tool 抽象 | 第三方插件作者只写 JS/TS；Rust 是"原子系统能力提供者"，命令边界即 API 边界 |
| 每个工具独立 webview 窗口 | 崩溃隔离 + 状态隔离 + 工具切换零成本（独立进程内 view） |
| 快速启动器启动时预创建（隐藏） | 第一次 `Ctrl+Space` 零延迟；预渲染不浪费内存（仅占一个空 webview） |
| 设置走 Rust 持久化（不是 `localStorage`） | 跨平台一致；可在 Rust 侧直接读/改（关窗 handler / 钩子行为都依赖它） |
| 设置用乐观更新 + 自定义事件广播 | 多窗口同时挂载 `useSettings()` 时不需要 polling；fire-and-forget 持久化避免阻塞 UI |
| 不引入 `sysinfo` / `netstat2` 等 Rust crate | 减少编译时间、避免 windows 平台的非稳定 API 依赖；CLI 输出解析已经够用 |
| 不用 Redux/Zustand | useState + 组件分层已足够；引入状态库是 v0.4+ 才考虑的事 |
| 不引入前端测试框架 | v0.2 仍在 UI 快速迭代期；写测试是 v0.3+ 才稳的 |

---

## Active Milestone

**v0.2（已完成）→ v0.3 规划中**

- v0.1 计划（[docs/superpowers/plans/2026-06-05-devtoolkit-v0.1.md](docs/superpowers/plans/2026-06-05-devtoolkit-v0.1.md)）所有 19 个任务都跑通
- v0.2 增量：Launcher / Quick Switcher / Settings / Tray / Windows hook / env-editor / kill_by_process_name / app scanner
- v0.3：文件系统动态加载插件 + `manifest.capabilities` 权限强制
- v0.4：远程插件市场 / 签名校验
- v1.0：打包发布 / 全平台 polish

修改前先看 [docs/PRD.md](docs/PRD.md) 和 [docs/tools-planning.md](docs/tools-planning.md) 确认产品定位。架构决策在 [docs/superpowers/specs/](docs/superpowers/specs/) 下的设计文档里。

---

## 开发命令

所有前端命令从**项目根**运行；Rust 命令从 `src-tauri/` 运行。

### 前端

- `npm run dev` — Vite dev server on port 1420（Tauri 固定，**不要改**）
- `npm run build` — `tsc && vite build`
- `npm run preview` — 预览构建产物
- `npx tsc --noEmit` — 仅类型检查

### Tauri / Rust

- `npm run tauri` — `tauri dev`（同时启动 Vite + 原生窗口，首次 ~10s）
- `cd src-tauri && cargo check` — 快速类型检查
- `cd src-tauri && cargo test` — 全部 Rust 单元测试
- `cd src-tauri && cargo test --lib platform::windows::tests` — 单个测试模块
- `cd src-tauri && cargo test --lib cmd::apps` — 应用扫描测试

### 手动验证

```bash
# 1. 起一个测试端口
python -m http.server 8765

# 2. 启动应用：主窗 + Ctrl+Space 唤出快速启动器 + 端口管理工具窗
npm run tauri dev
```

完整检查清单：① 主窗 Launcher 出现 ② 托盘图标出现 ③ `Ctrl+Space` 唤出快速启动器 ④ 输入"端口" / 选 port-manager ⑤ 工具窗出现，8765 在列表中 ⑥ Esc 关闭工具 ⑦ 关闭主窗 → 托盘菜单 → 显示主窗 仍可恢复。

---

## 架构：双层职责 + 多窗口协议

### 两层职责（v0.1 设计，未变）

```
┌──────────────────────────────────────────────────────────┐
│  Frontend (React, src/)                                  │
│    ├── Plugin System: Plugin / PluginManifest /         │
│    │                   PluginRegistry / PluginContext   │
│    ├── System API Bridge → api.ts (类型化 invoke 包装)   │
│    ├── useSettings() Hook → 跨实例广播 + 后端持久化      │
│    └── Windows-only bridge: WH_KEYBOARD_LL 钩子          │
└──────────────────────────┬───────────────────────────────┘
                           │ IPC
┌──────────────────────────┴───────────────────────────────┐
│  Backend (Rust, src-tauri/src/)                          │
│    ├── lib.rs — Tauri builder / AppState / 托盘 / 关窗    │
│    ├── cmd/         7 个命令模块（见下表）                │
│    ├── platform/    PortScanner trait + Win/Unix 解析器  │
│    └── windows_hook.rs  Win-only LL 钩子 + 窗口子类化    │
└──────────────────────────────────────────────────────────┘
```

**核心原则**：Rust 是"原子系统能力"——一次只做一件事（`list_ports`、`set_var`）。前端用这些能力组合出"工具"。

### 多窗口协议（v0.2 引入）

应用同时跑多个 webview，由 [src/main.tsx](src/main.tsx) 的 URL 路由分发：

| 窗口 label | URL 路径 | Root 组件 | 作用 |
|------------|----------|-----------|------|
| `main` | `index.html` | `App` | 主窗，按 `settings.mode` 选 Launcher / EmbeddedApp |
| `quick-switcher` | `?window=quick-switcher` | `QuickSwitcherRoot` | 全局快捷键唤出，浮窗 |
| `tool-<id>` | `?plugin=<id>` | `ToolWindowRoot` | 工具窗，按 `?plugin=` 选组件 |

**预热**：[src-tauri/src/lib.rs](src-tauri/src/lib.rs) 启动时调用 `quick_switcher::precreate(app)`，webview 已创建好但隐藏，第一次 `Ctrl+Space` 零延迟。

**Esc 关闭**：QS 自身 + ToolWindowRoot 都注册 capture-phase `keydown` 监听，Esc 触发隐藏/关闭。Windows 平台额外有 `WH_KEYBOARD_LL` 钩子兜底（见下）。

**use-and-go**：[cmd/windows.rs](src-tauri/src/cmd/windows.rs) 在 `use_and_go=true` 时为工具窗注册 `WindowEvent::Focused(false)` 监听，250ms grace 后未重新获焦就关窗。配合快速启动器的"即开即用"按钮。

---

## 实际文件结构（以磁盘为准）

```
src/
├── main.tsx                            # URL 路由：label/?window=/?plugin= → Root
├── App.tsx                             # 主窗 Root；按 mode 选 Launcher / EmbeddedApp
├── App.css
├── QuickSwitcherRoot.tsx               # 快速启动器窗 Root（薄壳，包 <QuickSwitcher/>）
├── ToolWindowRoot.tsx                  # 工具窗 Root；按 ?plugin= 选 plugin.Component
├── ToolWindowRoot.css
├── Launcher.tsx / .css                 # 主窗启动器 UI（默认 mode）
├── settings.ts                         # AppSettings 类型 + useSettings() Hook
├── types.ts                            # 共享 TS 类型（PortInfo / EnvSnapshot / PresetPlan / ...）
├── components/
│   ├── Sidebar.tsx / .css              # embedded 模式侧栏（mode='embedded' 时启用）
│   ├── StatusBar.tsx                   # embedded 模式底栏
│   ├── ConfirmDialog.tsx / .css        # 二次确认弹窗
│   ├── QuickSwitcher.tsx / .css        # 全局快速启动器（含 Pin / use-and-go）
│   └── SettingsPanel.tsx / .css        # 设置弹窗（含 ShortcutRecorder）
└── plugins/
    ├── types.ts                        # Plugin / PluginManifest / PluginContext
    ├── registry.ts                     # PluginRegistry + globalRegistry
    ├── context.ts                      # createPluginContext()（当前未深度使用）
    ├── api.ts                          # api = 类型化 invoke 包装
    └── builtin/
        ├── index.ts                    # 静态导入并 register 所有内置插件
        ├── port-manager/               # 端口管理（v0.1）
        │   ├── index.ts                # manifest（id/name/version/...）
        │   ├── PortView.tsx            # 视图
        │   └── PortView.css
        └── env-editor/                 # 环境变量（v0.2）
            ├── index.ts
            ├── EnvEditorView.tsx
            └── EnvEditorView.css

src-tauri/src/
├── main.rs                             # 调用 tool_bench_lib::run()
├── lib.rs                              # Tauri builder + AppState + 托盘 + 关窗
├── windows_hook.rs                     # #[cfg(windows)] WH_KEYBOARD_LL 钩子 + 窗口子类化
├── cmd/
│   ├── mod.rs
│   ├── ports.rs                        # list_ports / kill_port / kill_by_process_name
│   ├── env.rs                          # list_env / set_var_cmd / delete_var_cmd
│   │                                   # set_path_entries_cmd / detect_preset_cmd / apply_preset_cmd
│   ├── apps.rs                         # list_installed_apps / launch_app（Win: .lnk + PowerShell）
│   ├── quick_switcher.rs               # open_quick_switcher（+ 预创建 + 居中 + 收起时序）
│   ├── settings.rs                     # get_settings / set_settings / set_recording_mode
│   ├── windows.rs                      # open_tool_window / close_tool_window
│   └── capabilities.rs                 # list_capabilities
└── platform/
    ├── mod.rs                          # 工厂：create_scanner()
    ├── port_scanner.rs                 # PortScanner trait / PortInfo / PortError
    ├── windows.rs                      # netstat -anob 解析（带 -b trailer 拼接）
    └── unix.rs                         # lsof -i -P -n 解析
```

---

## Plugin System 实战规范

[src/plugins/types.ts](src/plugins/types.ts)：

```typescript
interface PluginManifest {
  id: string;            // 全局唯一；用于 window label (`tool-${id}`) 和 Pin id (`tool:${id}`)
  name: string;          // 显示名（中文友好）
  version: string;       // 语义化版本
  description: string;
  author: string;
  category: string;      // 自由分类：Network / System / Encode / Other...
  icon?: ComponentType;  // lucide-react 图标组件（不是字符串）
  entry: string;         // 静态导入用，相对路径
  capabilities?: string[];
  windowWidth?: number;  // 工具窗默认尺寸
  windowHeight?: number;
}

interface Plugin {
  manifest: PluginManifest;
  Component?: ComponentType;          // 工具窗挂载的视图组件
  activate: (ctx: PluginContext) => void | Promise<void>;
  deactivate?: () => void | Promise<void>;
}
```

**v0.1–v0.2**：插件**静态导入**（在 `src/plugins/builtin/index.ts` 中加一行 `globalRegistry.register(xxx)`）。**v0.3+**：改为扫描 `~/.tool-bench/plugins/`，解析 `plugin.toml` 动态加载。

**添加新工具的最小步骤**（这是插件基础设施的验收标准）：

1. 在 `src/plugins/builtin/<id>/` 下创建 `index.ts`（manifest + Component）+ `<Name>View.tsx`（视图）+ 可选 `<Name>View.css`
2. 在 `src/plugins/builtin/index.ts` 顶部 `import`，底部 `globalRegistry.register(...)` + `activate(builtinContext)`
3. 如果需要新后端能力：在 `src-tauri/src/cmd/<name>.rs` 加命令，在 [src-tauri/capabilities/default.json](src-tauri/capabilities/default.json) 加权限
4. 改完直接重启 `npm run tauri dev` 验证

**不要改** `App.tsx` / `Launcher.tsx` / `QuickSwitcher.tsx` / `ToolWindowRoot.tsx`——它们都从 `globalRegistry.list()` 数据驱动渲染。

### 工具 ID 命名空间（Pin / 启动器识别用）

- 内置工具：`tool:<id>` —— `<id>` 即 `manifest.id`
- 安装应用：`app:<fnv64-of-path>` —— FNV-1a 64 对归一化路径（小写 + 正斜杠）哈希（[cmd/apps.rs:226](src-tauri/src/cmd/apps.rs) `stable_id_for`）

---

## Tauri 命令清单（v0.2 实际暴露）

| 命令 | Rust 名 | 用途 |
|------|---------|------|
| `list_ports(query)` | `cmd::ports::list_ports` | `query` 字符串过滤；权限错以 `PortError::PermissionDenied` 透传 |
| `kill_port(port)` | `cmd::ports::kill_port` | 单 PID 释放；选 LISTENING 优先；UI 二次确认 |
| `kill_by_process_name(name)` | `cmd::ports::kill_by_process_name` | 批量结束（v0.2 新增） |
| `list_capabilities()` | `cmd::capabilities::list_capabilities` | 后端能力声明（V0.3+ 权限校验） |
| `list_installed_apps()` | `cmd::apps::list_installed_apps` | Win: .lnk + PowerShell；Unix: .desktop |
| `launch_app(target)` | `cmd::apps::launch_app` | `cmd /C start "" "<target>"`（detach） |
| `open_quick_switcher()` | `cmd::quick_switcher::open_quick_switcher` | 切换/创建/居中 |
| `open_tool_window(...)` | `cmd::windows::open_tool_window` | 创建或聚焦 `tool-<id>`；支持 use_and_go |
| `close_tool_window(pluginId)` | `cmd::windows::close_tool_window` | 显式关闭 |
| `get_settings()` | `cmd::settings::get_settings` | 读 `app_config_dir/settings.json`（带缓存） |
| `set_settings(settings)` | `cmd::settings::set_settings` | 原子写 + 同步 close_behavior + 重注册快捷键 |
| `set_recording_mode(recording)` | `cmd::settings::set_recording_mode` | 录制时关闭钩子吞咽 + 注销旧快捷键 |
| `list_env()` | `cmd::env::list_env` | user + system 变量 + 解析后的 PATH |
| `set_var(scope, name, value)` | `cmd::env::set_var_cmd` | 写单条 |
| `delete_var(scope, name)` | `cmd::env::delete_var_cmd` | 删除 |
| `set_path_entries(scope, entries[])` | `cmd::env::set_path_entries_cmd` | 整体替换 PATH |
| `detect_preset(kind, dir)` | `cmd::env::detect_preset_cmd` | 探测 JDK/Python/Node/Go/Rust 路径 |
| `apply_preset(plan)` | `cmd::env::apply_preset_cmd` | 应用 plan（写变量 + 改 PATH） |
| `get_hook_diagnostics()` | `crate::windows_hook::get_hook_diagnostics` | 钩子命中/吞咽/QS hwnd 诊断 |

所有命令返回 `Result<T, String>`（IPC 边界 stringified），内部错误用 `thiserror`（`PortError` / `AppsError` / `EnvError`）。

---

## Settings 系统（前后端契约）

[src/settings.ts](src/settings.ts) + [src-tauri/src/cmd/settings.rs](src-tauri/src/cmd/settings.rs)

```typescript
interface AppSettings {
  mode: 'embedded' | 'desktop';           // 主窗布局：Launcher（默认）or Sidebar+主区+StatusBar
  closeBehavior: 'quit' | 'hide';         // 关主窗：真退出 or 隐藏到托盘
  pinnedApps: string[];                   // Pin 进快速启动器的 id（'tool:<id>' 或 'app:<hash>'）
  quickLaunchShortcut: string;            // 全局快捷键字符串，例 'Ctrl+Space'
}

const DEFAULT_SETTINGS = {
  mode: 'desktop',
  closeBehavior: 'hide',
  pinnedApps: [],
  quickLaunchShortcut: 'Ctrl+Space',
};
```

**持久化路径**：`app.path().app_config_dir().join("settings.json")`（Windows: `%APPDATA%/com.toolBench.app/`；macOS: `~/Library/Application Support/com.toolBench.app/`；Linux: `~/.config/com.toolBench.app/`）

**原子写**：[settings.rs:67](src-tauri/src/cmd/settings.rs#L67) `save()` 写 `.json.tmp` 然后 `rename()`。

**前端状态同步**：`useSettings()` 在 set 时乐观更新 + `dispatchEvent('devtoolkit-settings-changed', ...)` 广播给同进程其它实例 + fire-and-forget `invoke('set_settings', ...)`。

**注意**：`mode: 'embedded'` 当前未深度启用（默认走 Launcher），保留 schema 是为了 v0.3+ 的"嵌入式工具窗"模式。

---

## Quick Switcher 行为细节

[src/components/QuickSwitcher.tsx](src/components/QuickSwitcher.tsx) + [src/QuickSwitcherRoot.tsx](src/QuickSwitcherRoot.tsx) + [src-tauri/src/cmd/quick_switcher.rs](src-tauri/src/cmd/quick_switcher.rs)

- 窗口配置：720×380，无装饰、`always_on_top`、`skip_taskbar`、可调整（最大 960×640、最小 480×240）
- 启动时预创建（隐藏）；首次 `Ctrl+Space` 零延迟
- 切换逻辑：可见则隐藏；不可见则居中显示
- **首次聚焦** 600ms grace（防 webview2 首次 paint 时 spurious blur）；**失焦 200ms** 后自动隐藏（`is_focused()` 最终检查兜底）
- 搜索：按 `score(query, name)` 排序（完全匹配 < 前缀匹配 < 子串位置），取前 64
- 工具支持：键盘上下左右移动 / Enter 打开 / Esc 关闭 / Pin 图标固定 / Zap 图标"即开即用"
- Pin 数据持久化在 `settings.pinnedApps`（同进程所有 QS 实例共享）

---

## Windows 平台细节

### windows_hook.rs（**仅 Windows 编译**）

`WH_KEYBOARD_LL` 全局钩子 + 窗口子类化。Why：**全局快捷键（包括 Alt+Space）会被系统/应用的 SC_KEYMENU 抢走**，Tauri 的 `global_shortcut` 在某些键上不够用；hook 是兜底。

- 钩子装在主线程（`SetWindowsHookExW` + `GetModuleHandleW(NULL)`）
- 关键静态：`HOOK_HANDLE` / `SUPPRESS` / `QS_HWND` + 诊断计数（`LL_HOOK_HITS` / `LL_HOOK_ESC_HITS` / 等）
- **吞 Alt+Space**（`SC_KEYMENU`）：通过子类化（`SetWindowLongPtrW` 替换 `WNDPROC`）拦截 `WM_SYSCOMMAND`，保留给用户自定义快捷键
- **吞 Esc 关 QS**：钩子里直接 `PostMessageW(QS_HWND, WM_CLOSE, 0, 0)`，避免 webview 焦点状态竞争
- **录制模式**（`set_recording_mode(true)`）：`SUPPRESS = false` 让键透传到 webview；结束 `SUPPRESS = true` 恢复
- **幂等子类化**：用 `GetPropW` 标记已子类化的 HWND，避免重复替换
- **诊断命令**：`get_hook_diagnostics()` 返回 JSON 计数；调试钩子行为时第一个去看

### apps.rs 的 PowerShell 解析

走 `powershell -File <script.ps1> <paths...>`，**不是** `-Command <script> <paths...>`。Why：PowerShell 5.1 的 `-Command` 会把后续所有 token 当成脚本再解析，路径里 `$` / 括号会被当成子表达式/变量引用，在受限执行策略下报 `PSSecurityException + InvalidResult`。`-File` + `param([string[]]$Links)` 把参数绑成参数，PowerShell 不会重解析。

脚本写到 `temp_dir/devtoolkit-resolve-<pid>.ps1`（pid 防并发），执行后 `remove_file` 清理。

### set_var / apply_preset 的 Windows 实现

`cmd/env.rs` 走注册表 / `SetX` 等 Windows API。改完用户变量需要新开 cmd 才能生效（OS 行为，应用层无解）；UI 用 warnings 提示用户。

---

## 关键约定（首次修改前读一遍）

1. **数据契约同步**：新增/修改 Tauri 命令字段必须**同步**改 `src/types.ts` + 对应的 Rust struct；编译期类型不保护 IPC 边界（运行时 serde 检查）
2. **`PortInfo.process_name` 是 `Option<String>`**：Windows `netstat` 非管理员大多返回 `null`；UI **必须**容忍
3. **`chcp 65001 > nul && netstat -ano` 不可省**：Windows 控制台默认编码（非英文系统）会破坏 netstat `-b` trailer 的 `[exe]` 行
4. **权限错误** stderr 子串匹配：Windows `"Access is denied"` / Unix `"Operation not permitted"` → `PortError::PermissionDenied`
5. **设置原子写**：永远是 `.tmp` → `rename`，不要直接 `write` 覆盖
6. **全局快捷键变更**：`unregister_all()` 后再 `on_shortcut(new, ...)`，否则会重复触发
7. **快捷键字符串格式**：用 Tauri `Shortcut::parse()` 能识别的格式（`Ctrl+Space` / `Alt+Space` / `CmdOrCtrl+Shift+P` 等）
8. **不引入第三方 Rust crate**（如 sysinfo / netstat2 / windows）—— 按设计原则解析 CLI / `WScript.Shell` 输出
9. **没有前端测试框架**（Vitest / RTL）—— v0.2 仍在 UI 快速迭代
10. **样式** = 原生 CSS + CSS Variables；**图标** = `lucide-react`；**状态** = `useState` + `useSettings`；**不要**引入 Tailwind / CSS-in-JS / Redux / Zustand

---

## 调试 / 诊断

- **Windows 钩子行为**：`invoke('get_hook_diagnostics')` 返回命中数、Esc 命中、QS hwnd 等。钩子不工作就先看这个
- **快捷键冲突**：先用 `Ctrl+Space`（默认推荐）；`Alt+Space` 在 Windows 上常被系统拦截（[SettingsPanel.tsx:308](src/components/SettingsPanel.tsx#L308) 有提示）
- **托盘不出现**：检查 `lib.rs::build_tray` 的 `default_window_icon()` 是否有 icon 资源
- **快速启动器不显示**：检查 `lib.rs::setup` 是否调用了 `quick_switcher::precreate`
- **设置改完不生效**：检查 `useSettings` 的 `loaded.current` ref 防重入；多个窗口共享同一设置

---

## Vite / Tauri / Capabilities 配置

- Vite 端口：`1420`（`strictPort: true`），Tauri 期望这个端口，**不要改**；HMR 走 `1421`（`TAURI_DEV_HOST` 设置时启用）
- `vite.config.ts` 的 `watch.ignored` 排除 `src-tauri/**`——后端改动不会触发前端 reload（`cargo` 单独跑）
- Tauri capabilities（前端权限）：[src-tauri/capabilities/default.json](src-tauri/capabilities/default.json)。**新增 invoke 命令时必须同步加权限**，否则会被 Tauri 静默拒绝
- `tauri.conf.json` 的窗口配置只用于 `main`；`quick-switcher` 和 `tool-<id>` 在 Rust 代码里用 `WebviewWindowBuilder` 动态创建

---

## 测试策略

- **Rust 单元测试**（`cargo test`）：
  - `platform::windows::tests` / `platform::unix::tests` —— 解析器（LISTEN/ESTABLISHED/UDP/无效行/去重）
  - `cmd::apps::tests` —— `stable_id_for` 路径分隔符大小写不敏感、不同路径得到不同 id
  - `cmd::settings::tests` —— 默认值、JSON 序列化往返（如有）
- **手动验证** 仍是验证 IPC + UI 完整流程的唯一方式。**不要**依赖"编译通过 + 单元测试绿"就声称完成
- **不引入** Vitest / RTL 等前端测试框架——v0.3+ 再评估

---

## Out of Scope（暂不实现 / 不要顺手做）

- 工具之间共享数据
- 工具访问互联网（保持离线）
- 主题切换（深色为唯一主题）
- 自动刷新（port-manager 当前是手动刷新按钮；v0.4 评估）
- Linux/macOS 的 `.desktop` 应用扫描完整性（v0.2 兜底即可）
- 前端测试框架
- 打包发布
- 远程插件市场 / 签名（v0.4+）

---

## 资源链接

- **PRD**：[docs/PRD.md](docs/PRD.md) —— 产品定位、功能边界
- **工具候选清单**：[docs/tools-planning.md](docs/tools-planning.md) —— 未来内置工具池
- **v0.1 设计**：[docs/superpowers/specs/2026-06-05-devtoolkit-v0.1-design.md](docs/superpowers/specs/2026-06-05-devtoolkit-v0.1-design.md)
- **v0.1 计划**：[docs/superpowers/plans/2026-06-05-devtoolkit-v0.1.md](docs/superpowers/plans/2026-06-05-devtoolkit-v0.1.md)
- **README**：[README.md](README.md) —— 用户/新人入口
- **Tauri 2 文档**：https://tauri.app/develop/
