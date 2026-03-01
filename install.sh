#!/usr/bin/env bash
# lynx4ai installer for macOS
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

# --- Check for Chrome ---
CHROME_FOUND=false
if [ "$OS" = "Darwin" ]; then
    if [ -x "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" ]; then
        CHROME_FOUND=true
        CHROME_VERSION=$("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --version 2>/dev/null || echo "unknown")
        ok "Chrome found: $CHROME_VERSION"
    fi
else
    for cmd in google-chrome chromium-browser chromium; do
        if command -v "$cmd" &>/dev/null; then
            CHROME_FOUND=true
            CHROME_VERSION=$("$cmd" --version 2>/dev/null || echo "unknown")
            ok "Chrome found: $CHROME_VERSION"
            break
        fi
    done
fi

if [ "$CHROME_FOUND" = false ]; then
    warn "Chrome not found. lynx4ai requires Chrome or Chromium to run."
    warn "Install Chrome: https://www.google.com/chrome/"
fi

# --- Check for Rust toolchain (build from source) ---
BUILD_FROM_SOURCE=false
RELEASE_URL=""

# Try to find a pre-built release first
info "Checking for pre-built release..."
LATEST_TAG=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/' || echo "")

if [ -n "$LATEST_TAG" ]; then
    ASSET_NAME="${BINARY}-${TARGET}"
    RELEASE_URL="https://github.com/$REPO/releases/download/$LATEST_TAG/$ASSET_NAME"
    # Check if asset exists
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
        warn "Rust toolchain not found. Installing via rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --quiet
        source "$HOME/.cargo/env"
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

# --- PATH check ---
if ! echo "$PATH" | tr ':' '\n' | grep -q "^$INSTALL_DIR$"; then
    warn "$INSTALL_DIR is not in your PATH"
    echo ""
    echo -e "  Add to your shell profile (~/.zshrc or ~/.bashrc):"
    echo ""
    echo -e "    ${CYAN}export PATH=\"$INSTALL_DIR:\$PATH\"${NC}"
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

# --- Optional: 1Password CLI check ---
if command -v op &>/dev/null; then
    OP_VERSION=$(op --version 2>/dev/null || echo "unknown")
    ok "1Password CLI found: v$OP_VERSION (auth_login tool enabled)"
else
    echo -e "  ${DIM}Optional: install 1Password CLI for auth_login tool:${NC}"
    echo -e "  ${DIM}  brew install --cask 1password-cli${NC}"
    echo ""
fi

ok "Installation complete."
echo ""
