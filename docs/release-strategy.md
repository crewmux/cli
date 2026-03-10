# Release Strategy

`CrewMux`는 두 개의 배포 채널을 유지합니다.

- `main`: 빠르게 검증하고 싶은 사용자용 `HEAD` 채널
- `vX.Y.Z` 태그: Homebrew stable install과 GitHub Release에 쓰는 stable 채널

이 문서는 stable release를 만들 때의 기준과 자동화 구성을 설명합니다.

## 목표

- `brew install crewmux/tap/crewmux`가 동작하는 stable formula를 유지
- 태그 기반으로 GitHub Release와 Homebrew stable formula를 동시에 생성
- release 직전 검증이 로컬과 GitHub Actions에서 동일하게 재현되도록 유지

## 버전 정책

- 버전 형식은 `MAJOR.MINOR.PATCH`
- Git 태그 형식은 `vMAJOR.MINOR.PATCH`
- `Cargo.toml`의 `version` 값과 Git 태그 버전은 반드시 일치해야 함
- pre-release(`-rc`, `-beta`)는 stable tap 업데이트 대상이 아님

## 배포 채널

### Development / HEAD

개발 중 최신 코드는 아래 흐름으로 배포합니다.

```bash
brew tap crewmux/tap
brew install --HEAD crewmux/tap/crewmux
```

이 채널은 `main`의 최신 상태를 바로 반영합니다.

### Stable

stable은 annotated tag를 기준으로 배포합니다.

```bash
git tag -a v0.1.0 -m "CrewMux v0.1.0"
git push origin v0.1.0
```

태그가 push되면 release workflow가 다음을 수행합니다.

1. release preflight 실행
2. GitHub Release 생성
3. stable Homebrew formula 생성
4. source archive SHA256 생성
5. `HOMEBREW_TAP_TOKEN`이 있으면 `crewmux/homebrew-tap` 자동 업데이트

## 로컬 운영 절차

stable release를 준비할 때는 아래 순서를 사용합니다.

1. `Cargo.toml` 버전 갱신
2. 설치/문서/Homebrew 문구 확인
3. 아래 preflight 실행

```bash
./scripts/release-preflight.sh 0.1.0
```

4. annotated tag 생성
5. 원격에 tag push
6. GitHub Actions release workflow 결과 확인

## Homebrew stable formula

stable formula는 source archive checksum이 필요하므로 수동 hash 입력을 없애는 스크립트를 같이 제공합니다.

```bash
./scripts/update-homebrew-stable.sh 0.1.0 /path/to/homebrew-tap/Formula/crewmux.rb
```

이 스크립트는 아래를 자동으로 수행합니다.

- GitHub tag source archive 다운로드
- SHA256 계산
- stable formula 렌더링

## GitHub Actions 구성

release workflow는 `.github/workflows/release.yml`에 있습니다.

핵심 동작:

- `push.tags: v*.*.*` 에 반응
- `./scripts/release-preflight.sh` 로 안정성 검증
- `softprops/action-gh-release` 로 GitHub Release 생성
- `crewmux.rb` 와 `.sha256` 파일을 release asset으로 업로드
- optional secret이 있으면 tap repo도 자동 업데이트

## 필요한 Secret

다른 repo인 `crewmux/homebrew-tap`에 자동 push하려면 GitHub Actions secret이 필요합니다.

- `HOMEBREW_TAP_TOKEN`

권장 권한:

- `contents: write` for `crewmux/homebrew-tap`

secret이 없으면 release 자체는 계속 성공하고, tap 업데이트만 건너뜁니다.

## 롤백 전략

stable release가 잘못 나갔을 때는 아래 순서로 정리합니다.

1. 잘못된 tag/release 비공개 또는 삭제
2. `homebrew-tap` formula를 직전 stable 버전으로 복구
3. 수정 커밋 merge
4. 새 버전으로 다시 tag

`v0.1.0`을 취소한 뒤 같은 번호를 재사용하기보다 `v0.1.1`로 바로 수정 배포하는 편이 안전합니다.

## 검증 기준

stable release gate는 현재 아래를 기준으로 둡니다.

- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --release`
- `bash -n install.sh scripts/*.sh`
- dashboard inline script syntax check

## 향후 확장

stable 트랙이 자리 잡으면 이후에 검토할 수 있는 항목은 아래와 같습니다.

- changelog 자동 생성
- macOS/Linux binary artifact 첨부
- `homebrew/core` 제출
- pre-release 채널(`rc`) 추가
