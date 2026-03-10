# Architecture

## 개요

`crewmux`는 tmux를 런타임으로 삼고, 파일시스템의 `meta.json`을 상태 저장소로 쓰는 단일 Rust 바이너리입니다. `cm`, `ai`는 호환용 별칭입니다. CLI, 웹 UI, 백그라운드 서비스가 모두 같은 메타데이터와 tmux 세션을 바라보므로 별도 DB나 서버 사이드 상태가 없습니다.

```text
┌──────────────────────────────────────────────┐
│ crewmux (single binary)                    │
│  ├─ team  -> tmux session lifecycle         │
│  ├─ task  -> worker spawn + dispatch        │
│  ├─ ctl   -> inspect / send / interrupt     │
│  ├─ web   -> axum API + embedded UI         │
│  └─ install/uninstall -> launchd wrapper    │
└──────────────────────────────────────────────┘
                 │
                 ├─ tmux sessions / panes
                 └─ ~/.crewmux/{tasks,logs,service}
```

## 런타임 흐름

### 1. 팀 시작

`crewmux team start`는 아래 순서로 동작합니다.

1. 프로젝트 디렉토리 canonicalize
2. 세션 이름 계산: `crewmux-<basename>`
3. tmux 세션 생성
4. `master` pane title 설정 후 선택된 provider 실행
5. `log` pane 생성 후 로그 tail
6. 기본 master prompt가 없으면 bootstrap
7. `meta.json` 저장
8. tmux attach

### 2. 워커 스폰

`crewmux task spawn` 또는 `POST /api/spawn`은 아래를 수행합니다.

1. 현재 메타데이터 로드
2. 다음 worker 이름 계산 (`claude-1`, `codex-1` ...)
3. master pane 기준으로 tmux split
4. tmux가 반환한 pane id를 그대로 메타데이터에 저장
5. worker CLI 실행
6. 필요시 3초 대기 후 태스크 전송
7. 메타데이터와 로그 갱신
8. worker 수에 따라 레이아웃 재정렬

### 3. 제어와 모니터링

- `ctl` 명령과 웹 API는 모두 `meta.json`에서 pane 정보를 읽습니다
- 출력 보기는 `tmux capture-pane`
- 메시지 전송은 `tmux send-keys`
- interrupt는 `tmux send-keys C-c`
- kill는 `tmux kill-pane` 또는 `kill-session`

## 모듈 구조

```text
src/
├── main.rs          # clap 진입점
├── meta.rs          # ~/.crewmux/* 경로와 메타데이터 직렬화
├── prompt.rs        # 기본 master prompt bootstrap
├── tmux.rs          # tmux CLI wrapper
├── cmd/
│   ├── team.rs      # 세션 lifecycle
│   ├── task.rs      # worker spawn / direct send
│   ├── ctl.rs       # 운영 명령
│   └── service.rs   # macOS launchd plist 관리
└── web/
    └── mod.rs       # axum routes + HTML embedding

static/
└── index.html       # 단일 파일 대시보드

assets/
└── master-prompt.md # 기본 master orchestration template
```

## 상태 모델

### 파일 구조

```text
~/.crewmux/
├── tasks/<session>/meta.json
├── logs/<session>.log
├── service/stdout.log
├── service/stderr.log
└── master-prompt.md
```

### `meta.json`

핵심 필드:

- `session`: tmux session 이름
- `project`: canonical project path
- `started`: 생성 시각
- `master.pane`: master pane ID
- `workers`: worker 이름 -> pane/type/model 매핑
- `log.pane`: log pane ID
- `last_task`, `task_count`: 최근 dispatch 요약

이 메타데이터는 CLI와 웹 API가 공통으로 참조하는 단일 소스입니다.

주의:

- pane 저장값은 tmux pane index가 아니라 pane ID(`%3` 같은 형식)를 우선 사용합니다
- 예전 메타데이터에 남아 있는 `window.pane` 형태도 최대한 호환합니다

## tmux 레이아웃

기본 window는 `team` 하나이며 pane title을 적극 활용합니다.

```text
window 1: team
├── pane 0: master
├── pane 1: log
├── pane 2+: workers
```

레이아웃 전략:

- worker 1~2개: `main-vertical`
- worker 3개 이상: `tiled`

## 웹 UI 구조

- 서버: `axum`
- 정적 자산: `include_str!`로 `static/index.html` 임베드
- 상태: 서버 메모리 상태 없음
- 실시간성: polling 기반
- 세션 목록 갱신: 8초
- 선택 세션 출력/상태 갱신: 3초 (`Auto: ON`)

## 중요한 설계 제약

- 세션 컨텍스트는 현재 작업 디렉토리에서 계산됩니다
- master agent 타입과 모델은 세션 생성 시 선택할 수 있습니다
- master는 기본 conflict-avoidance prompt를 사용하며, 사용자가 `~/.crewmux/master-prompt.md`를 수정해 override할 수 있습니다
- 예전 기본 prompt는 `.legacy.bak`로 백업 후 최신 템플릿으로 교체됩니다
- 서비스 설치는 macOS `launchd`만 지원합니다
- `stop-all`은 이름이 `crewmux-` 또는 legacy `cm-`, `ai-`로 시작하는 tmux 세션 전체를 대상으로 합니다
- API와 CLI 모두 tmux가 실제 상태 저장소 역할을 하기 때문에, pane을 외부에서 수동 조작하면 메타데이터와 어긋날 수 있습니다

## 확장 포인트

새 에이전트 타입을 추가하려면 최소한 아래 세 곳을 맞춰야 합니다.

1. [`src/cmd/task.rs`](/Users/ko/Documents/code/opensource/cli/src/cmd/task.rs) 의 CLI 실행 문자열 생성
2. [`src/web/mod.rs`](/Users/ko/Documents/code/opensource/cli/src/web/mod.rs) 의 `api_spawn`
3. [`static/index.html`](/Users/ko/Documents/code/opensource/cli/static/index.html) 의 spawn 타입 UI

추가로 provider를 늘리거나 orchestration 규칙을 바꾸면 `assets/master-prompt.md`, `src/prompt.rs`, 설치 문서, 웹 UI 모델 목록을 함께 맞춰야 합니다.
