#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
    echo "usage: $0 <tap-dir-or-name> [formula-path]" >&2
    echo "example: $0 crewmux/tap" >&2
    exit 1
fi

TARGET="$1"
SOURCE="${2:-Formula/crewmux.rb}"

if [[ ! -f "$SOURCE" ]]; then
    echo "formula not found: $SOURCE" >&2
    exit 1
fi

resolve_tap_dir() {
    local tap_name="$1"
    local owner="${tap_name%%/*}"
    local repo="${tap_name#*/}"

    if [[ "$owner" == "$repo" ]]; then
        echo "tap name must be in <owner>/<repo> format" >&2
        exit 1
    fi

    local tap_dir
    tap_dir="$(brew --repository)/Library/Taps/${owner}/homebrew-${repo}"
    if [[ ! -d "$tap_dir" ]]; then
        brew tap-new --no-git "$tap_name" >/dev/null
    fi
    printf '%s\n' "$tap_dir"
}

if [[ "$TARGET" == */* && ! -d "$TARGET" ]]; then
    TAP_DIR="$(resolve_tap_dir "$TARGET")"
else
    TAP_DIR="$TARGET"
fi

mkdir -p "$TAP_DIR/Formula"
cp "$SOURCE" "$TAP_DIR/Formula/crewmux.rb"

printf 'Synced formula to %s\n' "$TAP_DIR/Formula/crewmux.rb"
