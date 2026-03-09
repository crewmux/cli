#!/usr/bin/env bash
set -euo pipefail

# CrewMux installer — one-line install:
#   curl -sSL https://raw.githubusercontent.com/crewmux/cli/main/install.sh | bash
# or locally:
#   ./install.sh

RED='\033[0;31m'; GREEN='\033[0;32m'; CYAN='\033[0;36m'; BOLD='\033[1m'; DIM='\033[2m'; NC='\033[0m'
OS="$(uname -s)"
NON_INTERACTIVE=0
if [[ ! -t 0 ]]; then
    NON_INTERACTIVE=1
fi

WANTED_AGENTS="${CM_INSTALL_AGENTS:-${AI_INSTALL_AGENTS:-claude,codex}}"
INSTALL_SERVICE_DEFAULT="${CM_INSTALL_SERVICE:-${AI_INSTALL_SERVICE:-}}"
EXTRA_PATHS=()

echo -e "${BOLD}CrewMux — Installer${NC}"
echo ""

info() {
    echo -e "  ${CYAN}$1${NC}"
}

ok() {
    echo -e "  ${GREEN}$1${NC}"
}

warn() {
    echo -e "  ${DIM}$1${NC}"
}

die() {
    echo -e "${RED}$1${NC}" >&2
    exit 1
}

have() {
    command -v "$1" >/dev/null 2>&1
}

want_agent() {
    [[ ",$WANTED_AGENTS," == *",$1,"* ]]
}

note_extra_path() {
    local path="$1"
    for existing in "${EXTRA_PATHS[@]:-}"; do
        if [[ "$existing" == "$path" ]]; then
            return 0
        fi
    done
    EXTRA_PATHS+=("$path")
}

refresh_shell() {
    hash -r
}

run_as_root() {
    if [[ "$(id -u)" -eq 0 ]]; then
        "$@"
    elif have sudo; then
        sudo "$@"
    else
        die "Administrator privileges are required to install system packages. Install 'sudo' or run this script as root."
    fi
}

ensure_brew_in_path() {
    if have brew; then
        return 0
    fi
    if [[ -x /opt/homebrew/bin/brew ]]; then
        eval "$(/opt/homebrew/bin/brew shellenv)"
    elif [[ -x /usr/local/bin/brew ]]; then
        eval "$(/usr/local/bin/brew shellenv)"
    fi
}

ensure_homebrew() {
    ensure_brew_in_path
    if have brew; then
        return 0
    fi

    [[ "$OS" == "Darwin" ]] || die "Automatic Homebrew installation is only supported on macOS."
    have curl || die "curl is required to install Homebrew."

    info "Installing Homebrew..."
    NONINTERACTIVE=1 /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    ensure_brew_in_path
    have brew || die "Homebrew installation finished, but 'brew' is still not available."
}

linux_pkg_manager() {
    if have apt-get; then
        echo "apt-get"
    elif have dnf; then
        echo "dnf"
    elif have yum; then
        echo "yum"
    elif have pacman; then
        echo "pacman"
    else
        echo ""
    fi
}

install_packages() {
    if [[ "$OS" == "Darwin" ]]; then
        ensure_homebrew
        brew install "$@"
        return 0
    fi

    if [[ "$OS" != "Linux" ]]; then
        die "Automatic package installation is supported on macOS and Linux only."
    fi

    case "$(linux_pkg_manager)" in
        apt-get)
            run_as_root apt-get update
            run_as_root apt-get install -y "$@"
            ;;
        dnf)
            run_as_root dnf install -y "$@"
            ;;
        yum)
            run_as_root yum install -y "$@"
            ;;
        pacman)
            run_as_root pacman -Sy --noconfirm "$@"
            ;;
        *)
            die "No supported package manager found. Install these packages manually: $*"
            ;;
    esac
}

ensure_system_cmd() {
    local cmd="$1"
    shift
    local packages=("$@")

    if have "$cmd"; then
        ok "$cmd found"
        return 0
    fi

    info "Installing $cmd..."
    install_packages "${packages[@]}"
    refresh_shell
    have "$cmd" || die "Installed packages for $cmd, but the command is still missing."
    ok "$cmd installed"
}

node_major() {
    if ! have node; then
        echo 0
        return 0
    fi
    node -p "process.versions.node.split('.')[0]" 2>/dev/null || echo 0
}

ensure_node_runtime() {
    local current_major
    current_major="$(node_major)"
    if have node && have npm && [[ "$current_major" -ge 18 ]]; then
        ok "node found ($(node --version))"
        ok "npm found ($(npm --version))"
        return 0
    fi

    info "Installing Node.js and npm..."
    if [[ "$OS" == "Darwin" ]]; then
        install_packages node
    else
        case "$(linux_pkg_manager)" in
            apt-get|dnf|yum|pacman)
                install_packages nodejs npm
                ;;
            *)
                die "No supported package manager found for Node.js/npm installation."
                ;;
        esac
    fi

    refresh_shell
    current_major="$(node_major)"
    have node || die "Node.js installation finished, but 'node' is still missing."
    have npm || die "npm installation finished, but 'npm' is still missing."
    [[ "$current_major" -ge 18 ]] || die "Node.js 18+ is required for Claude Code / Codex CLI. Current version: $(node --version)"

    ok "node installed ($(node --version))"
    ok "npm installed ($(npm --version))"
}

ensure_rust() {
    if have cargo; then
        ok "cargo found ($(cargo --version))"
        return 0
    fi

    have curl || ensure_system_cmd curl curl
    info "Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck disable=SC1090
    source "$HOME/.cargo/env"
    refresh_shell
    have cargo || die "Rust installation finished, but 'cargo' is still missing."
    ok "cargo installed ($(cargo --version))"
}

ensure_npm_prefix() {
    local prefix

    if [[ -n "${NPM_CONFIG_PREFIX:-}" ]]; then
        mkdir -p "$NPM_CONFIG_PREFIX/bin"
        export PATH="$NPM_CONFIG_PREFIX/bin:$PATH"
        note_extra_path "$NPM_CONFIG_PREFIX/bin"
        return 0
    fi

    prefix="$(npm config get prefix 2>/dev/null || true)"
    if [[ -n "$prefix" && -w "$prefix" ]]; then
        return 0
    fi

    export NPM_CONFIG_PREFIX="$HOME/.npm-global"
    mkdir -p "$NPM_CONFIG_PREFIX/bin"
    export PATH="$NPM_CONFIG_PREFIX/bin:$PATH"
    note_extra_path "$NPM_CONFIG_PREFIX/bin"
}

npm_global_bin_dir() {
    if [[ -n "${NPM_CONFIG_PREFIX:-}" ]]; then
        printf '%s\n' "$NPM_CONFIG_PREFIX/bin"
        return 0
    fi

    local prefix
    prefix="$(npm prefix -g 2>/dev/null || true)"
    if [[ -n "$prefix" ]]; then
        printf '%s/bin\n' "$prefix"
    fi
}

install_npm_cli() {
    local bin="$1"
    local package="$2"

    if have "$bin"; then
        ok "$bin found"
        return 0
    fi

    ensure_node_runtime
    ensure_npm_prefix

    info "Installing $bin via npm ($package)..."
    npm install -g "$package"
    refresh_shell

    local npm_bin
    npm_bin="$(npm_global_bin_dir)"
    if [[ -n "$npm_bin" && -x "$npm_bin/$bin" ]]; then
        export PATH="$npm_bin:$PATH"
        note_extra_path "$npm_bin"
    fi

    have "$bin" || die "Installed $package, but '$bin' is still not available."
    ok "$bin installed"
}

should_install_service() {
    if [[ -n "$INSTALL_SERVICE_DEFAULT" ]]; then
        [[ "$INSTALL_SERVICE_DEFAULT" == "1" || "$INSTALL_SERVICE_DEFAULT" == "true" ]]
        return
    fi

    if [[ "$NON_INTERACTIVE" -eq 1 ]]; then
        return 0
    fi

    echo ""
    read -r -p "  Start dashboard on login? (Y/n) " reply
    [[ ! "$reply" =~ ^[Nn]$ ]]
}

# 1. Install system dependencies
ensure_system_cmd tmux tmux
have curl || ensure_system_cmd curl curl
have git || ensure_system_cmd git git

# 2. Install agent CLIs
if want_agent claude; then
    install_npm_cli claude @anthropic-ai/claude-code
else
    warn "Skipping Claude CLI install (CM_INSTALL_AGENTS=$WANTED_AGENTS)"
fi

if want_agent codex; then
    install_npm_cli codex @openai/codex
else
    warn "Skipping Codex CLI install (CM_INSTALL_AGENTS=$WANTED_AGENTS)"
fi

if ! have claude && ! have codex; then
    die "No agent CLI is available after installation. At least one of 'claude' or 'codex' must be present."
fi

# 3. Install Rust
ensure_rust

# 4. Build
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
echo ""
info "Building release..."
cd "$SCRIPT_DIR"
cargo build --release --quiet

# 5. Install binaries
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"
install -m 755 "$SCRIPT_DIR/target/release/cm" "$INSTALL_DIR/cm"
ln -sf "$INSTALL_DIR/cm" "$INSTALL_DIR/ai"
ok "Installed $INSTALL_DIR/cm"
ok "Linked $INSTALL_DIR/ai -> $INSTALL_DIR/cm"
note_extra_path "$INSTALL_DIR"

# 6. Create data directory
mkdir -p "$HOME/.crewmux"/{logs,tasks,service}

# 7. Install as background service
if should_install_service; then
    "$INSTALL_DIR/cm" install
else
    warn "Skipped background service installation"
fi

echo ""
echo -e "${GREEN}${BOLD}Done!${NC}"
echo ""
echo -e "  ${BOLD}Quick start:${NC}"
echo -e "    ${CYAN}cm team start${NC}                        Start a team session"
echo -e "    ${CYAN}cm task spawn -t codex -m gpt-5.3-codex${NC} \"fix auth\"   Spawn worker"
echo -e "    ${CYAN}cm ctl status${NC}                        Check status"
echo -e "    ${CYAN}cm web${NC}                               Open dashboard manually"
echo -e "    ${CYAN}open http://localhost:7700${NC}            Dashboard (if installed as service)"

if ((${#EXTRA_PATHS[@]} > 0)); then
    echo ""
    echo -e "  ${DIM}Add these paths to your shell profile if they are not already present:${NC}"
    for path in "${EXTRA_PATHS[@]}"; do
        echo -e "  ${CYAN}export PATH=\"$path:\$PATH\"${NC}"
    done
fi

echo ""
