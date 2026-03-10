#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
    echo "usage: $0 <version> <output-formula-path> [archive-url]" >&2
    echo "example: $0 0.1.0 /tmp/crewmux.rb" >&2
    exit 1
fi

VERSION="$1"
OUTPUT_PATH="$2"
ARCHIVE_URL="${3:-https://github.com/crewmux/cli/archive/refs/tags/v${VERSION}.tar.gz}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

ARCHIVE_PATH="$TMP_DIR/crewmux-v${VERSION}.tar.gz"

curl -fsSL "$ARCHIVE_URL" -o "$ARCHIVE_PATH"
SHA256="$(shasum -a 256 "$ARCHIVE_PATH" | awk '{print $1}')"

mkdir -p "$(dirname "$OUTPUT_PATH")"
"$REPO_ROOT/scripts/render-homebrew-formula.sh" "$VERSION" "$SHA256" "$ARCHIVE_URL" > "$OUTPUT_PATH"

printf 'Rendered stable formula for v%s\n' "$VERSION"
printf 'Archive URL: %s\n' "$ARCHIVE_URL"
printf 'SHA256: %s\n' "$SHA256"
printf 'Output: %s\n' "$OUTPUT_PATH"
