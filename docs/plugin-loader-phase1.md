# Phase 1 — 插件加载机制简化

> **目标**：加一个新工具的成本从「改 7 个文件」降到「新建 1 个文件夹 + 写 2 个文件」
> **范围**：仅前端构建期 + Rust 命令注册层。**不引入**运行时 sandbox、运行时 build、JSON-UI 降级。
> **状态**：本文档既是设计稿，也是待办清单。

---

## 1. 背景：V0.1 的痛点

V0.1 加一个 `env-editor` 实际触动 7 个文件（[git diff stat](https://github.com))：

```
src-tauri/Cargo.toml                +1  (tauri-plugin-dialog)
src-tauri/capabilities/default.json +1  (dialog:default)
src-tauri/src/cmd/mod.rs            +1  (pub mod env)
src-tauri/src/lib.rs                +7  (6 个 invoke_handler)
src-tauri/src/cmd/env.rs            +N  (新文件，命令实现)
src/plugins/api.ts                  +17 (6 个 invoke wrapper)
src/types.ts                        +40 (9 个 interface)
src/plugins/builtin/index.ts        +1  (register)
src/plugins/builtin/env-editor/...  +N  (新文件)
```

其中**真正是「新工具的内容」的只有 `cmd/env.rs` + `env-editor/` 目录**，其余 6 处是注册样板。

## 2. Phase 1 的目标

| 维度 | V0.1 现状 | V0.2 目标 |
|---|---|---|
| 加新 UI 插件 | 改 `builtin/index.ts` 1 行 | **不改**（Vite glob 自动扫） |
| 加新后端命令 | 改 `mod.rs` + `lib.rs` 2 处 | **只改 mod.rs 1 处** + 1 处 dispatch 注册表 |
| 加新 invoke wrapper | 改 `api.ts` N 行 | **不改**（codegen 自动） |
| 加新 TS 类型 | 改 `types.ts` N 行 | 改（仍然手写，但只此一处，可接受） |
| 加新 tauri-plugin | 改 `Cargo.toml` + `capabilities/default.json` | 改（**Tauri 框架硬约束，接受**） |

**新工具最小动作清单**：

```
plugins/<name>/                ← 新建文件夹
  plugin.json                  ← 新建（manifest 声明）
  index.tsx                    ← 新建（export plugin 对象）
  <View>.tsx                   ← 新建（实际 UI，可选拆出）
src-tauri/src/cmd/<name>.rs    ← 新建（命令实现 + register）
src-tauri/src/cmd/mod.rs       ← +1 行 pub mod <name>
src-tauri/src/cmd/dispatch.rs  ← +1 行 <name>::register(&mut r)
src/types.ts                   ← +N 行（如有新类型）
```

7 处 → 5 处。Rust 端的 mod.rs / dispatch.rs 2 处是 Rust 模块系统的硬约束（不可消除），其余 3 处是真正的"新工具内容"。

---

## 3. 架构设计

### 3.1 数据流

```
┌──────────────────────────────────────────────────────────────┐
│ plugins/port-manager/                                        │
│   ├── plugin.json   (Vite 编译期 import 为 manifest)         │
│   ├── index.tsx     (export Plugin 对象)                    │
│   └── PortView.tsx  (实际 UI)                                │
└───────────────┬──────────────────────────────────────────────┘
                │ Vite import.meta.glob 扫描
                ▼
┌──────────────────────────────────────────────────────────────┐
│ src/plugins/builtin/index.ts (Vite glob 自动生成)            │
│   for each plugins/*/index.tsx:                             │
│     globalRegistry.register(plugin)                          │
└───────────────┬──────────────────────────────────────────────┘
                │
                ▼ globalRegistry.list()
┌──────────────────────────────────────────────────────────────┐
│ UI 层 (无变化)                                               │
│   Sidebar / Launcher / QuickSwitcher 全部数据驱动            │
│   ToolWindowRoot 走 globalRegistry.get(pluginId)             │
└───────────────┬──────────────────────────────────────────────┘
                │ invoke('dispatch', { name, args })
                ▼
┌──────────────────────────────────────────────────────────────┐
│ src-tauri/src/cmd/dispatch.rs                                │
│   CommandRegistry (HashMap<String, Handler>)                 │
│   每个命令模块实现 pub fn register(&mut Registry)           │
└──────────────────────────────────────────────────────────────┘
```

### 3.2 关键决策

| 决策 | 选择 | 理由 |
|---|---|---|
| 插件 manifest 格式 | `plugin.json`（JSON） | 多语言、运行时/构建期可读、合并 schema 校验容易 |
| manifest → 代码 | Vite `import manifest from './plugin.json'` | 编译期内联进 bundle，不需运行时 fs |
| 插件发现 | Vite `import.meta.glob('../../plugins/*/index.tsx')` | 内置、不写 Vite 插件、Eager 模式 |
| 插件注册 | 循环 glob 结果 + `globalRegistry.register` | 与现有 registry API 兼容 |
| 后端命令注册 | `CommandRegistry` + `dispatch(name, args)` 万能 command | 消除 `invoke_handler!` 列表 |
| TS API 生成 | codegen 脚本读 plugin.json → 生成 `api.gen.ts` | 消除手写 invoke wrapper |
| TS 类型 | `src/types.ts` 仍手写 | codegen 生成类型不划算，靠单一 source of truth 控制 |
| Sandbox | **不做** | v0.2 仅服务内置插件，全代码 review；外部插件是 v0.3+ |

### 3.3 plugin.json Schema

```json
{
  "id": "port-manager",
  "name": "端口管理",
  "version": "0.1.0",
  "description": "查看和释放系统占用的端口",
  "author": "DevToolkit Team",
  "category": "Network",
  "icon": "Plug",
  "entry": "./index.tsx",
  "capabilities": ["network:read", "process:read", "process:kill"],
  "windowWidth": 800,
  "windowHeight": 560,
  "commands": [
    {
      "name": "list_ports",
      "argsRef": "ListPortsArgs",
      "returnsRef": "FilteredPorts"
    },
    {
      "name": "kill_port",
      "argsRef": "KillPortArgs",
      "returnsRef": "KillResult"
    }
  ]
}
```

字段说明：

- **id**：kebab-case 唯一标识，也是 Tauri 窗口 label 的前缀（`tool-<id>`）
- **icon**：字符串，引用 `lucide-react` 的图标名（见 §3.5 icon 解析）
- **entry**：相对插件目录的入口文件，默认 `./index.tsx`（可省略）
- **capabilities**：声明插件用到的"能力"（Rust 端做 dispatch 时按这个白名单过滤；v0.2 仅记录不强制）
- **commands**：声明插件调用的后端命令，codegen 据此生成 `api.gen.ts`；**argsRef / returnsRef 引用 `src/types.ts` 里 export 的类型名**
- **windowWidth / windowHeight**：可选，覆盖 ToolWindow 的默认尺寸

### 3.4 index.tsx 约定

```tsx
// plugins/port-manager/index.tsx
import { Plug } from 'lucide-react';
import type { Plugin } from '../../src/plugins/types';
import manifestRaw from './plugin.json';
import { PortView } from './PortView';

// plugin.json 没有 React 组件，所以 icon 字段是字符串；
// 这里把字符串映射回 lucide 组件。
const manifest = {
  ...manifestRaw,
  icon: Plug,
} as const;

export const portManagerPlugin: Plugin = {
  manifest,
  Component: PortView,
  activate(ctx) {
    ctx.log('Port manager activated');
  },
};

export default portManagerPlugin;
```

约定：
- 必须 `export default` 一个 `Plugin` 对象（Vite glob 用 `import.meta.glob<{ default: Plugin }>(...)` 收口）
- Plugin.manifest.icon 在 index.tsx 里**手写映射**（保持 JSON 简单），不再依赖 codegen

### 3.5 icon 解析策略

icon 是字符串（如 `"Plug"`），在 `index.tsx` 里手动 `import { Plug } from 'lucide-react'` 后赋值给 `manifest.icon`。

如果新插件想用非 lucide 的图标：
```tsx
import { MyCustomIcon } from './MyCustomIcon';
const manifest = { ...manifestRaw, icon: MyCustomIcon } as const;
```

**Trade-off**：放弃完全 JSON 化换 codegen 复杂度下降。**接受**。

### 3.6 Vite glob 收口

`src/plugins/builtin/index.ts`（**这个文件手写且不再改**）：

```typescript
import { globalRegistry } from '../registry';
import { createPluginContext } from '../context';

const builtinContext = createPluginContext();

const modules = import.meta.glob<{ default: import('../types').Plugin }>(
  '../../../plugins/*/index.tsx',
  { eager: true },
);

for (const [path, mod] of Object.entries(modules)) {
  const plugin = mod.default;
  if (!plugin) {
    console.error(`[plugins] ${path}: missing default export`);
    continue;
  }
  globalRegistry.register(plugin);
  plugin.activate(builtinContext);
}
```

**注意 glob 路径**：
- 文件位置：`src/plugins/builtin/index.ts`
- 目标位置：`plugins/<name>/index.tsx`（项目根目录的 plugins/）
- 相对路径：`../../../plugins/*/index.tsx`

### 3.7 Tauri 端 CommandRegistry

**`src-tauri/src/cmd/dispatch.rs`**（新建）：

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde_json::Value;

pub type HandlerResult = Result<Value, String>;
pub type Handler = Arc<dyn Fn(Value) -> HandlerResult + Send + Sync>;

pub struct CommandRegistry {
    handlers: HashMap<&'static str, Handler>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self { handlers: HashMap::new() }
    }

    pub fn register<F>(&mut self, name: &'static str, handler: F)
    where
        F: Fn(Value) -> HandlerResult + Send + Sync + 'static,
    {
        self.handlers.insert(name, Arc::new(handler));
    }

    pub fn dispatch(&self, name: &str, args: Value) -> HandlerResult {
        let handler = self
            .handlers
            .get(name)
            .ok_or_else(|| format!("unknown command: {name}"))?;
        handler(args)
    }
}

#[tauri::command]
pub fn dispatch(
    state: tauri::State<'_, Arc<Mutex<CommandRegistry>>>,
    name: String,
    args: Option<Value>,
) -> HandlerResult {
    let args = args.unwrap_or(Value::Null);
    let reg = state.lock().map_err(|e| format!("registry lock poisoned: {e}"))?;
    reg.dispatch(&name, args)
}
```

**`src-tauri/src/cmd/<name>.rs`**（每个命令模块）：

```rust
// ports.rs（已有，仅调整结构）
use serde_json::Value;
use crate::platform::port_scanner::PortScanner;
use super::dispatch::CommandRegistry;

pub async fn list_ports_logic(...) -> Result<FilteredPorts, String> { /* 原 #[tauri::command] 的实现 */ }

pub fn list_ports_dispatch(args: Value) -> Result<Value, String> {
    let query: ListPortsArgs = serde_json::from_value(args)?;
    let result = tokio::runtime::Handle::current().block_on(list_ports_logic(query))?;
    Ok(serde_json::to_value(result)?)
}

pub fn register(r: &mut CommandRegistry) {
    r.register("list_ports", list_ports_dispatch);
    r.register("kill_port", kill_port_dispatch);
    r.register("kill_by_process_name", kill_by_process_name_dispatch);
}
```

**Trade-off**：每个命令模块要写 `xxx_dispatch` wrapper，模板代码。**接受**（v0.2 范围；v0.3+ 再考虑 codegen 反向）。

**`src-tauri/src/lib.rs`**（调整）：

```rust
// invoke_handler 只挂 dispatch + 框架级 system commands
.invoke_handler(tauri::generate_handler![
    cmd::dispatch::dispatch,
    cmd::capabilities::list_capabilities,
    cmd::windows::open_tool_window,
    cmd::windows::close_tool_window,
    cmd::quick_switcher::open_quick_switcher,
    cmd::settings::get_settings,
    cmd::settings::set_settings,
    cmd::settings::set_recording_mode,
    crate::windows_hook::get_hook_diagnostics,
])
```

**plugin 自带的命令（如 ports / env）不再出现在 `invoke_handler!` 里**，全部走 dispatch 路由。

### 3.8 API 自动生成

`scripts/codegen-api.ts`（新建，Node 跑）：

```typescript
// 扫 plugins/*/plugin.json
// 对每个 commands[] 里的条目，生成 wrapper：
//   <pluginId>Api.<camelName>(args: <argsRef> = {}): Promise<<returnsRef>>
//   → invoke<<returnsRef>>('dispatch', { name, args })
// 输出：src/plugins/api.gen.ts（被 gitignore）
```

**调用时机**：
- dev：`npm run dev` 前自动跑（用 Vite 插件钩子或 prebuild）
- build：`npm run build` 自动跑
- 手动：`npm run codegen`

**plugin.json 改了但没跑 codegen 会怎样**：
- TS 编译过（api.gen.ts 没变），但运行时调用新命令会 404 → **不强制，靠开发者习惯**
- **v0.2 接受这个，v0.3+ 加 Vite 插件在 plugin.json 改变时自动重生成**

**`api.gen.ts` 不进 git**（`.gitignore` 加一行）—— 因为它能完全从 plugin.json + types.ts 推导。

但这有个问题：**新拉仓库的开发者必须先跑 `npm run codegen` 才能 dev**。
**方案**：把 codegen 写进 `npm run dev` 和 `npm run build` 的前置脚本（`predev` / `prebuild`）。

### 3.9 capabilities 与 dispatch 的关系

v0.2 **不强制** capabilities 检查（plugin.json 里的 capabilities 数组仅是文档/元数据）。

v0.3+ 引入：
- Rust 端给每个命令标注需要哪个 capability（`#[capability("network:read")]` 风格）
- dispatch 时检查调用方的 capability 列表（从 context 里来）
- 无权调用 → 403

**v0.2 把 capabilities 字段加进 schema 但不实现检查**，避免过度工程。

---

## 4. 文件改动清单

### 4.1 新增

| 文件 | 用途 |
|---|---|
| `plugins/port-manager/plugin.json` | port-manager manifest |
| `plugins/port-manager/index.tsx` | port-manager 入口 |
| `plugins/port-manager/PortView.tsx` | （从 src/plugins/builtin/port-manager/ 迁过来） |
| `plugins/env-editor/plugin.json` | env-editor manifest |
| `plugins/env-editor/index.tsx` | env-editor 入口 |
| `plugins/env-editor/EnvEditorView.tsx` | （迁移） |
| `src-tauri/src/cmd/dispatch.rs` | CommandRegistry + dispatch command |
| `scripts/codegen-api.ts` | api.gen.ts 生成器 |
| `src/plugins/api.gen.ts` | 自动生成（.gitignore） |
| `tests/plugin-loader.test.ts` | 集成测试（见 §5.3） |

### 4.2 修改

| 文件 | 改动 |
|---|---|
| `src/plugins/builtin/index.ts` | 替换为 glob 自动注册 |
| `src/plugins/types.ts` | `PluginManifest.icon` 类型改为 `string \| ComponentType`（兼容两种来源） |
| `src/plugins/api.ts` | 删除（或保留为 re-export `from './api.gen'`） |
| `src/types.ts` | 保留手写类型 |
| `src-tauri/src/lib.rs` | `invoke_handler!` 列表删除 ports / env 两条，其余不动 |
| `src-tauri/src/cmd/mod.rs` | + `pub mod dispatch;` |
| `src-tauri/src/cmd/ports.rs` | 加 `list_ports_dispatch` 等 wrapper + `pub fn register` |
| `src-tauri/src/cmd/env.rs` | 同上 |
| `src-tauri/src/cmd/capabilities.rs` | + 接受 dispatch 注册的版本（如果 list_capabilities 也走 dispatch） |
| `vite.config.ts` | 加 `codegen-api` 钩子（prebuild 跑） |
| `package.json` | 加 `predev` / `prebuild` / `codegen` 脚本 |
| `.gitignore` | + `src/plugins/api.gen.ts` |
| `tsconfig.json` | 确认 `src/plugins/api.gen.ts` 包含在编译范围（虽然 gitignore，但本地存在） |
| `src/components/Sidebar.tsx` | icon 解析兼容字符串（用 lucide 动态查表） |
| `src/components/Launcher.tsx` | 同上 |
| `src/components/QuickSwitcher.tsx` | 同上 |

### 4.3 删除

| 文件 | 原因 |
|---|---|
| `src/plugins/builtin/port-manager/` | 迁到 `plugins/port-manager/` |
| `src/plugins/builtin/env-editor/` | 迁到 `plugins/env-editor/` |

### 4.4 仍需改的（Tauri 硬约束，接受）

| 文件 | 触发条件 |
|---|---|
| `src-tauri/Cargo.toml` | 仅当新插件引入新 tauri-plugin crate |
| `src-tauri/capabilities/default.json` | 同上 |

---

## 5. Todo 清单

### P0 — 基础设施（必须先做）

- [ ] **P0-1** 创建 `plugins/.gitkeep`，建立新目录
- [ ] **P0-2** 写 `scripts/codegen-api.ts`（读 plugin.json → 写 api.gen.ts）
- [ ] **P0-3** `package.json` 加 `predev` / `prebuild` / `codegen` 三个 script
- [ ] **P0-4** `.gitignore` 加 `src/plugins/api.gen.ts`
- [ ] **P0-5** `src/plugins/builtin/index.ts` 改成 `import.meta.glob` 收口
- [ ] **P0-6** `src/plugins/types.ts`：`PluginManifest.icon` 类型放宽到 `string | ComponentType`
- [ ] **P0-7** `src/components/Sidebar.tsx` / `Launcher.tsx` / `QuickSwitcher.tsx`：写 `resolveIcon(name: string | LucideIcon)` 工具函数
- [ ] **P0-8** `src-tauri/src/cmd/dispatch.rs` 新建：CommandRegistry + dispatch command
- [ ] **P0-9** `src-tauri/src/cmd/mod.rs` + `pub mod dispatch;`
- [ ] **P0-10** `src-tauri/src/cmd/ports.rs`：拆 `xxx_dispatch` wrapper + `pub fn register`
- [ ] **P0-11** `src-tauri/src/cmd/env.rs`：同上
- [ ] **P0-12** `src-tauri/src/cmd/capabilities.rs`：决定 `list_capabilities` 走 dispatch 还是保留 invoke_handler（推荐保留，因为它返回的是元数据）
- [ ] **P0-13** `src-tauri/src/lib.rs`：`invoke_handler!` 列表删除 ports / env 条目

### P1 — 迁移现有插件

- [ ] **P1-1** 创建 `plugins/port-manager/plugin.json`（从 [src/plugins/builtin/port-manager/index.ts](src/plugins/builtin/port-manager/index.ts) 的 manifest 常量翻译）
- [ ] **P1-2** 创建 `plugins/port-manager/index.tsx`（移植 manifest + PortView 引用 + 字符串 icon 映射）
- [ ] **P1-3** 创建 `plugins/port-manager/PortView.tsx`（从 [src/plugins/builtin/port-manager/PortView.tsx](src/plugins/builtin/port-manager/PortView.tsx) 迁过来，调整 import 路径）
- [ ] **P1-4** 创建 `plugins/port-manager/ProcessPickerDialog.tsx`（同上）
- [ ] **P1-5** 重复 P1-1 ~ P1-4 给 env-editor
- [ ] **P1-6** 删除 `src/plugins/builtin/port-manager/` 整个目录
- [ ] **P1-7** 删除 `src/plugins/builtin/env-editor/` 整个目录
- [ ] **P1-8** 跑 `npm run codegen` 生成 `api.gen.ts`
- [ ] **P1-9** 验证：`npm run dev` 能正常启动，port-manager 和 env-editor 都出现
- [ ] **P1-10** 验证：`cd src-tauri && cargo check` 无错
- [ ] **P1-11** 验证：`npx tsc --noEmit` 无错

### P2 — 验证「加新工具 = 1 个文件夹」

- [ ] **P2-1** 写一个空插件 `plugins/hello/`，plugin.json + index.tsx（仅显示 "Hello"）
- [ ] **P2-2** 验证：Sidebar / Launcher / QuickSwitcher 自动出现这个插件
- [ ] **P2-3** 写一个需要后端命令的 `plugins/echo/`，plugin.json 含 commands 数组 + index.tsx 调 `echoApi.echo("hi")`
- [ ] **P2-4** 在 `src-tauri/src/cmd/echo.rs` 写一个 echo 命令 + register
- [ ] **P2-5** 在 `src-tauri/src/cmd/mod.rs` + `src-tauri/src/cmd/dispatch.rs` 各加 1 行
- [ ] **P2-6** 跑 `npm run codegen` + `npm run dev`，验证 echo 插件工作
- [ ] **P2-7** 记录：「加 echo 插件改了哪些文件」，验证清单与 §2 的承诺一致（5 处）

### P3 — 收尾

- [ ] **P3-1** 写 `tests/plugin-loader.test.ts`：扫 `plugins/*/plugin.json`，验证每个 id 唯一、必填字段齐全
- [ ] **P3-2** 写 `docs/plugin-author-guide.md`：第三方作者怎么加新插件
- [ ] **P3-3** 跑 `npm run build` + `cd src-tauri && cargo test`，全绿
- [ ] **P3-4** commit + 开 PR

---

## 6. 验证标准

完成 P0-P3 后必须满足：

1. **CLI 验证**：
   - `npm run dev` 启动，UI 出现 port-manager + env-editor 两个 tile
   - port-manager 能列端口（启个 `python -m http.server` 测）
   - env-editor 能读 / 写用户环境变量

2. **加新插件验证**：
   - 按 §5 P2-1 ~ P2-7 加 `plugins/hello/` 和 `plugins/echo/`
   - 全程不碰 `lib.rs` / `app.tsx` / `Sidebar.tsx` / `api.ts` / `types.ts`
   - 改动文件清单 = 5 处（4 个新文件 + 2 个 Rust 文件 +1 行 mod.rs + 1 行 dispatch.rs）

3. **回归验证**：
   - `npx tsc --noEmit` 通过
   - `cd src-tauri && cargo check` 通过
   - `cd src-tauri && cargo test` 全绿

4. **代码 review 检查**：
   - `src/plugins/api.ts` 不再手写（要么删除，要么 re-export from api.gen）
   - `src/plugins/builtin/index.ts` 不再手写 import（用 glob）
   - `lib.rs` 的 `invoke_handler!` 列表只包含 dispatch + system commands

---

## 7. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| `import.meta.glob` 路径在 Vite build 后行为不同 | dev OK，build 失败 | P1-9 跑 `npm run build` 验证 |
| codegen 脚本写错，`api.gen.ts` 编译失败 | TS 编译报错 | P0-3 把 codegen 绑到 prebuild |
| 旧插件 manifest 直接 import 失败 | dev 启动后某些插件不显示 | 迁移时改 P1-2 步骤同时调整 index.tsx 的 icon 字符串映射 |
| Tauri 端 dispatch 路由表需要 `tokio` runtime | 同步 handler vs 异步 handler | 选同步 handler（v0.2 命令都是 CPU-bound 或短时 IO，必要时用 `block_on`） |
| plugin.json 没有 schema 校验 | 写错字段运行时崩 | 暂不加 zod 依赖；v0.3+ 再加 |

---

## 8. 不在 v0.2 范围（明确划线）

- ❌ 运行时 sandbox（iframe / WebView 隔离）
- ❌ 用户写 .tsx → vite build → 运行时加载
- ❌ JSON UI 描述
- ❌ Marketplace / 安装 / 签名
- ❌ plugin.json 的 capabilities 强制检查
- ❌ 动态加载（编译期 glob 已经够用）
- ❌ 卸载 / 禁用插件
- ❌ codegen 反向（Rust → TS 类型生成）
