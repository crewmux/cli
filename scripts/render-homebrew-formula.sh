#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
    echo "usage: $0 <version> <sha256> [url]" >&2
    exit 1
fi

VERSION="$1"
SHA256="$2"
URL="${3:-https://github.com/crewmux/client/archive/refs/tags/v${VERSION}.tar.gz}"

cat <<EOF
class Crewmux < Formula
  desc "Multi-agent orchestration for tmux-powered teams"
  homepage "https://github.com/crewmux/client"
  url "${URL}"
  sha256 "${SHA256}"
  version "${VERSION}"

  depends_on "rust" => :build
  depends_on "tmux"

  def install
    system "cargo", "install", *std_cargo_args(root: libexec)

    installed_bins = Dir[libexec/"bin/*"]
    odie "No executable found in #{libexec}/bin" if installed_bins.empty?

    bin.install installed_bins.first => "crewmux"
  end

  def caveats
    <<~EOS
      CrewMux needs at least one agent CLI on your PATH at runtime:
        - claude
        - codex

      tmux is installed automatically as a Homebrew dependency.
    EOS
  end

  test do
    assert_match "Usage: crewmux", shell_output("#{bin}/crewmux --help")
    assert_match "Start a new AI team session", shell_output("#{bin}/crewmux team --help")
  end
end
EOF
