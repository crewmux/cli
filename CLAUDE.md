# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

CrewMux (`crewmux`) orchestrates AI agent teams (Claude, Codex) via tmux sessions. Single Rust binary, no database — all state lives in tmux sessions and `~/.crewmux/` JSON metadata files. The binary is also symlinked as `cm` and `ai` for backward compatibility.

## Build & Development Commands

```bash
cargo build                    # Debug build
cargo build --release          # Release build → target/release/crewmux
cargo fmt                      # Format code (required by CI)
cargo clippy --all-targets --all-features -- -D warnings  # Lint (must pass with zero warnings)
cargo test                     # Run all tests
cargo test <test_name>         # Run a single test by name
bash -n install.sh             # Validate install script syntax
```

Install locally:
```bash
cp target/release/crewmux ~/.local/bin/crewmux
```

## Architecture

### Module Structure

```
src/
├── main.rs          # Entry: clap CLI routing to Team/Task/Ctl/Web/Install/Uninstall
├── cmd/
│   ├── team.rs      # Session lifecycle (start/stop/stop-all/list/attach)
│   ├── task.rs      # Worker spawning, task dispatch, message routing
│   ├── ctl.rs       # Monitoring/control (status/roles/peek/send/broadcast/interrupt/kill-workers)
│   └── service.rs   # macOS launchd integration (install/uninstall)
├── meta.rs          # JSON metadata (TeamMeta/PaneMeta/WorkerMeta), session naming, legacy fallback
├── agent.rs         # CLI command builder for claude/codex agents + trust config auto-setup
├── prompt.rs        # Master prompt bootstrap/versioning (embedded default + legacy migration)
├── tmux.rs          # Low-level tmux subprocess wrapper (all tmux interactions go through here)
└── web/mod.rs       # axum REST API + embedded SPA (static/index.html via include_str!)
```

### State Model

Two sources of truth, no external database:
1. **Live tmux sessions** — actual process runtime
2. **`~/.crewmux/tasks/<session>/meta.json`** — pane IDs, worker names, models, task counts

CLI, Web API, and background service all read/write the same metadata files. The `meta.rs` module handles all path resolution, load/save, and legacy fallback to `~/.ai-team/`.

### Key Design Patterns

- **Session naming**: `crewmux-<directory-basename>` with legacy `cm-*` / `ai-*` fallback via `resolve_session_name()`
- **Worker naming**: `<type>-<number>` (e.g., `claude-1`, `codex-2`), auto-incremented via `next_worker_name()`, supports partial matching in `resolve_pane()`/`resolve_worker()`
- **Layout**: `main-vertical` for ≤2 workers, `tiled` for 3+, auto-adjusted on spawn/kill
- **Agent trust**: Claude — modifies `~/.claude.json` (JSON, sets `hasTrustDialogAccepted` + project entry). Codex — modifies `~/.codex/config.toml` (TOML, sets `trust_level = "trusted"` per project)
- **Master prompt**: Embedded at compile time from `assets/master-prompt.md` via `include_str!`. User-customizable at `~/.crewmux/master-prompt.md`. Legacy prompts auto-backed up as `.legacy.bak`
- **Error handling**: `anyhow::Result<T>` everywhere, `bail!` with user-facing messages
- **Web UI**: Single-file React/TS SPA in `static/index.html`, embedded via `include_str!`. Polling-based (no WebSocket). Brand assets embedded via `rust-embed` from `assets/brand/`

### Web API

REST endpoints served by axum on port 7700 (default):
- Session management: `GET /api/sessions`, `POST /api/sessions/create`, `DELETE /api/sessions/:session`, `POST /api/sessions/stop-all`
- Status/peek: `GET /api/status/:session`, `GET /api/peek/:session/:target`
- Agent control: `POST /api/send`, `POST /api/spawn`, `POST /api/interrupt`, `POST /api/kill-workers`, `POST /api/kill-agent`, `POST /api/open-terminal`
- Directory browsing: `GET /api/browse`, `GET /api/recents`

## CI

GitHub Actions (`.github/workflows/ci.yml`): matrix build on `ubuntu-latest` + `macos-latest` running `fmt --check`, `test`, `clippy`, `build --release`, and `bash -n install.sh`.

## Conventions

- Binary and all docs use `crewmux` as the primary command name. `cm` and `ai` are legacy aliases (symlinks created by `install.sh`)
- All timestamps use ISO 8601 via `chrono`
- Log format: `[HH:MM:SS] EVENT_TYPE [target] message` (append-only files in `~/.crewmux/logs/`)
- Master prompt lives in `assets/master-prompt.md` (compile-time embedded), user-customizable at `~/.crewmux/master-prompt.md`
- macOS-only for service/launchd features; tmux required on all platforms
- When modifying tmux pane/metadata formats, consider stale metadata and backward compatibility with `~/.ai-team/`
- If changing master/worker orchestration rules, update `assets/master-prompt.md` and related docs together
