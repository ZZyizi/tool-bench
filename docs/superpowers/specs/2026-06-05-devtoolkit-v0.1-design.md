# toolBench V0.1 MVP 设计规格

**日期**: 2026-06-05（v2 - 加入插件系统）
**项目**: toolBench (轻量级开发者桌面工具箱) — 工程目录 `d:\project\tool-bench`
**包命名**:
- npm package: `tool-bench`
- Rust crate: `toolBench`
- Rust lib: `tool_bench_lib`
- Tauri identifier: `com.toolBench.app`
- 产品显示名: `toolBench`
**里程碑**: v0.1.0 (P0 MVP)

---

## 1. 目标与背景

实现 PRD §7.1 定义的 v0.1.0 MVP：
- [ ] 端口占用列表展示
- [ ] 端口占用清除
- [ ] 基础 UI 框架
- [ ] 扩展接口定义（Plugin Manifest + Plugin Registry）

**未来方向**: 插件商城（V0.3+）— 用户可从市场安装第三方插件，零重启启用。

## 2. 已确认的关键决策

| 维度 | 决策 |
|------|------|
| 实施范围 | 完整 P0 MVP |
| 前端样式 | 原生 CSS + CSS Variables（深色主题） |
| 端口扫描实现 | 解析系统命令（netstat/lsof/ss） |
| 端口列表刷新 | 手动刷新按钮 |
| **插件运行时** | **JS/TS（运行在 webview 中）** |
| **插件加载范围** | **V0.1：架构预留 + Manifest 定义；V0.3+ 动态加载** |
| **前端工具列表** | **数据驱动（从 Plugin Registry 渲染）** |
| 前端状态管理 | useState + 组件分层 |
| 测试覆盖 | Rust 单元测试（netstat 解析、Platform 抽象） |

## 3. 整体架构：双层职责

```
┌─────────────────────────────────────────────────────────┐
│           Frontend (React, src/)                         │
│                                                          │
│  ┌────────────────────────────────────────────────────┐ │
│  │  Plugin System (V0.1 静态, V0.3+ 动态)            │ │
│  │  PluginManifest / PluginRegistry / PluginContext   │ │
│  │  Built-in: port-manager (作为 V0.1 唯一插件)       │ │
│  └────────────────────────────────────────────────────┘ │
│         ↓ invoke()                                       │
│  ┌────────────────────────────────────────────────────┐ │
│  │  System API Bridge (Tauri commands)                │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────┬───────────────────────────────┘
                          │ IPC
┌─────────────────────────┴───────────────────────────────┐
│           Backend (Rust, src-tauri/)                     │
│                                                          │
│  ┌────────────────────────────────────────────────────┐ │
│  │  System API Layer (Tauri commands)                 │ │
│  │  list_ports, kill_port, list_capabilities, ...    │ │
│  │  ↓                                                   │ │
│  │  Platform Layer (PortScanner trait)                │ │
│  │  Windows / Unix 实现 (netstat / lsof 解析)         │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

**关键洞察**：Rust 端退化为"系统能力提供者"，不再定义 Tool 抽象。Tool/Plugin 抽象完全在前端，第三方作者用 JS/TS 写插件，通过 Tauri command 桥接访问系统能力。

## 4. 插件系统设计（V0.1 核心）

### 4.1 插件清单格式 (Plugin Manifest)

```toml
# plugins/<plugin-id>/plugin.toml
[plugin]
id = "port-manager"             # 唯一标识 (kebab-case)
name = "端口管理"               # 显示名
version = "0.1.0"
description = "查看和释放端口占用"
author = "toolBench Team"
category = "Network"            # Network / Encode / System / Other
icon = "🔌"                     # emoji 或路径
entry = "./index.ts"            # 入口文件 (V0.3 动态加载用)
homepage = "..."                # 可选，V0.3 商城用

[plugin.capabilities]
# 该插件请求的系统能力，V0.3+ 用于权限控制
required = ["network:read", "process:read", "process:kill"]
```

### 4.2 TypeScript 插件接口

```typescript
// src/plugins/types.ts
export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  category: string;
  icon?: string;
  entry: string;
  capabilities?: string[];
}

export interface PluginContext {
  invoke: typeof import('@tauri-apps/api/core').invoke;
  notify: (message: string, type?: 'info' | 'success' | 'error') => void;
  log: (...args: unknown[]) => void;
}

export interface Plugin {
  manifest: PluginManifest;
  activate(context: PluginContext): void | Promise<void>;
  deactivate?(): void | Promise<void>;
  Component?: React.ComponentType;  // 插件渲染的 React 组件
}
```

### 4.3 插件注册器

```typescript
// src/plugins/registry.ts
export class PluginRegistry {
  private plugins = new Map<string, Plugin>();
  
  register(plugin: Plugin): void {
    if (this.plugins.has(plugin.manifest.id)) {
      throw new Error(`Plugin ${plugin.manifest.id} already registered`);
    }
    this.plugins.set(plugin.manifest.id, plugin);
  }
  
  unregister(id: string): void { ... }
  list(): Plugin[] { ... }
  get(id: string): Plugin | undefined { ... }
  byCategory(): Map<string, Plugin[]> { ... }
}

export const globalRegistry = new PluginRegistry();
```

### 4.4 插件上下文

```typescript
// src/plugins/context.ts
export const createPluginContext = (): PluginContext => ({
  invoke: tauriInvoke,
  notify: (msg, type) => { /* TODO: toast 系统 */ },
  log: (...args) => console.log('[plugin]', ...args),
});
```

### 4.5 V0.1 内置插件示例：port-manager

```typescript
// src/plugins/builtin/port-manager/index.ts
import { Plugin } from '../../types';
import { PortView } from './PortView';
import manifest from './plugin.toml';

export const portManagerPlugin: Plugin = {
  manifest,
  Component: PortView,
  activate(ctx) {
    ctx.log('Port manager activated');
  },
};
```

V0.1 中所有插件都是**静态 import**（V0.3+ 改为 dynamic import + 文件系统扫描）。

## 5. 前端结构

```
src/
├── main.tsx
├── App.tsx                       # 布局壳，从 registry 渲染
├── App.css                       # CSS Variables (深色主题)
├── components/
│   ├── Sidebar.tsx              # 侧边栏，按 category 分组渲染插件
│   ├── Sidebar.css
│   ├── StatusBar.tsx            # 状态栏（工具数 / 版本 / 状态）
│   └── ConfirmDialog.tsx        # 二次确认弹窗
├── plugins/
│   ├── types.ts                 # Plugin / PluginManifest
│   ├── registry.ts              # PluginRegistry + globalRegistry
│   ├── context.ts               # createPluginContext
│   ├── api.ts                   # 系统 API 封装（plugins 用）
│   └── builtin/
│       └── port-manager/
│           ├── plugin.toml
│           ├── index.ts         # Plugin 导出
│           ├── PortView.tsx     # 端口列表组件
│           └── PortView.css
└── types.ts                     # PortInfo / KillResult TS 类型
```

## 6. Rust 后端（系统 API）

### 6.1 模块结构（保持 V0.1 v1 设计）

```
src-tauri/src/
├── main.rs
├── lib.rs                       # run() + AppState
├── cmd/                         # Tauri commands = 插件可调用的系统 API
│   ├── mod.rs
│   ├── ports.rs                # list_ports, kill_port
│   └── capabilities.rs         # list_capabilities (V0.1 简化版)
└── platform/                    # 平台抽象
    ├── mod.rs
    ├── port_scanner.rs         # PortScanner trait + PortInfo
    ├── windows.rs              # WindowsPortScanner
    └── unix.rs                 # UnixPortScanner
```

### 6.2 新增 `list_capabilities` command

声明后端可提供的能力，V0.3+ 插件权限校验使用：

```rust
#[derive(Serialize)]
pub struct Capabilities {
    pub network_read: bool,    // netstat / lsof
    pub process_read: bool,    // tasklist / ps
    pub process_kill: bool,    // taskkill / kill
    pub dns: bool,             // V1.0+
    pub file_read: bool,       // V1.0+
}

#[tauri::command]
pub fn list_capabilities() -> Capabilities {
    Capabilities {
        network_read: true,
        process_read: true,
        process_kill: true,
        dns: false,
        file_read: false,
    }
}
```

### 6.3 端口 API（V0.1 v1 不变）

```rust
#[tauri::command]
pub fn list_ports(state: tauri::State<AppState>) -> Result<Vec<PortInfo>, String>;

#[tauri::command]
pub fn kill_port(port: u16, state: tauri::State<AppState>) -> Result<KillResult, String>;
```

### 6.4 AppState（V0.1 v1 不变）

```rust
pub struct AppState {
    pub scanner: Arc<dyn PortScanner>,
}
```

### 6.5 依赖（V0.1 v1 不变）

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
```

## 7. V0.1 完整文件清单

### Rust 端 (src-tauri/src/)
- `lib.rs` — 100 行左右：Tauri builder + AppState + 注册 command
- `cmd/ports.rs` — 80 行：list_ports / kill_port
- `cmd/capabilities.rs` — 30 行
- `platform/port_scanner.rs` — 100 行：trait + PortInfo
- `platform/windows.rs` — 150 行：netstat 解析 + 单元测试
- `platform/unix.rs` — 150 行：lsof 解析 + 单元测试
- `platform/mod.rs` — 20 行：工厂函数选择实现

### 前端 (src/)
- `main.tsx` — 10 行
- `App.tsx` — 80 行：布局 + 渲染 plugin
- `App.css` — 150 行：CSS Variables + 布局
- `components/Sidebar.tsx` — 60 行：从 registry 渲染
- `components/Sidebar.css`
- `components/StatusBar.tsx` — 30 行
- `components/ConfirmDialog.tsx` — 60 行
- `plugins/types.ts` — 50 行
- `plugins/registry.ts` — 50 行
- `plugins/context.ts` — 20 行
- `plugins/api.ts` — 30 行
- `plugins/builtin/port-manager/plugin.toml` — 10 行
- `plugins/builtin/port-manager/index.ts` — 10 行
- `plugins/builtin/port-manager/PortView.tsx` — 200 行
- `plugins/builtin/port-manager/PortView.css` — 100 行
- `types.ts` — 30 行

## 8. 关键交互

| 操作 | 流程 |
|------|------|
| 启动 | App 初始化 → 加载内置插件 → 渲染 Sidebar → 渲染 PortView → invoke('list_ports') |
| 切换工具 | 点击 Sidebar 项 → 切换渲染的 Component |
| 释放端口 | 选中行 → 二次确认弹窗 → invoke('kill_port') → toast → 刷新 |
| 错误处理 | PluginContext.notify() 统一通知，V0.1 用简单顶部 banner |

## 9. 测试策略

### Rust 单元测试
- `platform/windows.rs::parse_netstat` — mock netstat 输出
- `platform/unix.rs::parse_lsof` — mock lsof 输出
- `cmd/ports.rs::kill_port` 边界条件（端口未占用 / 权限不足）

### 手动验证
- `npm run tauri dev`
- 检查端口列表渲染
- 启动测试服务（`python -m http.server 8000`）→ 验证出现 8000
- kill → 验证端口消失
- kill 系统进程 → 验证错误处理
- 检查深色主题

## 10. 验收标准

- [ ] `cargo test` 全绿
- [ ] `npm run tauri dev` 启动后能列出端口
- [ ] 端口列表字段齐全：协议、端口、PID、进程名、状态
- [ ] 二次确认后能成功 kill
- [ ] UI 深色主题无明显瑕疵
- [ ] PluginManifest 格式定义清晰，1 个内置插件 (port-manager) 走通
- [ ] PluginRegistry 支持 register/unregister/list/byCategory
- [ ] 新增第二个内置插件（如 base64）只需：写 plugin.toml + Component + 在 builtin/index.ts 注册，不改 App.tsx
- [ ] capabilities command 返回当前可用能力

## 11. 范围外（V0.1 不做）

- 搜索过滤（PRD v0.2）
- 自动刷新（V0.1 手动）
- 设置面板 / 主题切换
- 多内置工具（V0.1 仅 port-manager）
- 动态插件加载（V0.3+）
- 插件商城 / 安装 / 签名校验（V0.3+）
- 插件权限系统（V0.3+ 启用 manifest capabilities）
- 打包发布

## 12. 未来路线图（参考，非 V0.1 范围）

### V0.2 - 内置工具扩展
- 增加 2-3 个内置工具（base64 编码、DNS 查询、JSON 格式化）
- 全部以 builtin plugin 形式实现
- 搜索过滤

### V0.3 - 动态插件加载
- 从 `~/.tool-bench/plugins/` 或应用内 plugins/ 加载 .toml + .js
- Dynamic import
- 插件启用/禁用
- 本地插件市场（filesystem browse）

### V0.4 - 远程插件商城
- HTTP 后端 / 静态资源服务 / CDN
- 插件搜索 / 详情 / 安装
- 版本管理 / 自动更新
- 签名校验 / 沙箱（V0.4+ 评估 WASM 沙箱需求）

## 13. 风险与缓解

| 风险 | 缓解 |
|------|------|
| netstat 中文 Windows 编码问题 | `chcp 65001 > nul && netstat -ano` |
| 进程名获取权限不足 | 接受 Option，None 时显示 PID only |
| kill 系统关键进程 | 二次确认 + 失败时友好提示 |
| 插件作者误用 invoke 调危险 API | V0.3+ 加入 capability 校验 |
| 第三方插件供应链 | V0.3+ 引入签名机制 |
