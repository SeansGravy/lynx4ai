# lynx4ai

Rust MCP server for AI browser automation via Chrome DevTools Protocol.

Reads the web through the accessibility tree instead of pixels — like the original [Lynx](https://en.wikipedia.org/wiki/Lynx_(web_browser)) browser (1992) but for AI agents in 2026. Structured, semantic page data at ~800 tokens instead of ~4,000 vision tokens for a screenshot.

## Why "lynx"?

In 1992, [Lynx](https://en.wikipedia.org/wiki/Lynx_(web_browser)) proved you could browse the entire web with nothing but text in a terminal. No images, no CSS, no JavaScript rendering — just the content, fast and clean.

lynx4ai carries that same philosophy forward for AI agents. Instead of rendering pixels and taking screenshots, it reads the web the way a screen reader does — through the accessibility tree. The result: structured, semantic page data at ~800 tokens instead of ~4,000 vision tokens for a screenshot.

Same idea, different era. Text was enough then. Structure is enough now.

### Lineage

- **Lynx (1992)** — The original text-mode browser. Proved the web is content, not rendering.
- **Pinchtab (2025)** — Pioneered accessibility-tree snapshots with stable refs for AI agents.
- **Sean's LLM browser hacks (2025-2026)** — Battle-tested patterns for modal dismissal, response stability detection, and resilient form automation across ChatGPT, Grok, and others.
- **lynx4ai (2026)** — All of the above, in Rust, as an MCP server. One binary. No runtime deps.

## Features

- **16 MCP tools** for complete browser control
- **Accessibility tree snapshots** with stable element refs (e0, e1, e2...)
- **Snapshot diffs** — only see what changed since last snapshot
- **Compact format** — 56-64% fewer tokens than full JSON
- **Interactive filter** — show only clickable/typeable elements (~75% fewer nodes)
- **Auto-dismiss overlays** — cookie banners, modals, popups
- **Page stability detection** — wait until content stops changing
- **1Password integration** — automated login with user/pass + TOTP
- **Persistent Chrome profiles** — stay logged in across sessions
- **Single binary** — no runtime dependencies, just needs Chrome installed

## Install

### One-liner (macOS / Linux)

```bash
curl -fsSL https://raw.githubusercontent.com/SeansGravy/lynx4ai/main/install.sh | bash
```

This will:
- Detect your platform (macOS Intel/Apple Silicon, Linux x86_64/ARM)
- Check for Chrome
- Build from source (installs Rust if needed)
- Install to `~/.local/bin/lynx4ai`
- Print setup instructions for your AI tool

### From source (manual)

```bash
git clone https://github.com/SeansGravy/lynx4ai.git
cd lynx4ai
make install
# or: cargo build --release && cp target/release/lynx4ai ~/.local/bin/
```

### Requirements

- **Chrome or Chromium** — the only runtime dependency
- **Rust 1.75+** — for building from source
- **1Password CLI** (`op`) — optional, for `auth_login` tool

## Setup

### Claude Code

```bash
claude mcp add lynx4ai ~/.local/bin/lynx4ai
```

### Claude Desktop

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "lynx4ai": {
      "command": "~/.local/bin/lynx4ai"
    }
  }
}
```

### Cursor / Windsurf / Codex

Add to `.mcp.json` in your project root:

```json
{
  "mcpServers": {
    "lynx4ai": {
      "command": "~/.local/bin/lynx4ai"
    }
  }
}
```

### Other AI Tools

lynx4ai works with any MCP-compatible tool. Just point the tool's MCP server config at the binary path.

## MCP Tools

### Instance Management
| Tool | Description |
|------|-------------|
| `instance_create` | Launch Chrome with persistent profile (headed/headless) |
| `instance_list` | List running instances with URLs and profile info |
| `instance_destroy` | Kill an instance by ID |

### Navigation & Reading
| Tool | Description |
|------|-------------|
| `navigate` | Go to URL (optional image blocking, configurable wait) |
| `snapshot` | A11y tree with stable refs. Params: `filter`, `diff`, `format`, `selector`, `max_tokens` |
| `text` | Extract readable page text (~800 tokens default) |
| `screenshot` | Base64 PNG screenshot (full page or viewport) |
| `pdf` | Base64 PDF export |

### Element Actions
| Tool | Description |
|------|-------------|
| `click` | Click element by ref (e.g., "e5") |
| `type_text` | Type into element by ref (optional clear_first) |
| `press` | Press key on element (Enter, Tab, Escape, arrows...) |
| `upload_file` | Upload file(s) via file input on page |

### Page Helpers
| Tool | Description |
|------|-------------|
| `eval` | Execute JavaScript, return result as JSON |
| `dismiss_overlays` | Dismiss cookie banners, modals, popups |
| `wait_for_stable` | Wait until page content stabilizes |

### Auth
| Tool | Description |
|------|-------------|
| `auth_login` | Pull creds from password manager, automate login (user/pass + TOTP) |

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `LYNX_HEADLESS` | `true` | Run Chrome in headless mode |
| `LYNX_CHROME_PATH` | auto-detect | Path to Chrome binary |
| `LYNX_PROFILE_DIR` | `~/.lynx4ai/profiles` | Persistent session storage |
| `LYNX_EVAL_ENABLED` | `true` | Enable/disable JavaScript eval tool |
| `LYNX_AUTH_PROVIDER` | `op` | Password manager CLI (1Password) |
| `RUST_LOG` | `lynx4ai=info` | Tracing filter |

## How It Works

1. **Create a browser instance** — launches Chrome with a persistent profile
2. **Navigate** — go to any URL, wait for the accessibility tree to populate
3. **Snapshot** — get the page's accessibility tree as flat JSON with stable element refs
4. **Interact** — click, type, and press keys using element refs from the snapshot
5. **Read** — extract text, take screenshots, or export as PDF

The accessibility tree gives AI agents structured, semantic page data: roles (button, link, textbox), names, values, and interaction states. No vision model needed — just parse the JSON.

### Snapshot Example

```
e0 [link] "Home"
e1 [link] "Products" interactive
e2 [searchbox] "" interactive
e3 [button] "Search" interactive
e4 [heading] "Welcome to Example Store"
e5 [link] "Featured Product" interactive
```

Each element gets a stable ref (e0, e1...) that you can use with `click`, `type_text`, and `press`.

## Building

```bash
# Development
cargo build
cargo test
cargo clippy

# Release (optimized single binary)
cargo build --release
# Binary at ./target/release/lynx4ai
```

## Requirements

- Rust 1.75+
- Chrome or Chromium installed
- 1Password CLI (`op`) for auth features (optional)

## License

MIT

## Credits

- [Pinchtab](https://github.com/pinchtab/pinchtab) — accessibility-tree snapshot pattern with stable refs
- [Lynx](https://en.wikipedia.org/wiki/Lynx_(web_browser)) — the original text-mode web browser (1992)
