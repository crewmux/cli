# AI Team Controller (`ai`)

tmux 기반 AI 에이전트 팀을 관리하는 올인원 CLI + 웹 대시보드.

Claude, Codex 등 여러 AI CLI 에이전트를 동시에 띄우고, 작업을 분배하고, 실시간으로 모니터링할 수 있습니다.

## 원라인 설치

```bash
git clone https://github.com/YOUR/ai-ctl.git && cd ai-ctl && ./install.sh
```

### 필요 조건

- macOS (Linux도 가능, launchd 대신 systemd 수동 설정)
- [tmux](https://github.com/tmux/tmux) (`brew install tmux`)
- [Rust](https://rustup.rs) (설치 스크립트가 자동 설치)
- `claude` CLI 또는 `codex` CLI (사용할 에이전트)

### 수동 빌드

```bash
cargo build --release
cp target/release/ai ~/.local/bin/ai
```

---

## 사용법

### 팀 세션

```bash
# 현재 디렉토리로 팀 시작 (master Claude 자동 생성)
ai team start

# 특정 디렉토리로 시작
ai team start /path/to/project

# 세션 목록
ai team list

# 세션 중지
ai team stop

# 모든 세션 중지
ai team stop-all

# 기존 세션에 다시 연결
ai team attach
```

### 워커 생성

```bash
# Claude 워커 1개 + 작업 지정
ai task spawn "Fix the login bug"

# Codex 워커 (기본 모델)
ai task spawn -t codex "Refactor the API"

# Codex 5.4 모델로 워커 생성
ai task spawn -t codex -m codex-5.4 "Optimize the query"

# Claude 워커 3개 동시 스폰
ai task spawn -t claude -n 3 "Write unit tests"

# 마스터에 직접 메시지
ai task master "Check worker progress"

# 특정 워커에 메시지
ai task send codex-1 "Also fix the edge case"

# 모든 워커 정리
ai task clean
```

### 모니터링 & 제어

```bash
# 팀 상태
ai ctl status

# 에이전트 목록
ai ctl roles

# 에이전트 출력 보기
ai ctl peek master
ai ctl peek codex-1 -l 100

# 에이전트에 메시지 보내기
ai ctl send master "How is it going?"

# 전체 브로드캐스트
ai ctl broadcast "Stop and report"

# 에이전트 중단 (Ctrl+C)
ai ctl interrupt codex-1
ai ctl interrupt all

# 모든 워커 킬
ai ctl kill-workers

# 로그 보기
ai ctl log
ai ctl log -f   # follow 모드
```

### 웹 대시보드

```bash
# 수동 실행 (기본 포트 7700)
ai web

# 포트 지정
ai web -p 8080

# macOS 서비스로 상시 실행 (로그인 시 자동 시작)
ai install

# 서비스 제거
ai uninstall
```

대시보드 주소: **http://localhost:7700**

대시보드에서 할 수 있는 것:
- 세션 생성 / 중지
- 워커 스폰 (타입, 모델, 수량 지정)
- 에이전트 실시간 출력 확인
- 에이전트에 메시지 전송
- 개별 워커 중단 / 킬
- 작업 로그 확인
- 자동 새로고침 (3초 간격)

---

## 아키텍처

```
ai (single binary, ~2.6MB)
├── ai team start     → tmux session + master Claude
├── ai task spawn     → tmux split-window + CLI agent
├── ai ctl status     → read ~/.ai-team/meta.json
├── ai web            → axum HTTP server + embedded HTML
└── ai install        → macOS launchd plist
```

- **메타데이터**: `~/.ai-team/tasks/<session>/meta.json`
- **로그**: `~/.ai-team/logs/<session>.log`
- **서비스 로그**: `~/.ai-team/service/stdout.log`

### 지원 에이전트 타입

| 타입 | CLI | 모델 지정 |
|------|-----|-----------|
| Claude | `claude` | `--model <model>` |
| Codex | `codex` | `-m <model>` |

---

## 전체 명령어 요약

| 명령어 | 설명 |
|--------|------|
| `ai team start [dir]` | 팀 세션 시작 |
| `ai team stop` | 세션 중지 |
| `ai team stop-all` | 모든 세션 중지 |
| `ai team list` | 세션 목록 |
| `ai team attach` | 세션 재연결 |
| `ai task spawn [-t type] [-m model] [-n count] "task"` | 워커 스폰 |
| `ai task master "msg"` | 마스터에 메시지 |
| `ai task send <name> "msg"` | 워커에 메시지 |
| `ai task clean` | 워커 전체 제거 |
| `ai ctl status` | 팀 상태 |
| `ai ctl roles` | 에이전트 목록 |
| `ai ctl peek <name>` | 출력 확인 |
| `ai ctl send <name> "msg"` | 메시지 전송 |
| `ai ctl broadcast "msg"` | 전체 전송 |
| `ai ctl interrupt [name\|all]` | 중단 (Ctrl+C) |
| `ai ctl kill-workers` | 워커 전체 킬 |
| `ai ctl log [-f]` | 로그 확인 |
| `ai web [-p port]` | 대시보드 실행 |
| `ai install` | 서비스 설치 |
| `ai uninstall` | 서비스 제거 |
