# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`cm`은 AI 에이전트 팀(Claude, Codex)을 tmux 세션으로 오케스트레이션하는 Rust CLI + 웹 대시보드. 단일 바이너리, DB 없음 — 모든 상태는 tmux 세션과 `~/.crewmux/` 하위 JSON 메타데이터 파일에 저장.

## Build & Development Commands

```bash
cargo build                    # Debug build
cargo build --release          # Release build → target/release/cm
cargo fmt                      # Format code (required by CI)
cargo clippy --all-targets --all-features -- -D warnings  # Lint (must pass with zero warnings)
cargo test                     # Run tests
bash -n install.sh             # Validate install script syntax
```

Install locally:
```bash
cp target/release/cm ~/.local/bin/cm
```

## Architecture

### Module Structure

```
src/
├── main.rs          # Entry point, clap CLI routing to 6 command families
├── cmd/
│   ├── team.rs      # Session lifecycle (start/stop/list/attach)
│   ├── task.rs      # Worker spawning and task dispatch
│   ├── ctl.rs       # Monitoring and control (status/peek/send/broadcast/interrupt)
│   └── service.rs   # macOS launchd integration (install/uninstall)
├── meta.rs          # JSON metadata management (TeamMeta, PaneMeta, WorkerMeta)
├── agent.rs         # CLI command builder for claude/codex agents + trust config
├── prompt.rs        # Master orchestration prompt bootstrap/versioning
├── tmux.rs          # Low-level tmux subprocess wrapper
└── web/mod.rs       # axum REST API server + embedded SPA dashboard (static/index.html)
```

### State Model

No external database. Two sources of truth:
1. **Live tmux sessions** — actual process runtime
2. **`~/.crewmux/tasks/<session>/meta.json`** — metadata (pane IDs, worker names, models)

CLI, Web API, and background service all read/write the same files.

### Key Design Patterns

- **Session naming**: `cm-<directory-basename>` with legacy `ai-*` fallback
- **Worker naming**: `<type>-<number>` (e.g., `claude-1`, `codex-2`), auto-incremented, supports partial matching
- **Layout**: `main-vertical` for 1-2 workers, `tiled` for 3+, auto-adjusted on spawn/kill
- **Agent trust**: Claude modifies `~/.claude.json` (JSON), Codex modifies `~/.codex/config.toml` (TOML)
- **Error handling**: `anyhow::Result<T>` throughout, bail with user-friendly messages
- **Web UI**: Single-file React/TS SPA embedded via `include_str!`, polling-based (no WebSocket)

### Web API

13 REST endpoints served by axum on port 7700 (default):
- Session management: `/api/sessions`, `/api/sessions/create`, `/api/sessions/:session`
- Status/peek: `/api/status/:session`, `/api/peek/:session/:target`
- Agent control: `/api/send`, `/api/spawn`, `/api/interrupt`, `/api/kill-workers`, `/api/kill-agent`
- Directory browsing: `/api/browse`, `/api/recents`
- Brand assets embedded via `rust-embed`

## CI

GitHub Actions (`.github/workflows/ci.yml`): matrix build on ubuntu-latest + macos-latest running fmt, test, clippy, release build, and install.sh syntax check.

## Conventions

- 바이너리 및 명령어는 `cm`으로 통일 (`ai`는 레거시 alias)
- All timestamps use ISO 8601 via `chrono`
- Log format: `[HH:MM:SS] EVENT_TYPE [target] message` (append-only files)
- Master prompt lives in `assets/master-prompt.md` (embedded at compile time), user-customizable at `~/.crewmux/master-prompt.md`
- macOS-only for service/launchd features; tmux required on all platforms
