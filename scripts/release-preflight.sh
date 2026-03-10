#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "usage: $0 <version>" >&2
    echo "example: $0 0.1.0" >&2
    exit 1
fi

VERSION="$1"

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "version must match <major>.<minor>.<patch>" >&2
    exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

if [[ -n "$(git status --short)" ]]; then
    echo "working tree must be clean before a stable release" >&2
    exit 1
fi

CARGO_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)"
if [[ "$CARGO_VERSION" != "$VERSION" ]]; then
    echo "Cargo.toml version ($CARGO_VERSION) does not match requested release version ($VERSION)" >&2
    exit 1
fi

if ! command -v node >/dev/null 2>&1; then
    echo "node is required to syntax-check the dashboard script" >&2
    exit 1
fi

echo "==> cargo fmt --check"
cargo fmt --check

echo "==> cargo test"
cargo test

echo "==> cargo clippy --all-targets --all-features -- -D warnings"
cargo clippy --all-targets --all-features -- -D warnings

echo "==> cargo build --release"
cargo build --release

echo "==> bash -n install.sh scripts/*.sh"
bash -n install.sh scripts/*.sh

echo "==> node --check dashboard script"
awk '/<script>/{flag=1;next}/<\/script>/{flag=0}flag' static/index.html > /tmp/crewmux-dashboard-release-check.js
trap 'rm -f /tmp/crewmux-dashboard-release-check.js' EXIT
node --check /tmp/crewmux-dashboard-release-check.js

echo "release preflight passed for v$VERSION"
