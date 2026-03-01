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

## Use Cases

### Cross-LLM Automation

Use one AI to drive another through the browser. lynx4ai gives any MCP-capable agent full browser control — including navigating to other LLM web interfaces.

- **Claude driving ChatGPT** — Use Claude Code with lynx4ai to navigate to chat.openai.com, type prompts, read responses, compare outputs
- **Codex driving Claude** — Have OpenAI's Codex CLI use lynx4ai to interact with Claude's web UI
- **Multi-model comparison** — Write a script that sends the same prompt to ChatGPT, Gemini, and Grok via their web UIs, then collects and compares all responses
- **Best-of-N routing** — Agent navigates to multiple LLM interfaces, asks each the same question, picks the best answer
- **LLM-as-judge via browser** — One model evaluates another model's web UI output by reading the accessibility tree

### Authenticated Web Portals

Persistent Chrome profiles mean you log in once and stay logged in. Combine with `auth_login` for fully automated credential entry.

- **Medical portals** — Pull lab results, appointment history, medication lists from MyChart or similar
- **Corporate intranets** — Navigate internal dashboards, HR systems, IT portals, wiki pages, ticketing systems (ServiceNow, Jira, Confluence) — anything your browser can reach, lynx4ai can read
- **Banking & finance** — Check balances, download statements, monitor transactions
- **Government portals** — Tax filings, benefits status, license renewals
- **SaaS dashboards** — Pull data from admin panels, analytics dashboards, CRM systems

### Data Extraction & Research

- **Competitive analysis** — Navigate competitor sites, extract pricing, features, product specs into structured data
- **Job board scraping** — Search multiple job sites, extract listings, filter and organize results
- **Academic research** — Navigate journals, pull abstracts, build citation lists
- **Real estate / listings** — Browse Zillow, Redfin, or rental sites and extract structured property data
- **Price monitoring** — Check prices across multiple retailers, track changes over time

### Form Automation

- **Repetitive form filling** — Insurance claims, expense reports, vendor onboarding, compliance forms
- **Multi-step wizards** — Walk through multi-page signup flows, checkout processes, application forms
- **Bulk operations** — Submit the same form across multiple accounts or platforms

### Testing & QA

- **AI-driven E2E testing** — Let your AI agent navigate your app and verify behavior through the accessibility tree
- **Accessibility auditing** — The snapshot IS the accessibility tree — use it to check if your site's a11y roles, names, and states are correct
- **Regression monitoring** — Snapshot a page periodically, use `diff` mode to detect unexpected changes

### Content & Social

- **Social media management** — Post, read, and manage content across platforms through their web UIs
- **Email automation** — Navigate webmail, read/compose/send through the browser
- **CMS operations** — Create, edit, publish content in WordPress, Ghost, or any CMS

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
- Install Chrome if missing (via Homebrew on macOS, apt/dnf/pacman on Linux)
- Install Rust if needed (via rustup)
- Build from source and install to `~/.local/bin/lynx4ai`
- Print setup instructions for your AI tool

### From source (manual)

```bash
git clone https://github.com/SeansGravy/lynx4ai.git
cd lynx4ai
make install
# or: cargo build --release && cp target/release/lynx4ai ~/.local/bin/
```

### Requirements

- **Chrome or Chromium** — the only runtime dependency (installer handles this)
- **Rust 1.75+** — for building from source (installer handles this)
- **1Password CLI** (`op`) — optional, for `auth_login` tool

## Setup

lynx4ai works with any AI tool that supports MCP (Model Context Protocol) over stdio. Here's how to set it up for each one.

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

### OpenAI Codex CLI

```bash
codex mcp add lynx4ai -- ~/.local/bin/lynx4ai
```

Or add to `~/.codex/config.toml`:

```toml
[mcp_servers.lynx4ai]
command = "~/.local/bin/lynx4ai"
```

### ChatGPT Desktop

ChatGPT Desktop only supports remote (HTTPS) MCP servers, not local stdio. To use lynx4ai with ChatGPT, you'd need to wrap it in an HTTP bridge like [mcp-proxy](https://github.com/nichochar/mcp-proxy) or [mcp.run](https://www.mcp.run).

### Cursor

Add to `~/.cursor/mcp.json` (global) or `.cursor/mcp.json` (project):

```json
{
  "mcpServers": {
    "lynx4ai": {
      "command": "~/.local/bin/lynx4ai"
    }
  }
}
```

### Windsurf

Add to `~/.codeium/windsurf/mcp_config.json`:

```json
{
  "mcpServers": {
    "lynx4ai": {
      "command": "~/.local/bin/lynx4ai"
    }
  }
}
```

Or click the MCP icon in the Cascade panel and select "Configure".

### GitHub Copilot (VS Code)

Add to `.vscode/mcp.json` in your project:

```json
{
  "servers": {
    "lynx4ai": {
      "command": "~/.local/bin/lynx4ai"
    }
  }
}
```

> **Note:** VS Code uses `"servers"` as the top-level key, not `"mcpServers"`.

Or via command line:

```bash
code --add-mcp '{"name":"lynx4ai","command":"~/.local/bin/lynx4ai"}'
```

### Gemini CLI

```bash
gemini mcp add lynx4ai ~/.local/bin/lynx4ai
```

Or add to `~/.gemini/settings.json`:

```json
{
  "mcpServers": {
    "lynx4ai": {
      "command": "~/.local/bin/lynx4ai"
    }
  }
}
```

### Grok (xAI)

The xAI API supports remote MCP servers only (HTTP/SSE). For local stdio, use the third-party [grok-cli](https://github.com/superagent-ai/grok-cli):

```bash
grok mcp add lynx4ai --transport stdio --command ~/.local/bin/lynx4ai
```

Or add to `.grok/settings.json`:

```json
{
  "mcpServers": {
    "lynx4ai": {
      "transport": "stdio",
      "command": "~/.local/bin/lynx4ai"
    }
  }
}
```

### Amazon Q Developer

```bash
q mcp add --name lynx4ai --command ~/.local/bin/lynx4ai
```

Or add to `~/.aws/amazonq/mcp.json`:

```json
{
  "mcpServers": {
    "lynx4ai": {
      "command": "~/.local/bin/lynx4ai"
    }
  }
}
```

### Cline (VS Code)

Open the Cline sidebar in VS Code, click the MCP Servers icon, select "Configure", and add:

```json
{
  "mcpServers": {
    "lynx4ai": {
      "command": "~/.local/bin/lynx4ai",
      "disabled": false
    }
  }
}
```

### Quick Reference

| Tool | Config | CLI Shortcut |
|------|--------|-------------|
| Claude Code | automatic | `claude mcp add lynx4ai ~/.local/bin/lynx4ai` |
| Claude Desktop | `claude_desktop_config.json` | — |
| Codex CLI | `~/.codex/config.toml` | `codex mcp add lynx4ai -- ~/.local/bin/lynx4ai` |
| ChatGPT Desktop | HTTP only (needs bridge) | — |
| Cursor | `~/.cursor/mcp.json` | — |
| Windsurf | `~/.codeium/windsurf/mcp_config.json` | — |
| VS Code Copilot | `.vscode/mcp.json` | `code --add-mcp '{...}'` |
| Gemini CLI | `~/.gemini/settings.json` | `gemini mcp add lynx4ai ~/.local/bin/lynx4ai` |
| Grok (grok-cli) | `.grok/settings.json` | `grok mcp add lynx4ai --transport stdio ...` |
| Amazon Q | `~/.aws/amazonq/mcp.json` | `q mcp add --name lynx4ai --command ...` |
| Cline | VS Code globalStorage | — |

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

## License

MIT

## Credits

- [Pinchtab](https://github.com/pinchtab/pinchtab) — accessibility-tree snapshot pattern with stable refs
- [Lynx](https://en.wikipedia.org/wiki/Lynx_(web_browser)) — the original text-mode web browser (1992)
