# 插件作者指南

本指南介绍如何往 DevToolkit 加一个新的内置工具（插件）。

> 设计背景与权衡见 [plugin-loader-phase1.md](./plugin-loader-phase1.md)。
> 范围说明：本指南只覆盖**内置**插件（与主仓库一起编译）。第三方动态加载是 v0.3+ 的事。

---

## TL;DR — 5 处改动

| # | 文件 | 类型 | 内容 |
|---|---|---|---|
| 1 | `plugins/<id>/plugin.json` | 新建 | manifest |
| 2 | `plugins/<id>/index.tsx` | 新建 | 入口 + icon 映射 |
| 3 | `src-tauri/src/cmd/<id>.rs` | 新建 | 后端命令实现（如需要） |
| 4 | `src-tauri/src/cmd/mod.rs` | +1 行 | `pub mod <id>;` |
| 5 | `src-tauri/src/cmd/dispatch.rs` | +1 行 | `super::<id>::register(&mut r);` |

只用前端时可省略 3、4、5。
新类型需要扩 `src/types.ts`（不在上表，因为只此一处且可读性更好）。

---

## 1. 起步：纯前端插件

最小例子：一个只显示 "Hello" 的工具。

### 1.1 创建目录

```
plugins/hello/
├── plugin.json
└── index.tsx
```

### 1.2 `plugin.json`

```json
{
  "id": "hello",
  "name": "Hello",
  "version": "0.1.0",
  "description": "示例插件",
  "author": "Your Name",
  "category": "Other",
  "icon": "Hand",
  "entry": "./index.tsx",
  "capabilities": [],
  "windowWidth": 480,
  "windowHeight": 320,
  "commands": []
}
```

字段说明：

- `id` — kebab-case，全局唯一，会变成 Tauri 窗口 label 的一部分
- `category` — 必须是 `Network` / `Encode` / `System` / `Other` 之一
- `icon` — `lucide-react` 的图标名（字符串）。在 `index.tsx` 里映射成组件
- `capabilities` — 元数据；v0.2 不做强制检查
- `windowWidth` / `windowHeight` — 可选，覆盖 ToolWindow 默认尺寸
- `commands` — 后端命令清单。无后端时填 `[]`

### 1.3 `index.tsx`

```tsx
import { Hand } from 'lucide-react';
import type { Plugin } from '../../src/plugins/types';
import manifestRaw from './plugin.json';

const manifest = {
  ...manifestRaw,
  icon: Hand,
} as const;

function HelloView() {
  return <div style={{ padding: 16 }}>Hello, world!</div>;
}

export const helloPlugin: Plugin = {
  manifest,
  Component: HelloView,
  activate(ctx) {
    ctx.log('Hello activated');
  },
};

export default helloPlugin;
```

要点：

- **必须 `export default`** 一个 `Plugin` 对象。`src/plugins/builtin/index.ts` 的 Vite glob 靠 `default` 收口
- icon 字符串 → 组件的映射在这里手写。manifest 里的 `icon` 字段会被覆盖
- `activate(ctx)` 在 registry 注册时调用一次。`ctx.log` / `ctx.notify` / `ctx.invoke` 可用

### 1.4 运行

```bash
npm run tauri dev
```

Sidebar / Launcher / QuickSwitcher 会自动出现 "Hello" 项。**不需要**改任何注册表。

---

## 2. 进阶：带后端命令的插件

如果插件需要调用 Rust（读文件、跑 CLI、操纵系统状态等），在 `plugin.json` 里声明 commands，
然后写对应的 Rust 模块。

### 2.1 在 `plugin.json` 里声明命令

```json
{
  "id": "echo",
  ...
  "commands": [
    {
      "name": "echo",
      "argsRef": "EchoArgs",
      "returnsRef": "EchoResult"
    }
  ]
}
```

- `name` — 后端 dispatch 表里的 key，前端调用名
- `argsRef` / `returnsRef` — 引用 `src/types.ts` 里 export 的类型名
  - 不需要参数时写 `"void"`
  - 不返回数据时写 `"void"`

### 2.2 在 `src/types.ts` 加类型

```ts
// 入参 / 出参 一起加在文件末尾，靠近相关类型：
export interface EchoArgs {
  message: string;
}

export interface EchoResult {
  message: string;
}
```

只手写一次，codegen 会读这些类型生成前端 wrapper。

### 2.3 写 Rust 命令模块 `src-tauri/src/cmd/echo.rs`

```rust
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub struct EchoResult {
    pub message: String,
}

pub fn echo_inner(message: &str) -> EchoResult {
    EchoResult { message: message.to_string() }
}

pub fn register(r: &mut super::dispatch::CommandRegistry) {
    r.register("echo", |args: Value| -> Result<Value, String> {
        let m = args
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing or non-string 'message'".to_string())?;
        let result = echo_inner(m);
        serde_json::to_value(result).map_err(|e| e.to_string())
    });
}
```

约定：

- 业务逻辑放在 `_inner` 函数里（纯函数，方便写单元测试）
- 公开 `pub fn register(r: &mut CommandRegistry)`，把 closure 挂到 dispatch 表
- closure 自己负责解 args、调 `_inner`、序列化结果。复用 `parse_args::<T>(args)` 帮你解 JSON

### 2.4 把模块挂到 dispatch 表（2 处 +1 行）

```rust
// src-tauri/src/cmd/mod.rs
pub mod echo;
```

```rust
// src-tauri/src/cmd/dispatch.rs — build_registry()
pub fn build_registry(app_state: &crate::AppState) -> CommandRegistry {
    let mut r = CommandRegistry::new();
    super::ports::register(&mut r, app_state.scanner.clone());
    super::env::register(&mut r);
    super::echo::register(&mut r);   // ← 加这一行
    r
}
```

### 2.5 前端调用

```tsx
import { echoApi } from '../../src/plugins/api.gen';

const r = await echoApi.echo({ message: 'hi' });
// r.message === 'hi'
```

`api.gen.ts` 由 `npm run codegen` 自动生成；`predev` / `prebuild` 已经把它串进流程，所以
正常 `npm run tauri dev` 时不用手动跑。

---

## 3. 调试与排错

### 3.1 manifest 字段写错

```bash
$ npm run validate-plugins
[validate-plugins] plugins/foo/plugin.json: id "Foo Bar" must be kebab-case (a-z, 0-9, '-')
```

`validate-plugins` 在每次 `npm run codegen` 前自动跑，第一时间报错。

### 3.2 codegen 没生成新命令的 wrapper

```bash
npm run codegen
```

或确认 `predev` / `prebuild` 没被跳过。`src/plugins/api.gen.ts` 在 `.gitignore` 里，
本地不存在时一次 codegen 就能补出。

### 3.3 前端报 `unknown command: xxx`

后端没注册：检查 `cmd/<name>.rs::register` 是否被 `dispatch.rs::build_registry` 调用。

### 3.4 cargo check 报 `couldn't convert serde_json::Error to String`

把 `serde_json::from_value(args)?` 换成 `dispatch::parse_args::<T>(args)?`。

### 3.5 插件不出现在 Sidebar

- 检查 `index.tsx` 有没有 `export default plugin`
- 检查 `plugin.json` 的 `category` 是不是合法值
- 检查浏览器 console，Vite glob 会日志报告加载失败的模块

---

## 4. 约定与底线

- **图标**：优先用 `lucide-react`。需要自定义图标时，在 `index.tsx` import 自己的 React 组件赋给 `manifest.icon`
- **`_inner` 纯函数**：Rust 业务逻辑必须可单元测试。dispatch closure 只做 JSON 编解码
- **类型**：所有跨边界的 args / result 必须在 `src/types.ts` export，并被 `plugin.json` 引用
- **不要**直接 `import './styles.css'` 全局污染。每个插件的样式放在自己目录里、用 CSS module 或 inline style
- **不要**改 `lib.rs` 的 `invoke_handler!` 列表 —— plugin command 全部走 dispatch

---

## 5. 检查清单

加完一个带后端的插件后，本地跑：

```bash
npm run validate-plugins   # manifest 校验
npm run codegen            # 生成 api.gen.ts
npx tsc --noEmit           # 前端类型检查
cd src-tauri && cargo test # 后端测试
npm run tauri dev          # 手工冒烟
```

全绿 = 可以 PR。
