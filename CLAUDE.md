# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

你全局使用中文回答

## Project: DevToolkit

A lightweight desktop toolbox for developers, built on **Tauri 2 + React 19 + TypeScript 5.8 + Vite 7**. Targets Windows / macOS / Linux, < 1s startup, < 50 MB RAM, fully offline.

- **Package naming** (use these exact names — they're referenced in many places):
  - npm package: `tool-bench` ([package.json](package.json))
  - Rust crate: `toolBench` ([src-tauri/Cargo.toml](src-tauri/Cargo.toml))
  - Rust lib: `tool_bench_lib` (called from [src-tauri/src/main.rs](src-tauri/src/main.rs))
  - Tauri identifier: `com.toolBench.app` ([src-tauri/tauri.conf.json](src-tauri/tauri.conf.json))
  - Display name: `toolBench`

## Active Milestone: v0.1 MVP

Per [PRD §7.1](PRD.md) and the v0.1 spec at [docs/superpowers/specs/2026-06-05-devtoolkit-v0.1-design.md](docs/superpowers/specs/2026-06-05-devtoolkit-v0.1-design.md), the goal is port list + port kill + base UI + plugin infrastructure (port-manager as the first built-in plugin).

- **Source of truth for in-progress work**: [docs/superpowers/plans/2026-06-05-devtoolkit-v0.1.md](docs/superpowers/plans/2026-06-05-devtoolkit-v0.1.md) — 19 tasks, TDD-flavored, with checkboxes. Check it before adding new files.
- **Active implementation branch**: `v0.1` is being developed in a git worktree at `.worktrees/v0.1/` (see `superpowers:using-git-worktrees`). `main` currently holds the unmodified Tauri scaffold.

## Development Commands

All frontend commands run from the **project root**. Rust commands run from `src-tauri/`.

### Frontend
- `npm run dev` — Vite dev server on port 1420 (Tauri-fixed; do not change)
- `npm run build` — `tsc && vite build` (type-check + bundle)
- `npm run preview` — preview built bundle
- `npx tsc --noEmit` — type-check only

### Tauri / Rust
- `npm run tauri` — `tauri dev` (boots Vite + native window, ~10s startup)
- `cd src-tauri && cargo check` — fast type-check
- `cd src-tauri && cargo test` — run all unit tests
- `cd src-tauri && cargo test --lib platform::windows::tests` — run a single test module

### Manual verification (per plan Task 19)
```bash
python -m http.server 8765   # in a separate terminal
npm run tauri dev            # then verify 8765 appears in the port table
```

## Architecture: Two-Layer Responsibility

```
┌─────────────────────────────────────────────────────────┐
│  Frontend (React, src/)                                 │
│   ├── Plugin System (Plugin / PluginManifest /         │
│   │                  PluginRegistry / PluginContext)   │
│   │     Built-in: port-manager (V0.1 only plugin)       │
│   └── System API Bridge → invoke('list_ports', ...)     │
└─────────────────────────┬───────────────────────────────┘
                          │ IPC
┌─────────────────────────┴───────────────────────────────┐
│  Backend (Rust, src-tauri/src/)                         │
│   └── System API Layer (Tauri commands)                │
│         ↓ Platform Layer (PortScanner trait)            │
│           Windows: netstat  │  Unix: lsof              │
└─────────────────────────────────────────────────────────┘
```

**Key insight**: Rust is a thin "system capability provider" — it does NOT define a Tool abstraction. All Tool/Plugin logic lives in the frontend. Third-party plugin authors write JS/TS and call system capabilities via `invoke()`.

## File Layout (target per v0.1 plan)

```
src/
├── main.tsx, App.tsx, App.css          # layout shell, data-driven
├── types.ts                            # shared TS types (PortInfo, KillResult)
├── components/
│   ├── Sidebar.tsx                     # renders plugins from registry
│   ├── StatusBar.tsx
│   └── ConfirmDialog.tsx               # 二次确认弹窗 for kill
├── plugins/
│   ├── types.ts                        # Plugin, PluginManifest, PluginContext
│   ├── registry.ts                     # PluginRegistry + globalRegistry
│   ├── context.ts                      # createPluginContext()
│   ├── api.ts                          # typed wrappers over invoke()
│   └── builtin/
│       ├── index.ts                    # registers all built-in plugins
│       └── port-manager/
│           ├── plugin.toml             # [plugin] / [plugin.capabilities]
│           ├── index.ts                # Plugin export
│           ├── PortView.tsx
│           └── PortView.css

src-tauri/src/
├── main.rs                             # calls tool_bench_lib::run()
├── lib.rs                              # Tauri builder + AppState + handler registration
├── cmd/
│   ├── mod.rs
│   ├── ports.rs                        # list_ports, kill_port
│   └── capabilities.rs                 # list_capabilities
└── platform/
    ├── mod.rs                          # factory selects WindowsPortScanner / UnixPortScanner
    ├── port_scanner.rs                 # PortScanner trait, PortInfo, PortError
    ├── windows.rs                      # netstat parser + tests
    └── unix.rs                         # lsof parser + tests
```

**Module gating**: `platform/windows.rs` is `#[cfg(windows)]`, `platform/unix.rs` is `#[cfg(unix)]`. The trait is shared. Tauri commands in `cmd/` are platform-agnostic — they call into the factory.

## Plugin System Quick Reference

- **Manifest** ([plugin.toml](src/plugins/builtin/port-manager/plugin.toml) format): `id`, `name`, `version`, `description`, `author`, `category` (Network / Encode / System / Other), `icon`, `entry`, optional `capabilities` array. V0.1 stores manifests statically; V0.3+ scans filesystem.
- **V0.1 plugins are static imports** in [src/plugins/builtin/index.ts](src/plugins/builtin/index.ts). Dynamic loading is V0.3+.
- **Adding a second built-in plugin** (e.g. base64) requires: new folder under `src/plugins/builtin/<name>/` with `plugin.toml` + `index.ts` + `<Name>View.tsx`, plus **one line** in `builtin/index.ts`. No changes to `App.tsx`, `Sidebar.tsx`, or anything in `components/`. This is the acceptance test for the plugin infrastructure.
- **Sidebar is data-driven**: it iterates `globalRegistry.list()` grouped by category.

## Tauri Commands Exposed to Plugins

| Command | Args | Returns | Notes |
|---------|------|---------|-------|
| `list_ports` | — | `PortInfo[]` | uses platform factory; permission errors surface as `PortError::PermissionDenied` |
| `kill_port` | `port: u16` | `KillResult` | 二次确认 in UI; on backend, `kill(pid)` runs `taskkill /F` or `kill -9` |
| `list_capabilities` | — | `Capabilities` | declares what backend can do; V0.3+ uses this for plugin permission checks |

All command return types use `Result<T, String>` at the boundary (stringified error); internal errors are typed via `PortError` + `thiserror`.

## Conventions Specific to This Repo

- **PortInfo.process_name is `Option<String>`** — Windows netstat doesn't return it cheaply, so we accept `None` and the UI shows PID-only in that case.
- **Windows netstat invocation** uses `chcp 65001 > nul && netstat -ano` to handle localized text. Don't simplify this.
- **Permission errors** are detected by stderr substring match: `"Access is denied"` (Windows) / `"Operation not permitted"` (Unix). Map to `PortError::PermissionDenied` so the UI can show a friendly message.
- **No third-party Rust crates** beyond what's in Cargo.toml (tauri, tauri-plugin-opener, serde, serde_json, thiserror). Don't add a sysinfo/netstat2 crate — the design explicitly parses CLI output.
- **State management** is `useState` + component layering. No Redux/Zustand in V0.1.
- **Styling** is native CSS + CSS Variables (dark theme). No Tailwind/CSS-in-JS in V0.1.

## Testing Strategy

- **Rust unit tests** cover parsers: `parse_netstat` and `parse_lsof` (4 tests each, mock CLI output) plus `kill_port` edge cases. Run with `cd src-tauri && cargo test`.
- **Manual verification** of Tauri behavior is the only way to validate the IPC + UI flow. Plan Task 19 has the script.
- **No frontend test framework** in V0.1 — out of scope.

## Out of Scope for V0.1 (do not implement)

- Search / filter (V0.2)
- Auto-refresh (V0.1 uses a manual refresh button)
- Settings panel / theme switching
- Additional built-in tools beyond `port-manager`
- Dynamic plugin loading from filesystem (V0.3)
- Plugin marketplace / install / signature verification (V0.4)
- Plugin permission enforcement from `manifest.capabilities` (V0.3+)
- Production packaging / distribution

## Vite / Tauri Plumbing Notes

- Vite is locked to port 1420 (`strictPort: true` in [vite.config.ts](vite.config.ts)). Tauri expects this exact port.
- HMR uses port 1421 when `TAURI_DEV_HOST` is set.
- Vite's `watch.ignored` excludes `src-tauri/**` so backend changes don't trigger full frontend reloads (use `cargo` separately).
- Tauri capabilities (frontend permissions) live in [src-tauri/capabilities/default.json](src-tauri/capabilities/default.json). Add new permissions there when registering new plugins/commands.

## Resources

- PRD: [PRD.md](PRD.md) — product goals, user personas, feature priority
- v0.1 spec: [docs/superpowers/specs/2026-06-05-devtoolkit-v0.1-design.md](docs/superpowers/specs/2026-06-05-devtoolkit-v0.1-design.md) — full architecture, file manifest, acceptance criteria
- v0.1 plan: [docs/superpowers/plans/2026-06-05-devtoolkit-v0.1.md](docs/superpowers/plans/2026-06-05-devtoolkit-v0.1.md) — 19 tasks with exact code to write
- Tauri 2 docs: https://tauri.app/develop/
