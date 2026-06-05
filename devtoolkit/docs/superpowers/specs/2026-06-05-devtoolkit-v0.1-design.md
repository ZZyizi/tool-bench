# DevToolkit V0.1 MVP 设计规格

**日期**: 2026-06-05
**项目**: DevToolkit (轻量级开发者桌面工具箱)
**里程碑**: v0.1.0 (P0 MVP)

---

## 1. 目标

实现 PRD §7.1 定义的 v0.1.0 MVP：
- [ ] 端口占用列表展示
- [ ] 端口占用清除
- [ ] 基础 UI 框架
- [ ] 扩展接口定义（Tool trait + ToolRegistry）

## 2. 已确认的关键决策

| 维度 | 决策 |
|------|------|
| 实施范围 | 完整 P0 MVP |
| 前端样式 | 原生 CSS + CSS Variables（深色主题） |
| 端口扫描实现 | 解析系统命令（netstat/lsof/ss） |
| 端口列表刷新 | 手动刷新按钮 |
| 后端架构 | Tool trait + 每工具独立 Tauri command |
| 前端架构 | useState + 组件分层 |
| 测试覆盖 | Rust 单元测试（netstat 解析器、错误处理） |

## 3. 整体架构

严格遵循 PRD §4.2 的四层架构：

```
┌─────────────────────────────────────────────────────────┐
│              UI Layer (React, src/)                     │
│  App  →  Sidebar  +  Content  →  PortView/PortKill      │
├─────────────────────────────────────────────────────────┤
│              Tauri IPC Bridge (@tauri-apps/api)         │
│  invoke('list_ports'), invoke('kill_port', {port})     │
├─────────────────────────────────────────────────────────┤
│              Command Layer (Rust, src-tauri/src/cmd/)   │
│  pub fn list_ports() / pub fn kill_port(port: u16)      │
├─────────────────────────────────────────────────────────┤
│              Tool Interface (src-tauri/src/tool/)       │
│  trait Tool + struct ToolRegistry                      │
├─────────────────────────────────────────────────────────┤
│              Core / Tool Implementation                 │
│  PortTool (跨平台)                                     │
├─────────────────────────────────────────────────────────┤
│              Platform Layer (src-tauri/src/platform/)   │
│  PortScanner trait → WindowsPortScanner / UnixPortScanner│
└─────────────────────────────────────────────────────────┘
```

## 4. Rust 后端设计

### 4.1 模块划分

```
src-tauri/src/
├── main.rs                  # Tauri 启动入口
├── lib.rs                   # run() 入口
├── cmd/                     # Tauri command 层
│   ├── mod.rs
│   ├── ports.rs            # list_ports, kill_port
│   └── tools.rs            # list_tools (V0.2 用, V0.1 仅占位)
├── tool/                    # Tool trait 与 Registry
│   ├── mod.rs
│   └── registry.rs
├── tools/                   # 具体 Tool 实现
│   ├── mod.rs
│   └── port.rs             # PortTool
└── platform/                # 平台抽象
    ├── mod.rs
    ├── port_scanner.rs     # PortScanner trait
    ├── windows.rs          # #[cfg(windows)] WindowsPortScanner
    └── unix.rs             # #[cfg(unix)] UnixPortScanner
```

### 4.2 核心 trait

```rust
// src-tauri/src/tool/mod.rs
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn category(&self) -> ToolCategory;
}

#[derive(Debug, Clone, Serialize)]
pub enum ToolCategory {
    Port,
    Network,
    Encode,
}

// src-tauri/src/tool/registry.rs
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self { ... }
    pub fn register(&mut self, tool: Box<dyn Tool>) { ... }
    pub fn list(&self) -> Vec<&dyn Tool> { ... }
    pub fn get(&self, name: &str) -> Option<&dyn Tool> { ... }
}
```

### 4.3 平台抽象

```rust
// src-tauri/src/platform/port_scanner.rs
pub struct PortInfo {
    pub protocol: Protocol,    // TCP / UDP
    pub port: u16,
    pub pid: u32,
    pub state: String,         // LISTEN / ESTABLISHED / ...
    pub process_name: Option<String>,
}

pub enum Protocol { Tcp, Udp }

pub trait PortScanner: Send + Sync {
    fn list(&self) -> Result<Vec<PortInfo>, PortError>;
    fn kill(&self, pid: u32) -> Result<(), PortError>;
}
```

### 4.4 平台实现

**Windows** (`platform/windows.rs`):
- `list()`: 解析 `netstat -ano` 输出，格式 `协议 本地地址 外部地址 状态 PID`
- `kill(pid)`: 调用 `taskkill /PID <pid> /F`
- 进程名通过 `tasklist /FI "PID eq <pid>"` 或 `GetModuleBaseNameW` 获取（V0.1 简化用 tasklist）

**Unix** (`platform/unix.rs`):
- `list()`: 解析 `lsof -i -P -n` 输出（macOS 与 Linux 通用）
- `kill(pid)`: 调用 `kill -9 <pid>`
- 进程名由 lsof 直接提供

### 4.5 Tauri command

```rust
// src-tauri/src/cmd/ports.rs
#[tauri::command]
pub fn list_ports(state: tauri::State<AppState>) -> Result<Vec<PortInfo>, String> {
    state.scanner.list().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn kill_port(port: u16, state: tauri::State<AppState>) -> Result<KillResult, String> {
    // 1. 找到占用该 port 的 PID
    // 2. 二次确认在前端做；后端只接收执行命令
    // 3. 调用 scanner.kill(pid)
    // 4. 返回结果（成功/失败/原因）
}

#[derive(Serialize)]
pub struct KillResult {
    pub success: bool,
    pub pid: u32,
    pub port: u16,
    pub message: String,
}
```

### 4.6 AppState

```rust
pub struct AppState {
    pub registry: ToolRegistry,
    pub scanner: Arc<dyn PortScanner>,
}
```

### 4.7 错误处理

定义 `PortError` 枚举，覆盖：
- `CommandFailed(String)` - 子进程退出非 0
- `ParseError(String)` - 输出解析失败
- `PermissionDenied` - kill 权限不足
- `NotFound` - 端口未找到

通过 `thiserror` 派生 `Error` trait。

### 4.8 依赖（新增）

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"           # 新增
```

## 5. 前端设计

### 5.1 组件结构

```
src/
├── main.tsx                 # React 根
├── App.tsx                  # 布局壳：Sidebar + Content
├── App.css                  # 全局样式 + CSS 变量（深色主题）
├── components/
│   ├── Sidebar.tsx         # 工具列表（V0.1 硬编码 1 个：端口管理）
│   ├── Sidebar.css
│   ├── PortView.tsx        # 端口列表视图
│   └── PortView.css
└── types.ts                # PortInfo / KillResult TS 类型
```

### 5.2 状态管理

- `App.tsx`: `useState<ToolKey>('port-view')` 管理当前选中工具
- `PortView.tsx`:
  - `useState<PortInfo[]>([])` 端口列表
  - `useState<boolean>(false)` loading
  - `useState<string|null>(null)` error
  - `useState<PortInfo|null>(null)` 选中行
  - `useState<boolean>(false)` 二次确认对话框
- 刷新按钮触发 `invoke('list_ports')`
- 释放按钮触发 `invoke('kill_port', { port })`

### 5.3 数据流

```
用户点击"刷新"
  → PortView 调用 invoke('list_ports')
  → Rust list_ports() → WindowsPortScanner.list() → 解析 netstat -ano
  → 返回 Vec<PortInfo> → 序列化 JSON
  → 前端 setState(render table)

用户点击"释放"某行
  → setSelected(row) + setConfirmDialog(true)
  → 用户确认 → invoke('kill_port', { port })
  → Rust kill_port() → 找 PID → taskkill /F
  → 返回 KillResult → toast / 刷新列表
```

### 5.4 样式规范

- `App.css` 定义 CSS 变量：`--bg`, `--fg`, `--accent`, `--border`, `--row-hover` 等
- 深色主题（PRD §5.1）
- 等宽字体（PRD §5.1）展示端口、PID
- 单页布局，零弹窗（除二次确认）

## 6. 关键交互细节

| 操作 | 流程 |
|------|------|
| 启动 | 立即展示端口列表（loading → data） |
| 搜索 | V0.1 暂不实现（PRD v0.2 内容） |
| 选中行 | 行高亮，底部展示进程详情 + 释放按钮 |
| 释放端口 | 弹窗二次确认 → 后端 kill → toast 显示结果 → 自动刷新 |
| 错误 | 行内 / 顶部状态栏展示 |

## 7. 测试策略

### 7.1 Rust 单元测试

- `platform/windows.rs::parse_netstat`: 给定 mock netstat 输出，断言 PortInfo
- `platform/unix.rs::parse_lsof`: 给定 mock lsof 输出，断言 PortInfo
- `tools/port.rs::PortTool::name/description/category`: 基础断言
- `tool/registry.rs`: register / get / list 行为

### 7.2 手动验证

- `npm run tauri dev` 启动应用
- 检查端口列表渲染
- 启动一个测试服务（如 `python -m http.server 8000`）
- 在 UI 中刷新，验证出现 8000 端口
- 选中并释放，验证端口消失
- 释放系统关键进程，确认错误处理友好

## 8. 验收标准

- [ ] `cargo test` 全绿
- [ ] `npm run tauri dev` 启动后能看到当前系统所有 TCP/UDP 占用端口
- [ ] 端口列表包含字段：协议、端口、PID、进程名、状态
- [ ] 释放端口有二次确认，确认后能成功 kill
- [ ] 释放端口后刷新，端口从列表消失
- [ ] kill 失败时显示友好错误（如权限不足）
- [ ] UI 深色主题，无明显样式瑕疵
- [ ] `Tool` trait 与 `ToolRegistry` 已定义，添加新工具只需注册无需改 command 层

## 9. 范围外（V0.1 不做）

- 搜索过滤（PRD v0.2）
- 设置面板 / 主题切换（PRD v0.2）
- 自动刷新（V0.1 手动）
- 网络检测、DNS、编解码等工具（PRD v1.0）
- 单元测试以外的前端测试
- 打包发布

## 10. 风险与缓解

| 风险 | 缓解 |
|------|------|
| `netstat -ano` 中文 Windows 输出编码问题 | 用 `Command::new("cmd").args(&["/C", "chcp 65001 > nul && netstat -ano"])` |
| 进程名获取权限不足 | 接受 Option，None 时显示 PID only |
| kill 系统关键进程 | 二次确认 + 显示进程信息 + 失败时友好提示 |
| 跨平台 netstat 输出格式差异 | 通过 cfg 隔离，每个平台独立测试 |
