#!/usr/bin/env bash
set -euo pipefail

# ai-ctl installer — one-line install:
#   curl -sSL https://raw.githubusercontent.com/YOUR/ai-ctl/main/install.sh | bash
# or locally:
#   ./install.sh

RED='\033[0;31m'; GREEN='\033[0;32m'; CYAN='\033[0;36m'; BOLD='\033[1m'; DIM='\033[2m'; NC='\033[0m'

echo -e "${BOLD}AI Team Controller — Installer${NC}"
echo ""

# 1. Check dependencies
check_dep() {
    if ! command -v "$1" &>/dev/null; then
        echo -e "${RED}Missing: $1${NC}. $2"
        return 1
    fi
}

check_dep tmux "Install with: brew install tmux" || exit 1
echo -e "  ${GREEN}tmux${NC} found"

# 2. Check/install Rust
if ! command -v cargo &>/dev/null; then
    echo -e "  ${CYAN}Installing Rust...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi
echo -e "  ${GREEN}cargo${NC} found ($(cargo --version))"

# 3. Build
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
echo ""
echo -e "  ${CYAN}Building release...${NC}"
cd "$SCRIPT_DIR"
cargo build --release --quiet 2>&1

# 4. Install binary
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"
cp "$SCRIPT_DIR/target/release/ai" "$INSTALL_DIR/ai"
chmod +x "$INSTALL_DIR/ai"
echo -e "  ${GREEN}Installed${NC} $INSTALL_DIR/ai"

# 5. Check PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo ""
    echo -e "  ${DIM}Add to your shell profile:${NC}"
    echo -e "  ${CYAN}export PATH=\"\$HOME/.local/bin:\$PATH\"${NC}"
fi

# 6. Create data directory
mkdir -p "$HOME/.ai-team"/{logs,tasks,service}

# 7. Install as background service
echo ""
read -p "  Start dashboard on login? (Y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Nn]$ ]]; then
    "$INSTALL_DIR/ai" install
fi

echo ""
echo -e "${GREEN}${BOLD}Done!${NC}"
echo ""
echo -e "  ${BOLD}Quick start:${NC}"
echo -e "    ${CYAN}ai team start${NC}                        Start a team session"
echo -e "    ${CYAN}ai task spawn -t codex -m codex-5.4${NC} \"fix auth\"   Spawn worker"
echo -e "    ${CYAN}ai ctl status${NC}                        Check status"
echo -e "    ${CYAN}ai web${NC}                               Open dashboard manually"
echo -e "    ${CYAN}open http://localhost:7700${NC}            Dashboard (if installed as service)"
echo ""
