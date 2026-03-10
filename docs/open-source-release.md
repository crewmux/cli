# Open Source Release

이 문서는 `crewmux`를 공개 저장소로 다듬을 때 필요한 체크리스트를 정리합니다.

## 최소 공개 기준

- README가 설치/사용/제약을 현재 코드 기준으로 설명함
- `docs/`에 CLI, API, 아키텍처, orchestration 규칙이 정리돼 있음
- `install.sh`가 의존성 설치를 포함해 첫 경험을 망치지 않음
- CI가 `fmt`, `test`, `clippy`, `bash -n install.sh`를 검증함
- 서비스 로그/메타데이터 위치가 문서화돼 있음

## 공개 전 체크리스트

1. 저장소 이름, 바이너리 이름, launchd label을 최종 이름으로 확정
2. LICENSE 선택
3. GitHub Actions 활성화
4. README의 install URL을 실제 GitHub raw URL로 교체
5. 스크린샷 또는 짧은 GIF 추가
6. issue / PR 템플릿 추가 여부 결정
7. stable release 전략 문서화 및 Homebrew stable formula 경로 준비
8. 첫 릴리스 태그와 changelog 작성

## 추천 GitHub 공개 구성

- `README.md`
- `CONTRIBUTING.md`
- `docs/`
- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `install.sh`
- 예시 스크린샷 1장

## stable release 준비

- [docs/release-strategy.md](/Users/ko/Documents/project/ai-ctl/docs/release-strategy.md)에 stable / HEAD 채널 구분과 태그 정책을 문서화
- `scripts/release-preflight.sh` 로 release 직전 검증 절차를 고정
- `scripts/update-homebrew-stable.sh` 로 stable Homebrew formula 생성 자동화
- `HOMEBREW_TAP_TOKEN` secret이 있으면 release workflow에서 tap repo까지 자동 갱신

## 네이밍 메모

현재 이름은 `crewmux` / `com.crewmux.web` 로 정리됐고, 기존 `cm`, `ai`는 호환용 별칭으로만 유지됩니다.

## 이름 확정 후 바꿔야 할 곳

- `Cargo.toml` 패키지 설명
- `src/main.rs` CLI name/about
- `src/cmd/service.rs` 의 launchd label
- README / docs / 설치 URL / 스크린샷
