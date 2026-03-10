# CLI Reference

## 공통 규칙

- `crewmux team start|stop|attach [dir]`는 대상 디렉토리를 직접 받을 수 있습니다
- `crewmux task *`와 `crewmux ctl *`는 현재 작업 디렉토리에서 세션 이름을 계산합니다
- 즉 `/path/to/app`에서 `crewmux team start`를 했다면, 이후 제어 명령도 같은 디렉토리에서 실행하는 것이 기본 전제입니다
- 세션 이름 규칙은 `crewmux-<basename>` 입니다
- 기존 `cm-*`, `ai-*` 세션이 이미 떠 있으면 같은 프로젝트에서 `crewmux`가 그 세션을 이어서 제어합니다

예:

- `/Users/ko/my-project` -> `crewmux-my-project`
- `/Users/ko/Documents/NCNC` -> `crewmux-NCNC`

## `crewmux team`

세션 생명주기를 다루는 명령 모음입니다.

| 명령어 | 설명 |
|--------|------|
| `crewmux team start [dir] [--master-type type] [--master-model model]` | 새 팀 세션 시작. 이미 있으면 attach |
| `crewmux team stop [dir]` | 해당 프로젝트 세션 종료 |
| `crewmux team stop-all` | 이름이 `crewmux-` 또는 legacy `cm-`, `ai-`로 시작하는 모든 tmux 세션 종료 |
| `crewmux team list` | 현재 활성 세션 목록 |
| `crewmux team attach [dir]` | 기존 세션에 재연결 |

### `crewmux team start`

```bash
crewmux team start
crewmux team start /path/to/project
crewmux team start -t codex -m gpt-5.4
```

동작:

- tmux 세션 생성
- `master` pane 생성 후 선택한 provider 실행
- `log` pane 생성 후 로그 tail
- `~/.crewmux/tasks/<session>/meta.json` 저장

추가 규칙:

- `~/.crewmux/master-prompt.md`가 없으면 기본 conflict-avoidance 템플릿이 자동 생성됩니다
- Claude master는 이 파일을 append system prompt로 사용합니다
- Codex master를 선택해도 같은 파일 내용을 첫 프롬프트로 전달합니다

### `crewmux team stop`

```bash
crewmux team stop
crewmux team stop /path/to/project
```

세션 종료 후 `~/.crewmux/tasks/<session>/` 메타데이터 디렉토리를 지웁니다. 로그 파일은 남습니다.

### `crewmux team attach`

```bash
crewmux team attach
crewmux team attach /path/to/project
```

기존 tmux 세션에 붙습니다.

## `crewmux task`

워커 생성과 태스크 디스패치용 명령 모음입니다.

| 명령어 | 설명 |
|--------|------|
| `crewmux task spawn [options] "task"` | 워커 생성 후 태스크 전송 |
| `crewmux task master "msg"` | master에 직접 메시지 전송 |
| `crewmux task send <worker> "msg"` | 특정 워커에 메시지 전송 |
| `crewmux task clean` | 모든 worker pane 종료 |

### `crewmux task spawn`

```bash
crewmux task spawn [OPTIONS] [TASK]...
```

옵션:

| 옵션 | 설명 |
|------|------|
| `-t, --type <TYPE>` | `claude` 또는 `codex` |
| `-m, --model <MODEL>` | 에이전트 모델 이름 |
| `-n, --count <COUNT>` | 생성할 워커 수 |

예시:

```bash
crewmux task spawn "Fix the login bug"
crewmux task spawn -t codex -m gpt-5.3-codex "Optimize the query"
crewmux task spawn -t codex -m gpt-5.4 "OpenAI general coding pass"
crewmux task spawn -t claude -n 3 "Write unit tests"
```

주의:

- `clap` help에는 `[TASK]...`로 보이지만 실제 CLI 구현은 빈 태스크를 허용하지 않습니다
- 태스크가 비어 있으면 usage 에러를 반환합니다
- 워커가 뜬 뒤 약 3초 대기한 다음 태스크를 전송합니다

### `crewmux task master`

```bash
crewmux task master "Check worker progress and summarize"
```

master pane에 바로 메시지를 보냅니다.

### `crewmux task send`

```bash
crewmux task send codex-1 "Also handle null values"
```

특정 워커에 메시지를 보냅니다. 실제 구현은 부분 일치도 허용하지만, 혼선을 피하려면 canonical worker 이름을 쓰는 편이 안전합니다.

### `crewmux task clean`

```bash
crewmux task clean
```

모든 worker pane을 종료하고 메타데이터의 worker 목록을 비웁니다.

## `crewmux ctl`

모니터링과 운영 제어용 명령 모음입니다.

| 명령어 | 설명 |
|--------|------|
| `crewmux ctl status` | 현재 팀 상태 요약 |
| `crewmux ctl roles` | master/worker/log 목록 |
| `crewmux ctl peek [target] [-l N]` | pane 출력 캡처 |
| `crewmux ctl send <target> "msg"` | 에이전트에 메시지 전송 |
| `crewmux ctl log [-f]` | task log 출력 또는 follow |
| `crewmux ctl broadcast "msg"` | master + 모든 worker에 메시지 전송 |
| `crewmux ctl interrupt [target\|all]` | Ctrl+C 전송 |
| `crewmux ctl kill-workers` | 모든 worker pane 종료 |

### `crewmux ctl status`

```bash
crewmux ctl status
```

현재 세션, 프로젝트 경로, 마지막 태스크, master, worker 목록을 출력합니다.

### `crewmux ctl roles`

```bash
crewmux ctl roles
```

사용 가능한 target 이름을 확인할 때 쓰는 명령입니다. `master`, 각 worker, `log`가 보입니다.

### `crewmux ctl peek`

```bash
crewmux ctl peek
crewmux ctl peek master
crewmux ctl peek codex-1 -l 100
crewmux ctl peek log -l 30
```

규칙:

- 기본 target은 `master`
- `master`, `log`, worker 이름을 받을 수 있습니다
- worker 이름은 부분 일치도 지원하지만 exact match 권장

### `crewmux ctl send`

```bash
crewmux ctl send master "How is it going?"
crewmux ctl send codex-1 "Focus on performance"
```

`master`, `log`, worker pane으로 메시지를 보낼 수 있습니다. 일반적인 운영에서는 `master` 또는 worker 대상으로 사용하는 편이 자연스럽습니다.

### `crewmux ctl log`

```bash
crewmux ctl log
crewmux ctl log -f
```

현재 디렉토리 기준 세션의 로그 파일을 출력합니다.

### `crewmux ctl broadcast`

```bash
crewmux ctl broadcast "Stop and report your progress"
```

master와 모든 worker에 같은 메시지를 순차 전송합니다.

### `crewmux ctl interrupt`

```bash
crewmux ctl interrupt codex-1
crewmux ctl interrupt master
crewmux ctl interrupt all
```

규칙:

- 특정 target을 주면 해당 pane에 Ctrl+C를 보냅니다
- `all`은 master + 모든 worker에 Ctrl+C를 보냅니다
- `log` pane은 `all`에 포함되지 않습니다

### `crewmux ctl kill-workers`

```bash
crewmux ctl kill-workers
```

모든 worker pane을 종료합니다. master와 log pane은 남습니다.

## `crewmux web`

```bash
crewmux web
crewmux web -p 8080
```

내장 axum 서버를 띄웁니다. 기본 포트는 `7700`입니다.

## `crewmux install` / `crewmux uninstall`

```bash
crewmux install
crewmux uninstall
```

- `crewmux install`: macOS `launchd` 서비스 등록
- `crewmux uninstall`: 서비스 제거

서비스 로그:

- `~/.crewmux/service/stdout.log`
- `~/.crewmux/service/stderr.log`
