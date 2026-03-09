# Contributing

`CrewMux`는 tmux 세션, 파일시스템 메타데이터, 웹 UI가 동시에 맞물리는 도구라서 작은 수정도 런타임/문서/UI를 같이 보는 편이 안전합니다. PR은 "작동", "문서 반영", "검증 가능성" 세 가지를 함께 만족시키는 방향을 기본으로 합니다.

## 로컬 개발 환경

권장:

- macOS 또는 Linux
- `tmux`
- Rust stable
- Node.js 18+
- `claude` CLI 또는 `codex` CLI

처음 세팅:

```bash
./install.sh
```

특정 provider만 원하면:

```bash
CM_INSTALL_AGENTS=claude ./install.sh
CM_INSTALL_AGENTS=codex ./install.sh
```

## 작업 원칙

- 기능 변경 시 관련 문서도 같이 갱신합니다
- 사용자 경험에 영향을 주는 수정이면 CLI와 웹 UI 흐름을 둘 다 확인합니다
- tmux pane / metadata 포맷을 건드릴 때는 stale metadata와 하위 호환을 먼저 생각합니다
- master/worker orchestration 규칙을 바꾸면 `assets/master-prompt.md`와 문서를 같이 맞춥니다
- 큰 리팩터링보다 추적 가능한 작은 단계 PR을 선호합니다

## 기본 검증

PR 전 최소한 아래는 통과시켜 주세요.

```bash
cargo fmt
cargo test
cargo clippy --all-targets --all-features -- -D warnings
bash -n install.sh
```

가능하면 추가로 확인:

```bash
cargo build --release
./install.sh
cm web
```

## 문서 규칙

- README는 "처음 보는 사용자" 기준으로 유지합니다
- `docs/`는 구현 상세와 운영 규칙을 설명합니다
- 예시 명령은 현재 코드 기준으로 실제 동작하는 형태만 남깁니다
- provider/model 이름은 UI/CLI 구현과 정확히 일치해야 합니다

## PR 체크리스트

- 변경 목적이 README 또는 docs에 반영되어 있음
- 새 옵션/동작이 CLI help 또는 API 문서와 어긋나지 않음
- 설치 흐름을 바꿨다면 `install.sh`와 Getting Started를 같이 수정함
- master 전략을 바꿨다면 prompt 템플릿과 orchestration 문서를 같이 수정함
- 빌드/테스트/클리피 결과를 확인함

## 릴리스 전 확인

- `cargo build --release`
- 설치 스크립트 재실행
- `cm install` 후 서비스 기동 확인
- `http://localhost:7700`에서 웹 UI 확인
- 세션 생성, worker spawn, interrupt, kill, open iTerm까지 수동 확인
