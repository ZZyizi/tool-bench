# toolBench

> 面向开发者的轻量级桌面工具箱 —— 小而美、开箱即用、离线运行、可扩展。
> 当前的代码包名：`tool-bench`（Rust crate 名：`toolBench`）。

[![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8D8?logo=tauri)](https://tauri.app/)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.8-3178C6?logo=typescript)](https://www.typescriptlang.org/)
[![Vite](https://img.shields.io/badge/Vite-7-646CFF?logo=vite)](https://vitejs.dev/)
[![Rust](https://img.shields.io/badge/Rust-stable-DEA584?logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/平台-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)](#跨平台支持)
[![License](https://img.shields.io/badge/license-待定-lightgrey)](#许可证)

---

## 项目简介

**toolBench** 是一款使用 **Tauri 2 + React 19 + TypeScript 5.8 + Vite 7** 构建的跨平台桌面工具箱，专注于解决开发者在日常工作中高频但繁琐的小问题。

核心理念：

- **快**：启动 < 1s，无需配置
- **轻**：< 50 MB 内存，单文件运行
- **准**：输出精确，无需解读
- **离线**：完全本地运行，不上传任何数据
- **扩展**：插件化架构，按需添加工具

### 目标用户

- 后端 / 全栈开发者
- DevOps 工程师
- 运维与本地环境诊断人员

### v0.1 内置工具

| 工具 | 功能 |
|------|------|
| 端口管理 | 查看所有被占用的端口 / 一键释放指定端口 |

未来计划加入：网络连通性检测、DNS 解析、Hosts 管理、JSON / Base64 编解码等。

更多规划见 [PRD §7 版本规划](PRD.md#7-版本规划)。

---

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面壳 | Tauri 2（Rust） |
| 前端框架 | React 19 + TypeScript 5.8 |
| 构建工具 | Vite 7 |
| 跨平台 | Windows / macOS / Linux |
| 状态管理 | React `useState`（v0.1 暂不引入 Redux/Zustand） |
| 样式 | 原生 CSS + CSS Variables（深色主题） |
| IPC | `@tauri-apps/api` 的 `invoke()` |

> **依赖原则**：不引入 sysinfo / netstat2 等第三方 Rust crate —— 平台层直接解析 `netstat` / `lsof` 的 CLI 输出。

---

## 快速开始

### 前置要求

- **Node.js** ≥ 18
- **Rust** stable（[rustup](https://rustup.rs)）
- 平台构建工具：
  - **Windows**：Microsoft Visual Studio C++ Build Tools + WebView2
  - **macOS**：Xcode Command Line Tools
  - **Linux**：webkit2gtk、libssl-dev 等（见 [Tauri 文档](https://tauri.app/start/prerequisites/)）

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

> Vite 端口固定为 `1420`（`strictPort: true`），Tauri 期望这个端口，**不要修改**。

### 构建生产包

```bash
npm run build        # tsc + vite build
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

---

## 项目架构

DevToolkit 采用 **两层职责分离** 的设计 —— **Rust 只做"系统能力提供者"**，**所有 Tool / Plugin 逻辑都在前端**。第三方插件作者只需要写 JS/TS，通过 `invoke()` 调用后端能力。

```
┌─────────────────────────────────────────────────────────┐
│  Frontend (React, src/)                                 │
│   ├── Plugin System (Plugin / PluginManifest /         │
│   │                  PluginRegistry / PluginContext)   │
│   │     Built-in: port-manager (v0.1 唯一插件)         │
│   └── System API Bridge → invoke('list_ports', ...)    │
└─────────────────────────┬───────────────────────────────┘
                          │ IPC
┌─────────────────────────┴───────────────────────────────┐
│  Backend (Rust, src-tauri/src/)                         │
│   └── System API Layer (Tauri commands)                │
│         ↓ Platform Layer (PortScanner trait)            │
│           Windows: netstat  │  Unix: lsof              │
└─────────────────────────────────────────────────────────┘
```

### 目录结构

```
.
├── src/                                # 前端 (React + TS)
│   ├── main.tsx, App.tsx, App.css      # 布局 shell（数据驱动）
│   ├── types.ts                        # 共享 TS 类型
│   ├── components/                     # Sidebar / StatusBar / ConfirmDialog
│   └── plugins/
│       ├── types.ts                    # Plugin / PluginManifest / PluginContext
│       ├── registry.ts                 # PluginRegistry + globalRegistry
│       ├── context.ts                  # createPluginContext()
│       ├── api.ts                      # 类型化的 invoke 包装
│       └── builtin/
│           ├── index.ts                # 注册所有内置插件
│           └── port-manager/           # v0.1 唯一插件
│               ├── plugin.toml         # [plugin] / [plugin.capabilities]
│               ├── index.ts
│               ├── PortView.tsx
│               └── PortView.css
├── src-tauri/                          # 后端 (Rust)
│   └── src/
│       ├── main.rs                     # 调用 tool_bench_lib::run()
│       ├── lib.rs                      # Tauri builder + AppState
│       ├── cmd/
│       │   ├── ports.rs                # list_ports / kill_port
│       │   └── capabilities.rs         # list_capabilities
│       └── platform/
│           ├── mod.rs                  # 工厂方法（按平台选择实现）
│           ├── port_scanner.rs         # PortScanner trait / PortInfo / PortError
│           ├── windows.rs              # netstat 解析 + 测试（#[cfg(windows)]）
│           └── unix.rs                 # lsof 解析 + 测试（#[cfg(unix)]）
├── docs/
│   └── superpowers/
│       ├── specs/                      # 各版本设计文档
│       └── plans/                      # 任务分解与进度跟踪
├── PRD.md                              # 产品需求文档
├── package.json                        # npm 包（tool-bench）
├── vite.config.ts                      # Vite 配置（端口 1420）
└── CLAUDE.md                           # Claude Code 工作指引
```

---

## 暴露给前端的 Tauri 命令

| 命令 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `list_ports` | — | `PortInfo[]` | 通过 platform 工厂获取；权限错误会以 `PortError::PermissionDenied` 形式回传 |
| `kill_port` | `port: u16` | `KillResult` | 前端 UI 二次确认；后端调用 `taskkill /F`（Windows）或 `kill -9`（Unix） |
| `list_capabilities` | — | `Capabilities` | 声明后端可提供的能力（v0.3+ 用于插件权限校验） |

> 跨边界类型在 IPC 边界统一为 `Result<T, String>`，内部错误由 `PortError` + `thiserror` 定义。

---

## 添加新工具（插件）

v0.1 插件是 **静态导入** 的，过程非常轻量：

1. 在 [src/plugins/builtin/](src/plugins/builtin/) 下创建新目录，例如 `<name>/`
2. 准备以下文件：
   - `plugin.toml` —— 声明 `id` / `name` / `version` / `category` / `entry` / 可选 `capabilities`
   - `index.ts` —— 导出 `Plugin` 对象
   - `<Name>View.tsx` + `<Name>View.css` —— 工具视图
3. 在 [src/plugins/builtin/index.ts](src/plugins/builtin/index.ts) 中 **加一行** 注册
4. 如果插件需要新的后端能力：
   - 在 `src-tauri/src/cmd/` 添加命令
   - 在 [src-tauri/capabilities/default.json](src-tauri/capabilities/default.json) 注册前端权限
5. 重新启动 `npm run tauri dev`

> **无需** 改动 `App.tsx` / `Sidebar.tsx` / `components/` —— Sidebar 通过 `globalRegistry.list()` 自动渲染。"加一个新插件只改一个文件" 就是插件基础设施的验收标准。

---

## 跨平台支持

| 平台 | 端口查询 | 进程终止 |
|------|----------|----------|
| Windows | `netstat -ano`（强制 chcp 65001 避免乱码） | `taskkill /F /PID xxx` |
| macOS | `lsof -i` | `kill -9 PID` |
| Linux | `lsof -i` | `kill -9 PID` |

**权限错误识别**（在 stderr 上做子串匹配）：

- Windows：`"Access is denied"`
- Unix：`"Operation not permitted"`

命中后映射为 `PortError::PermissionDenied`，UI 展示友好提示。

> 注意：`PortInfo.process_name` 是 `Option<String>`。Windows 下 `netstat` 不能廉价地拿到进程名，因此返回 `None`，UI 在此场景下只显示 PID。

---

## 测试策略

- **Rust 单元测试** 覆盖解析器：
  - `parse_netstat`：4 个测试用例（mock CLI 输出）
  - `parse_lsof`：4 个测试用例
  - `kill_port`：边界场景
- **手动验证** 是验证 IPC + UI 完整流程的唯一方式，参见下方的 "手动验证" 段落。
- v0.1 **不引入** 前端测试框架（Vitest / RTL 等）—— 在路线图外。

### 手动验证

```bash
# 终端 1：启动一个会占用端口的本地服务
python -m http.server 8765

# 终端 2：启动应用，验证 8765 出现在端口表格中
npm run tauri dev
```

---

## 路线图

| 版本 | 范围 |
|------|------|
| **v0.1**（当前） | 端口列表 / 端口释放 / 基础 UI / 插件基础设施（port-manager） |
| v0.2 | 搜索 / 过滤 / 工具切换动画 / 设置面板（主题、语言）/ 快捷键 |
| v0.3 | 文件系统动态加载插件 / 插件权限强制（基于 `manifest.capabilities`） |
| v0.4 | 插件市场 / 安装 / 签名校验 |
| v1.0 | 网络连通性检测 / DNS 解析 / Hosts 管理 / 编解码工具 / 打包发布 |

详细见 [PRD §7](PRD.md#7-版本规划) 与 [docs/superpowers/specs/](docs/superpowers/specs/) 下的设计文档。

---

## 贡献

欢迎 PR！但请先阅读：

1. [CLAUDE.md](CLAUDE.md) —— 项目约定、命名规范、目录结构
2. [PRD.md](PRD.md) —— 产品定位与功能边界
3. 相关版本的设计文档（[docs/superpowers/specs/](docs/superpowers/specs/)）与执行计划（[docs/superpowers/plans/](docs/superpowers/plans/)）

**v0.1 不在范围内**（暂不接受）：

- 搜索 / 过滤（v0.2）
- 自动刷新（v0.1 仅手动刷新）
- 设置面板 / 主题切换
- 除 `port-manager` 外的内置工具
- 文件系统动态加载插件（v0.3）
- 插件市场 / 签名（v0.4）
- 打包发布

---

## 资源链接

- **产品文档**：[docs/PRD.md](docs/PRD.md)
- **v0.1 设计**：[docs/superpowers/specs/2026-06-05-devtoolkit-v0.1-design.md](docs/superpowers/specs/2026-06-05-devtoolkit-v0.1-design.md)
- **v0.1 计划**：[docs/superpowers/plans/2026-06-05-devtoolkit-v0.1.md](docs/superpowers/plans/2026-06-05-devtoolkit-v0.1.md)
- **Tauri 2 文档**：https://tauri.app/develop/

---
