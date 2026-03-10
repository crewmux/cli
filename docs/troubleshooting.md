# Troubleshooting

## `No active team session. Run 'crewmux team start' first.`

가장 흔한 원인은 현재 디렉토리가 세션을 시작한 프로젝트 폴더와 다르기 때문입니다.

해결:

```bash
cd /path/to/original/project
crewmux ctl status
```

또는 명시적으로:

```bash
crewmux team attach /path/to/original/project
```

`crewmux task *`와 `crewmux ctl *`는 현재 작업 디렉토리에서 세션 이름을 계산합니다.

## `tmux`가 없다고 나옴

`crewmux`는 tmux를 직접 호출합니다. 먼저 tmux가 shell에서 실행되는지 확인하세요.

```bash
tmux -V
```

macOS(Homebrew):

```bash
brew install tmux
```

처음 설치라면 `./install.sh`가 tmux 설치까지 시도합니다.

## `claude: command not found`

`claude`로 master를 띄우려 했는데 Claude CLI가 없으면 해당 세션의 master pane이 정상 동작하지 않습니다.

확인:

```bash
claude --help
```

필요하면 Claude CLI 설치 후 다시 `crewmux team start -t claude`를 실행하세요. Codex만 쓸 거라면 `crewmux team start -t codex`로 시작하면 됩니다.

기본 installer는 Claude CLI도 자동 설치하려고 시도합니다. Claude만 설치하고 싶으면:

```bash
CREWMUX_INSTALL_AGENTS=claude ./install.sh
```

## Codex 워커만 안 뜸

`codex`는 선택 의존성입니다. Codex worker를 띄우려면 shell에서 `codex`가 실행 가능해야 합니다.

확인:

```bash
codex --help
```

Codex만 설치하려면:

```bash
CREWMUX_INSTALL_AGENTS=codex ./install.sh
```

## 웹 대시보드는 뜨는데 세션 생성이 실패함

주로 아래 셋 중 하나입니다.

1. `project_dir`가 잘못됨
2. 서버 프로세스가 `tmux` 또는 `claude`를 못 찾음
3. 대상 디렉토리에 접근 권한이 없음

먼저 서비스 로그를 확인하세요.

```bash
tail -n 100 ~/.crewmux/service/stdout.log
tail -n 100 ~/.crewmux/service/stderr.log
```

## `crewmux install` 후 서비스에서만 `claude`/`codex`를 못 찾음

`launchd`는 셸과 다른 환경변수를 씁니다. 현재 plist에는 일반적인 Homebrew/사용자 경로를 넣어두지만, 실제 설치 경로가 다르면 서비스에서 바이너리를 못 찾을 수 있습니다.

확인 순서:

1. 터미널에서 `which claude`, `which codex`
2. [`src/cmd/service.rs`](/Users/ko/Documents/code/opensource/cli/src/cmd/service.rs) 의 PATH 구성 확인
3. 필요하면 `crewmux uninstall` 후 경로를 맞춘 상태에서 `crewmux install`

`install.sh`가 `~/.npm-global/bin` 같은 사용자 경로를 추가로 사용했다면, 해당 경로가 launchd PATH에도 들어가야 합니다.

## `http://localhost:7700` 포트가 이미 사용 중임

수동 실행 시 다른 포트를 사용하세요.

```bash
crewmux web -p 8080
```

서비스 설치(`crewmux install`)는 현재 고정 포트 `7700`을 사용합니다.

## 세션 카드가 비어 있는데 Recent Projects는 보임

- 세션 목록은 "현재 살아 있는 tmux 세션"만 보여줍니다
- Recent Projects는 예전 `meta.json` 이력에서 존재하는 경로를 보여줍니다

즉, 세션은 종료됐지만 이전에 열었던 프로젝트 경로는 recent에 남아 있을 수 있습니다.

## 로그와 메타데이터 위치를 모르겠음

```text
~/.crewmux/tasks/<session>/meta.json
~/.crewmux/logs/<session>.log
~/.crewmux/service/stdout.log
~/.crewmux/service/stderr.log
```

문제가 생기면 보통 아래 순서로 보면 됩니다.

1. `crewmux ctl status`
2. `crewmux ctl roles`
3. `crewmux ctl peek master -l 100`
4. `crewmux ctl log -f`

## pane을 tmux에서 직접 닫은 뒤 상태가 이상함

`crewmux`는 tmux 상태와 `meta.json`을 같이 사용합니다. pane을 외부에서 직접 닫으면 메타데이터가 즉시 정리되지 않을 수 있습니다.

가장 안전한 정리 방법:

```bash
crewmux ctl kill-workers
crewmux team stop
crewmux team start
```

## master가 워커 충돌을 자꾸 냄

기본적으로 `crewmux`는 conflict-avoidance master prompt를 자동 생성해 사용합니다. 그래도 운영 규칙을 더 강하게 바꾸고 싶으면 아래 파일을 수정하세요.

```bash
open ~/.crewmux/master-prompt.md
```

추천 규칙은 [Orchestration Guide](./orchestration-guide.md)에 정리돼 있습니다.
