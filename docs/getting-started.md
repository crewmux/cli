# Getting Started

이 문서는 `crewmux`을 처음 설치하고 실제 프로젝트에서 첫 팀 세션을 띄우는 최소 절차를 정리합니다. 문서 예시는 macOS 기준입니다. Linux에서는 서비스 설치 부분만 수동으로 대체하면 됩니다.

## 1. 사전 확인

필수:

- `tmux`
- Node.js 18+
- `claude` CLI 또는 `codex` CLI
- Rust toolchain (`install.sh` 사용 시 자동 설치 가능)

선택:

- `codex` CLI: Codex 워커를 쓸 때만 필요

확인 명령:

```bash
tmux -V
claude --help
codex --help   # Codex를 쓸 예정일 때만
```

중요: 이제 `crewmux team start`에서 master provider를 고를 수 있습니다. `claude`만 있으면 Claude master, `codex`만 있으면 Codex master로 시작하면 됩니다.

## 2. 설치

### 권장: 설치 스크립트 사용

```bash
git clone <repo-url> crewmux
cd crewmux
./install.sh
```

설치 스크립트는 아래를 처리합니다.

1. `tmux`, `curl`, `git` 확인 및 시스템 패키지 설치
2. Node.js / npm 확인
3. Claude Code CLI, Codex CLI 설치
4. `cargo`가 없으면 Rust 설치
5. 릴리스 빌드
6. `~/.local/bin/crewmux` 설치
7. 기존 `~/.local/bin/cm`, `~/.local/bin/ai` 제거
8. 선택적으로 `crewmux install` 실행

옵션:

```bash
CREWMUX_INSTALL_AGENTS=claude ./install.sh
CREWMUX_INSTALL_AGENTS=codex ./install.sh
CREWMUX_INSTALL_SERVICE=0 ./install.sh
```

### 수동 설치

```bash
cargo build --release
mkdir -p ~/.local/bin
cp target/release/crewmux ~/.local/bin/crewmux
rm -f ~/.local/bin/cm ~/.local/bin/ai
export PATH="$HOME/.local/bin:$PATH"
```

## 3. 설치 검증

```bash
crewmux --help
crewmux team --help
crewmux task --help
crewmux ctl --help
```

위 출력이 정상이고 `tmux`, `claude`가 shell에서 직접 실행되면 첫 실행 준비가 끝난 상태입니다.

## 4. 첫 팀 세션 시작

프로젝트 디렉토리 안으로 이동한 뒤 시작합니다.

```bash
cd /path/to/your/project
crewmux team start
crewmux team start -t codex -m gpt-5.4
```

실행하면 다음이 자동으로 만들어집니다.

- tmux 세션 `crewmux-<디렉토리명>`
- master pane (`claude` 또는 `codex`)
- log pane (`tail -f ~/.crewmux/logs/<session>.log`)
- `~/.crewmux/tasks/<session>/meta.json`
- `~/.crewmux/master-prompt.md` 기본 템플릿(없을 때만)

예전 기본 prompt가 남아 있으면 `.legacy.bak`로 백업 후 새 템플릿으로 교체됩니다.

세션 이름은 현재 디렉토리명에서 계산됩니다. 예를 들어 `/Users/ko/work/api-server`에서 시작하면 세션 이름은 `crewmux-api-server` 입니다.

## 5. 워커 스폰과 제어

### 태스크와 함께 워커 생성

```bash
crewmux task spawn -t codex -m gpt-5.3-codex "Fix the login bug"
crewmux task spawn -t claude -n 2 "Write tests for src/auth.rs"
```

CLI에서는 `crewmux task spawn`의 태스크 문자열이 필수입니다.

### 상태 확인

```bash
crewmux ctl status
crewmux ctl roles
crewmux ctl peek master
crewmux ctl peek codex-1 -l 100
```

### 메시지 전송

```bash
crewmux task master "Summarize current progress"
crewmux task send codex-1 "Focus on the edge case handling"
crewmux ctl broadcast "Stop and report current status"
```

### 중단 / 정리

```bash
crewmux ctl interrupt all
crewmux ctl kill-workers
crewmux team stop
```

## 6. 웹 대시보드 사용

### 수동 실행

```bash
crewmux web
```

### macOS 로그인 시 자동 실행

```bash
crewmux install
```

브라우저에서 [http://localhost:7700](http://localhost:7700) 로 접속합니다.

웹 UI에서 가능한 작업:

- 세션 생성/중지
- Recent Projects와 폴더 브라우저로 프로젝트 선택
- 워커 스폰
- 에이전트 출력 확인
- 메시지 전송
- 개별 워커 interrupt/kill
- task log 확인

## 7. 자주 놓치는 점

- `crewmux task *`, `crewmux ctl *`는 현재 디렉토리 기준으로 세션을 찾습니다
- 다른 디렉토리에서 `crewmux ctl status`를 치면 `No active team session`이 나올 수 있습니다
- 이 경우 세션을 시작했던 프로젝트 폴더로 다시 `cd`해서 실행해야 합니다
- 웹 UI는 idle worker 생성이 가능하지만 CLI는 빈 태스크를 허용하지 않습니다
- master는 기본 conflict-avoidance prompt를 자동으로 사용합니다. 필요하면 `~/.crewmux/master-prompt.md`를 수정하세요

## 다음 문서

- [CLI Reference](./cli-reference.md)
- [Web API Reference](./api-reference.md)
- [Orchestration Guide](./orchestration-guide.md)
- [Troubleshooting](./troubleshooting.md)
