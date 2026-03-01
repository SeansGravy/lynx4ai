#!/usr/bin/env bash
# lynx4ai installer for macOS / Linux
# Usage: curl -fsSL https://raw.githubusercontent.com/SeansGravy/lynx4ai/main/install.sh | bash
set -euo pipefail

REPO="SeansGravy/lynx4ai"
BINARY="lynx4ai"
INSTALL_DIR="${LYNX_INSTALL_DIR:-$HOME/.local/bin}"

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
DIM='\033[2m'
BOLD='\033[1m'
NC='\033[0m'

info()  { echo -e "${CYAN}[lynx4ai]${NC} $*"; }
ok()    { echo -e "${GREEN}[lynx4ai]${NC} $*"; }
warn()  { echo -e "${YELLOW}[lynx4ai]${NC} $*"; }
fail()  { echo -e "${RED}[lynx4ai]${NC} $*"; exit 1; }

# --- Banner ---
echo ""
echo -e "${BOLD}  ╦  ╦ ╦╔╗╔╦ ╦ ╦  ╔═╗╦${NC}"
echo -e "${BOLD}  ║  ╚╦╝║║║╠╩╗║══╠═╣║${NC}"
echo -e "${BOLD}  ╩═╝ ╩ ╝╚╝╩ ╩   ╩ ╩╩${NC}"
echo -e "${DIM}  AI browser automation via accessibility tree${NC}"
echo -e "${DIM}  Like Lynx (1992) but for AI agents${NC}"
echo ""

# --- Detect platform ---
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Darwin) PLATFORM="apple-darwin" ;;
    Linux)  PLATFORM="unknown-linux-gnu" ;;
    *)      fail "Unsupported OS: $OS (macOS and Linux only)" ;;
esac

case "$ARCH" in
    x86_64)  TARGET="x86_64-$PLATFORM" ;;
    aarch64|arm64) TARGET="aarch64-$PLATFORM" ;;
    *)       fail "Unsupported architecture: $ARCH" ;;
esac

info "Platform: ${BOLD}$OS $ARCH${NC} ($TARGET)"

# ============================================================
# Dependencies
# ============================================================

# --- Homebrew (macOS only, needed for Chrome + git if missing) ---
install_homebrew() {
    if [ "$OS" = "Darwin" ] && ! command -v brew &>/dev/null; then
        info "Installing Homebrew..."
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
        # Add brew to PATH for this session
        if [ -x "/opt/homebrew/bin/brew" ]; then
            eval "$(/opt/homebrew/bin/brew shellenv)"
        elif [ -x "/usr/local/bin/brew" ]; then
            eval "$(/usr/local/bin/brew shellenv)"
        fi
        ok "Homebrew installed"
    fi
}

# --- Git ---
if ! command -v git &>/dev/null; then
    info "Git not found — installing..."
    if [ "$OS" = "Darwin" ]; then
        # xcode-select --install provides git on macOS, or use brew
        if command -v brew &>/dev/null; then
            brew install git
        else
            install_homebrew
            brew install git
        fi
    else
        if command -v apt-get &>/dev/null; then
            sudo apt-get update -qq && sudo apt-get install -y -qq git
        elif command -v dnf &>/dev/null; then
            sudo dnf install -y git
        elif command -v pacman &>/dev/null; then
            sudo pacman -Sy --noconfirm git
        else
            fail "Cannot install git — please install it manually"
        fi
    fi
    ok "Git installed: $(git --version)"
else
    ok "Git found: $(git --version)"
fi

# --- Chrome / Chromium ---
CHROME_FOUND=false
CHROME_PATH=""

find_chrome() {
    if [ "$OS" = "Darwin" ]; then
        for app in \
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
            "/Applications/Chromium.app/Contents/MacOS/Chromium" \
            "/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary"; do
            if [ -x "$app" ]; then
                CHROME_FOUND=true
                CHROME_PATH="$app"
                return
            fi
        done
    else
        for cmd in google-chrome google-chrome-stable chromium-browser chromium; do
            if command -v "$cmd" &>/dev/null; then
                CHROME_FOUND=true
                CHROME_PATH="$(command -v "$cmd")"
                return
            fi
        done
    fi
}

find_chrome

if [ "$CHROME_FOUND" = true ]; then
    CHROME_VERSION=$("$CHROME_PATH" --version 2>/dev/null || echo "unknown")
    ok "Chrome found: $CHROME_VERSION"
else
    info "Chrome not found — installing..."
    if [ "$OS" = "Darwin" ]; then
        install_homebrew
        brew install --cask google-chrome
        ok "Chrome installed via Homebrew"
    else
        if command -v apt-get &>/dev/null; then
            # Debian/Ubuntu
            info "Installing Chrome via apt..."
            curl -fsSL https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb -o /tmp/chrome.deb
            sudo apt-get install -y -qq /tmp/chrome.deb
            rm -f /tmp/chrome.deb
        elif command -v dnf &>/dev/null; then
            # Fedora/RHEL
            info "Installing Chromium via dnf..."
            sudo dnf install -y chromium
        elif command -v pacman &>/dev/null; then
            # Arch
            info "Installing Chromium via pacman..."
            sudo pacman -Sy --noconfirm chromium
        else
            warn "Could not auto-install Chrome. Please install manually:"
            warn "  https://www.google.com/chrome/"
        fi
    fi

    # Re-check
    find_chrome
    if [ "$CHROME_FOUND" = true ]; then
        CHROME_VERSION=$("$CHROME_PATH" --version 2>/dev/null || echo "unknown")
        ok "Chrome installed: $CHROME_VERSION"
    else
        warn "Chrome not detected after install attempt."
        warn "lynx4ai requires Chrome or Chromium at runtime."
    fi
fi

# --- Rust toolchain ---
# (only needed if building from source — checked below)

# ============================================================
# Install lynx4ai
# ============================================================

# Try to find a pre-built release first
BUILD_FROM_SOURCE=false
RELEASE_URL=""

info "Checking for pre-built release..."
LATEST_TAG=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null \
    | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/' || echo "")

if [ -n "$LATEST_TAG" ]; then
    ASSET_NAME="${BINARY}-${TARGET}"
    RELEASE_URL="https://github.com/$REPO/releases/download/$LATEST_TAG/$ASSET_NAME"
    HTTP_CODE=$(curl -sI -o /dev/null -w "%{http_code}" "$RELEASE_URL" 2>/dev/null || echo "000")
    if [ "$HTTP_CODE" != "200" ] && [ "$HTTP_CODE" != "302" ]; then
        RELEASE_URL=""
    fi
fi

if [ -n "$RELEASE_URL" ]; then
    info "Found pre-built binary: $LATEST_TAG"
else
    info "No pre-built binary found — building from source"
    BUILD_FROM_SOURCE=true

    if ! command -v cargo &>/dev/null; then
        info "Rust not found — installing via rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --quiet
        # Source cargo env for this session
        if [ -f "$HOME/.cargo/env" ]; then
            source "$HOME/.cargo/env"
        fi
        ok "Rust installed: $(rustc --version)"
    else
        ok "Rust found: $(rustc --version)"
    fi
fi

# --- Install directory ---
mkdir -p "$INSTALL_DIR"
info "Install directory: ${BOLD}$INSTALL_DIR${NC}"

# --- Download or build ---
if [ "$BUILD_FROM_SOURCE" = true ]; then
    TMPDIR=$(mktemp -d)
    trap 'rm -rf "$TMPDIR"' EXIT

    info "Cloning $REPO..."
    git clone --depth 1 "https://github.com/$REPO.git" "$TMPDIR/lynx4ai" 2>/dev/null

    info "Building release binary (this may take a minute)..."
    cd "$TMPDIR/lynx4ai"
    cargo build --release 2>&1 | tail -1

    cp "target/release/$BINARY" "$INSTALL_DIR/$BINARY"
    ok "Built and installed: $INSTALL_DIR/$BINARY"
else
    info "Downloading $RELEASE_URL..."
    curl -fsSL "$RELEASE_URL" -o "$INSTALL_DIR/$BINARY"
    ok "Downloaded: $INSTALL_DIR/$BINARY"
fi

chmod +x "$INSTALL_DIR/$BINARY"

# --- Verify ---
INSTALLED_PATH="$INSTALL_DIR/$BINARY"
if [ -x "$INSTALLED_PATH" ]; then
    SIZE=$(ls -lh "$INSTALLED_PATH" | awk '{print $5}')
    ok "Binary installed: ${BOLD}$INSTALLED_PATH${NC} ($SIZE)"
else
    fail "Installation failed — binary not found at $INSTALLED_PATH"
fi

# ============================================================
# Post-install
# ============================================================

# --- PATH check ---
if ! echo "$PATH" | tr ':' '\n' | grep -q "^$INSTALL_DIR$"; then
    warn "$INSTALL_DIR is not in your PATH"
    echo ""
    SHELL_NAME="$(basename "$SHELL" 2>/dev/null || echo "bash")"
    case "$SHELL_NAME" in
        zsh)  RC_FILE="~/.zshrc" ;;
        fish) RC_FILE="~/.config/fish/config.fish" ;;
        *)    RC_FILE="~/.bashrc" ;;
    esac
    echo -e "  Add to ${BOLD}$RC_FILE${NC}:"
    echo ""
    if [ "$SHELL_NAME" = "fish" ]; then
        echo -e "    ${CYAN}fish_add_path $INSTALL_DIR${NC}"
    else
        echo -e "    ${CYAN}export PATH=\"$INSTALL_DIR:\$PATH\"${NC}"
    fi
    echo ""
fi

# --- MCP config examples ---
echo ""
echo -e "${BOLD}Setup for your AI tool:${NC}"
echo ""
echo -e "  ${CYAN}Claude Code${NC} — run this command:"
echo ""
echo -e "    claude mcp add lynx4ai $INSTALLED_PATH"
echo ""
echo -e "  ${CYAN}Claude Desktop${NC} — add to claude_desktop_config.json:"
echo ""
echo -e '    "lynx4ai": { "command": "'$INSTALLED_PATH'" }'
echo ""
echo -e "  ${CYAN}Cursor / Windsurf / Codex${NC} — add to .mcp.json:"
echo ""
echo -e '    { "mcpServers": { "lynx4ai": { "command": "'$INSTALLED_PATH'" } } }'
echo ""

# --- Optional deps ---
if command -v op &>/dev/null; then
    OP_VERSION=$(op --version 2>/dev/null || echo "unknown")
    ok "1Password CLI found: v$OP_VERSION (auth_login tool enabled)"
else
    echo -e "  ${DIM}Optional: install 1Password CLI for auth_login tool:${NC}"
    if [ "$OS" = "Darwin" ]; then
        echo -e "  ${DIM}  brew install --cask 1password-cli${NC}"
    else
        echo -e "  ${DIM}  https://developer.1password.com/docs/cli/get-started/${NC}"
    fi
    echo ""
fi

# --- Summary ---
echo -e "${BOLD}Installed:${NC}"
echo -e "  ${GREEN}lynx4ai${NC}  $INSTALLED_PATH ($SIZE)"
[ "$CHROME_FOUND" = true ] && echo -e "  ${GREEN}chrome${NC}   $CHROME_VERSION"
echo -e "  ${GREEN}git${NC}      $(git --version 2>/dev/null)"
command -v cargo &>/dev/null && echo -e "  ${GREEN}rust${NC}     $(rustc --version 2>/dev/null)"
command -v op &>/dev/null && echo -e "  ${GREEN}op${NC}       v$(op --version 2>/dev/null) (optional)"
echo ""

ok "Installation complete."
echo ""
