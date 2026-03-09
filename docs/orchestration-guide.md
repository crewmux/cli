# Orchestration Guide

이 문서는 master가 워커를 어떻게 나눠야 충돌이 적고 생산성이 높은지 설명합니다. 제품 기본값은 `assets/master-prompt.md`에 들어 있으며, 첫 master 실행 시 `~/.crewmux/master-prompt.md`로 bootstrap 됩니다.

## 기본 원칙

1. 병렬화보다 ownership을 먼저 정합니다.
2. 같은 파일/모듈을 동시에 수정하는 구현 워커는 만들지 않습니다.
3. 겹칠 가능성이 보이면 분석 1명 -> 구현 1명 -> 검증 1명 순으로 순차화합니다.
4. 새 워커를 만들기 전에 기존 워커를 재사용할 수 있는지 먼저 봅니다.

## 좋은 분할 기준

- `src/web/*` 와 `src/cmd/*` 같이 디렉토리가 명확히 갈리는 경우
- 프론트 UI 구현과 백엔드 API 구현처럼 런타임 경계가 다른 경우
- 구현 워커와 테스트/리뷰 워커가 write scope를 분리할 수 있는 경우
- 문서 작업과 코드 작업처럼 산출물이 다른 경우

## 피해야 할 분할

- 같은 feature의 구현과 리팩터링을 서로 다른 구현 워커에게 동시에 맡기기
- 공유 타입, schema, lockfile, root config를 여러 워커가 동시에 만지게 하기
- "전체 코드베이스 정리"처럼 ownership이 모호한 태스크를 여러 명에게 던지기
- 이미 진행 중인 worker가 있는 영역에 새 worker를 또 붙이기

## 추천 패턴

### 패턴 A: 구현 1 + 검증 1

- Codex worker: 구현, 테스트 추가
- Claude worker: 리뷰, 리스크 확인, 누락 케이스 점검

적합한 상황:

- 변경 범위는 좁지만 회귀 리스크는 높은 버그 수정

### 패턴 B: 분석 1 -> 구현 1

- Claude worker: 구조 파악, 파일 ownership 정의, 접근법 정리
- Codex worker: 분석 결과를 바탕으로 bounded implementation

적합한 상황:

- 어디를 수정해야 하는지 애매한 초기 진입

### 패턴 C: 영역 분할 구현

- worker A: `src/web/*`
- worker B: `src/web/mod.rs`는 master가 직접 분배하거나 단일 owner만 유지
- worker C: `docs/*`

적합한 상황:

- 파일 ownership을 명확히 쪼갤 수 있을 때만

## Worker 태스크 작성법

좋은 태스크에는 아래가 들어갑니다.

- 정확한 목표
- 수정 가능한 파일/디렉토리
- 수정하면 안 되는 영역
- 필요한 검증
- 응답 형식: changed files, tests run, blockers, risk

예시:

```text
Fix the session status bug in src/web/mod.rs.
Own only src/web/mod.rs and related docs/api-reference.md updates.
Do not touch static/index.html or tmux/session lifecycle code.
Run cargo test and summarize changed files, tests run, and remaining risk.
```

## Master follow-up 규칙

- 같은 ownership의 후속 작업은 새 worker를 만들지 말고 기존 worker에 보냅니다
- 충돌 가능성이 생기면 한쪽 worker를 멈추고 ownership을 다시 선언합니다
- 결과를 합칠 때는 changed files 목록 기준으로 교차 여부를 먼저 확인합니다

## 운영 팁

- worker가 1~2명일 때가 가장 안정적입니다
- 3명 이상이면 대부분 리뷰/문서/테스트처럼 비충돌 역할을 섞는 편이 안전합니다
- idle worker를 미리 많이 띄우는 것보다 필요 시점에 bounded task로 띄우는 편이 낫습니다
