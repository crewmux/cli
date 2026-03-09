# Web API Reference

Base URL: `http://localhost:7700`

## 공통 사항

- 대부분의 응답은 JSON입니다
- `GET /api/peek/...`만 plain text를 반환합니다
- 오류 응답은 `400`, `404`, `500`과 함께 plain text 메시지를 반환합니다
- session 이름은 CLI와 동일하게 기본적으로 `cm-<project-basename>` 규칙을 따릅니다

## Sessions

### `GET /api/sessions`

현재 살아 있는 tmux 세션 목록을 반환합니다.

특징:

- `cm-`로 시작하는 tmux 세션과 legacy `ai-` 세션만 노출됩니다
- 이전 이력이나 inactive 세션은 포함되지 않습니다

응답 예시:

```json
[
  {
    "name": "cm-my-project",
    "project": "/Users/ko/my-project",
    "worker_count": 2,
    "started": "2026-03-09T10:00:00Z",
    "active": true
  }
]
```

### `POST /api/sessions/create`

새 세션을 생성합니다.

요청:

```json
{
  "project_dir": "/Users/ko/my-project",
  "master_type": "codex",
  "master_model": "gpt-5.4"
}
```

동작:

- 경로를 canonicalize
- tmux 세션 생성
- 선택한 master provider 실행
- log pane 생성
- `meta.json` 저장

응답 예시:

```json
{
  "ok": true,
  "message": "Session 'cm-my-project' created",
  "session": "cm-my-project"
}
```

주의:

- 이미 존재하는 세션이면 `ok: true`와 함께 `already exists` 메시지를 반환합니다
- `master_type`, `master_model`을 생략하면 기본값은 `claude` / `null` 입니다

### `DELETE /api/sessions/{session}`

세션을 중지하고 `~/.crewmux/tasks/<session>/` 메타데이터를 삭제합니다. 로그 파일은 유지됩니다.

### `POST /api/sessions/stop-all`

이름이 `cm-` 또는 legacy `ai-`로 시작하는 모든 활성 세션을 중지합니다.

## Status & Output

### `GET /api/status/{session}`

세션 상세 상태를 조회합니다.

응답 예시:

```json
{
  "session": "cm-my-project",
  "project": "/Users/ko/my-project",
  "started": "2026-03-09T10:00:00Z",
  "last_task": "Fix the auth bug",
  "task_count": 3,
  "master": {
    "name": "master",
    "agent_type": "codex",
    "model": "gpt-5.4",
    "pane": "%1"
  },
  "workers": [
    {
      "name": "codex-1",
      "agent_type": "codex",
      "model": "gpt-5.3-codex",
      "pane": "%3"
    }
  ]
}
```

참고:

- pane 값은 tmux pane id(`%3`) 또는 호환용 target 문자열입니다
- worker 목록은 이름 기준 정렬되어 반환됩니다

### `GET /api/peek/{session}/{target}?lines=80`

tmux pane 출력을 캡처합니다.

파라미터:

- `target`: `master`, `log`, worker 이름
- `lines`: 캡처할 줄 수, 기본값 `80`

응답: plain text

참고:

- 내부적으로 worker 이름 부분 일치도 해석하지만 exact worker 이름을 보내는 것이 안전합니다

## Agent Control

### `POST /api/send`

특정 pane에 메시지를 전송합니다.

요청:

```json
{
  "session": "cm-my-project",
  "target": "codex-1",
  "message": "Also fix the edge case"
}
```

`target`으로 `master`, `log`, worker 이름을 사용할 수 있습니다.

### `POST /api/spawn`

워커를 생성하고, `task`가 비어 있지 않으면 태스크를 전송합니다.

요청:

```json
{
  "session": "cm-my-project",
  "worker_type": "codex",
  "model": "gpt-5.3-codex",
  "count": 2,
  "task": "Fix the login bug"
}
```

필드:

- `worker_type`: `claude` 또는 `codex`
- `model`: 선택
- `count`: 선택, 기본 `1`
- `task`: 빈 문자열 허용

특징:

- `task`가 비어 있으면 idle worker만 생성합니다
- `task`가 비어 있을 때는 `task_count`와 `last_task`를 갱신하지 않습니다
- 워커 생성 후 약 3초 대기한 뒤 태스크를 전송합니다

### `POST /api/interrupt`

특정 pane 또는 전체 active agent에 Ctrl+C를 전송합니다.

요청:

```json
{
  "session": "cm-my-project",
  "target": "all"
}
```

규칙:

- `target=all`이면 master + 모든 worker에 Ctrl+C를 보냅니다
- `log` pane은 `all` 대상이 아닙니다
- 특정 target으로는 `master`, `log`, worker 이름을 받을 수 있습니다

### `POST /api/kill-workers`

모든 worker pane을 종료합니다.

요청:

```json
{
  "session": "cm-my-project"
}
```

### `POST /api/kill-agent`

특정 worker pane만 종료합니다.

요청:

```json
{
  "session": "cm-my-project",
  "target": "codex-1"
}
```

규칙:

- worker만 종료할 수 있습니다
- `master`와 `log`는 허용되지 않습니다
- worker 부분 일치는 지원하지만, 중복 매치가 생길 수 있으므로 exact name 권장

### `POST /api/open-terminal`

선택한 target이 보이도록 tmux pane을 고른 뒤 iTerm으로 세션을 엽니다.

요청:

```json
{
  "session": "cm-my-project",
  "target": "master"
}
```

특징:

- `target` 생략 시 기본값은 `master`
- iTerm이 설치된 macOS 환경을 전제로 합니다

## Directory Browser

### `GET /api/browse?path=/Users/ko/Documents`

디렉토리 브라우저용 폴더 목록을 반환합니다.

응답 예시:

```json
{
  "current": "/Users/ko/Documents",
  "parent": "/Users/ko",
  "dirs": [
    {
      "name": "project",
      "path": "/Users/ko/Documents/project",
      "is_git": true
    }
  ],
  "is_git": false
}
```

특징:

- 숨김 디렉토리는 제외됩니다
- 각 디렉토리에 대해 `.git` 존재 여부를 `is_git`으로 표시합니다

### `GET /api/recents`

이전 `meta.json` 이력에서 아직 존재하는 프로젝트 디렉토리만 추려 반환합니다.

응답 예시:

```json
[
  "/Users/ko/Documents/project",
  "/Users/ko/Documents/another-app"
]
```

## 상태 코드 가이드

| 코드 | 의미 |
|------|------|
| `200` | 정상 처리 |
| `400` | 잘못된 입력 경로, 금지된 대상(master kill 등) |
| `404` | 세션 또는 target 없음 |
| `500` | tmux 실행 실패, 파일 I/O 실패, 내부 예외 |
