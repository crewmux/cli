# Homebrew

`CrewMux`를 Homebrew로 배포하려면 공식 문서 기준으로 별도 tap 저장소를 두는 구성이 필요합니다. Homebrew 5 기준으로는 로컬 formula 파일 경로를 직접 `brew install`/`brew style` 대상으로 쓰는 흐름이 막혀 있어서, tap 저장소를 전제로 문서와 검증 흐름을 맞춰 두었습니다.

관련 파일:

- [Formula/crewmux.rb](/Users/ko/Documents/project/ai-ctl/Formula/crewmux.rb): 현재 `--HEAD` 설치용 formula
- [scripts/render-homebrew-formula.sh](/Users/ko/Documents/project/ai-ctl/scripts/render-homebrew-formula.sh): stable release formula 생성 스크립트
- [scripts/update-homebrew-stable.sh](/Users/ko/Documents/project/ai-ctl/scripts/update-homebrew-stable.sh): source archive 다운로드 + SHA256 계산 + stable formula 생성 자동화
- [docs/release-strategy.md](/Users/ko/Documents/project/ai-ctl/docs/release-strategy.md): stable / HEAD 릴리스 전략

## 저장소 구조

- 메인 코드: `crewmux/cli`
- Homebrew tap: `crewmux/homebrew-tap`

Homebrew tap 이름은 `crewmux/tap`으로 노출되고, 실제 GitHub 저장소 이름은 `homebrew-tap`입니다.

## 설치

최신 `main` 기준 설치:

```bash
brew tap crewmux/tap
brew install --HEAD crewmux/tap/crewmux
```

첫 stable tag 이후에는 아래처럼 `--HEAD` 없이 설치할 수 있습니다.

```bash
brew install crewmux/tap/crewmux
```

이 formula는 Homebrew에 `crewmux` 실행 파일을 설치합니다. upstream이 내놓는 단일 실행 파일 이름과 무관하게 Homebrew에서는 `crewmux`로 노출합니다.

## stable release 배포 흐름

1. 메인 저장소에 `vX.Y.Z` 태그 생성
2. release workflow 또는 로컬 스크립트로 stable formula 생성
3. 결과를 `crewmux/homebrew-tap/Formula/crewmux.rb`에 반영

예시:

```bash
./scripts/update-homebrew-stable.sh 0.1.0 /tmp/crewmux.rb
```

기본 URL은:

```text
https://github.com/crewmux/cli/archive/refs/tags/v<version>.tar.gz
```

## tap 업데이트

메인 저장소의 formula를 tap 저장소 checkout으로 복사하려면 아래처럼 진행합니다.

```bash
./scripts/sync-homebrew-tap.sh /path/to/homebrew-tap
```

stable formula를 생성할 때는:

```bash
./scripts/update-homebrew-stable.sh 0.1.0 /path/to/homebrew-tap/Formula/crewmux.rb
```

GitHub Actions를 통한 자동 반영을 원하면 `HOMEBREW_TAP_TOKEN` secret을 설정하면 됩니다. 자세한 흐름은 [docs/release-strategy.md](/Users/ko/Documents/project/ai-ctl/docs/release-strategy.md)에 정리돼 있습니다.

## 로컬 검증

Homebrew 5 기준 로컬 검증은 tap 이름을 통해 실행해야 합니다.

```bash
brew tap-new --no-git crewmux/tap
TAP_DIR="$(brew --repository)/Library/Taps/crewmux/homebrew-tap"
./scripts/sync-homebrew-tap.sh "$TAP_DIR"
brew style crewmux/tap/crewmux
brew install --build-from-source --HEAD crewmux/tap/crewmux
brew test crewmux/tap/crewmux
brew untap crewmux/tap
```

## 주의사항

- `claude`와 `codex`는 runtime 의존성이며 formula가 자동 설치하지 않습니다
- `tmux`는 Homebrew dependency로 선언했습니다
- stable formula를 쓰려면 첫 tagged release가 먼저 필요합니다
- tag 기반 stable release는 `.github/workflows/release.yml`이 담당합니다
