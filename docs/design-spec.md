# Design Specification

## 문제 정의

AI CLI를 여러 개 동시에 돌리려면 보통 여러 터미널 창과 수동 복사/붙여넣기, 상태 확인, 로그 추적이 필요합니다. `cm`의 목적은 이 과정을 "프로젝트 단위 팀 세션"으로 정리해, 여러 에이전트를 한 번에 띄우고 제어하는 운영 오버헤드를 줄이는 것입니다.

## 대상 사용자

- 로컬 개발 환경에서 여러 AI CLI를 병렬 운용하려는 개발자
- 한 프로젝트 안에서 master/worker 식 협업을 반복적으로 사용하는 사용자
- tmux 기반 워크플로우에 익숙하거나, 최소한 tmux를 설치할 수 있는 사용자

## 제품 원칙

1. 단일 바이너리 중심
2. CLI 우선, 웹 UI는 같은 기능의 얇은 레이어
3. 상태 저장은 tmux + 파일시스템만 사용
4. 세션은 프로젝트 디렉토리에 강하게 묶음
5. 운영 중 무엇이 일어나는지 로그와 pane 출력으로 추적 가능해야 함

## 현재 제공 범위

### 세션 관리

- 프로젝트별 세션 생성/종료/목록/재연결
- 웹 UI에서 디렉토리 브라우저와 recent project 기반 세션 생성

### 에이전트 관리

- master pane 자동 생성
- 기본 master orchestration prompt 자동 bootstrap
- Claude/Codex worker spawn
- 모델 인자 전달
- 다중 worker 동시 spawn
- direct send / broadcast / interrupt / kill-workers / kill-agent

### 관찰성

- 현재 세션 상태 요약
- pane 출력 캡처
- task dispatch 로그
- 서비스 stdout/stderr 로그

### 웹 대시보드

- 세션 카드 뷰
- 에이전트 리스트
- output viewer
- spawn form
- send bar
- recent projects / directory browser

## 명시적 제약

- master agent provider/model 선택 가능
- installer는 tmux / Node.js / Rust / Claude CLI / Codex CLI 자동 설치를 시도하지만, 각 provider 로그인까지 대신 처리하지는 않음
- background service는 macOS `launchd`만 지원
- 실시간 갱신은 polling 기반
- 서버는 별도 인증 계층이 없음
- task queue, retry, scheduling, agent health detection은 아직 없음

## UX 기대치

- 첫 실행 후 1분 안에 세션을 띄울 수 있어야 함
- 사용자는 세션 이름을 직접 관리하지 않아도 됨
- 웹 UI와 CLI가 같은 상태를 보여야 함
- 실패 시 사용자가 확인할 경로(`tmux`, `meta.json`, `logs`)가 명확해야 함

## 현재 리스크

- 사용자가 다른 디렉토리에서 `ctl`/`task` 명령을 실행하면 세션을 못 찾는다고 느낄 수 있음
- `cm-` prefix 기반 `stop-all`은 보수적으로 관리해야 하며 legacy `ai-*` 세션도 고려해야 함
- tmux pane을 외부에서 직접 닫으면 메타데이터와 실제 상태가 어긋날 수 있음
- Linux에서는 서비스 경험이 macOS보다 거칠다

## 다음 단계 제안

우선순위가 높은 개선 과제:

1. session selection을 CWD 외 인자/환경변수로도 받을 수 있게 확장
2. WebSocket 또는 SSE 기반 출력 스트리밍
3. stale pane / stale metadata 정리 로직
4. Linux 서비스(systemd) 지원
5. worker ownership / task queue를 메타데이터에 명시적으로 저장
