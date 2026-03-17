# lynx-mcp

AI browser automation via Chrome accessibility tree. An MCP server that gives LLMs eyes and hands in a real browser.

Inspired by the original [Lynx text browser](https://en.wikipedia.org/wiki/Lynx_(web_browser)) (1992) — because AI doesn't need pixels, it needs structure.

## What it does

- Launches Chrome instances with persistent profiles
- Captures the accessibility tree with stable element refs (`e0`, `e1`, `e2`...)
- Click, type, press keys by ref ID — no pixel coordinates, no fragile selectors
- Auto-dismisses cookie banners and overlays
- Authenticates via 1Password CLI (username + password + TOTP)
- User intervention system — browser banner + macOS notification when login/CAPTCHA detected
- Screenshot and PDF export

## Install

```bash
cargo install --git https://github.com/SeansGravy/lynx-mcp
```

Or build from source:

```bash
git clone https://github.com/SeansGravy/lynx-mcp
cd lynx-mcp
cargo build --release
```

## Register with Claude Code

```bash
claude mcp add lynx_mcp ./target/release/lynx-mcp
```

## Tools (16)

| Tool | Description |
|------|-------------|
| `instance_create` | Launch a Chrome instance with persistent profile |
| `instance_list` | List running instances |
| `instance_destroy` | Destroy an instance |
| `navigate` | Go to a URL, wait for load |
| `snapshot` | Accessibility tree with stable refs — filter, diff, compact modes |
| `text` | Extract readable page text |
| `click` | Click an element by ref — auto-dismisses overlays on failure |
| `type_text` | Type into an input by ref |
| `press` | Press a key (Enter, Tab, Escape...) on an element |
| `upload_file` | Upload files via file input elements |
| `eval` | Execute JavaScript in page context |
| `dismiss_overlays` | Remove cookie banners, modals, popups |
| `wait_for_stable` | Wait for page content to stabilize |
| `screenshot` | Capture page as PNG |
| `pdf` | Export page as PDF |
| `auth_login` | Log in via 1Password credentials |
| `request_intervention` | Ask user to intervene (CAPTCHA, MFA) with browser banner + system notification |

## Requirements

- Rust 1.85+ (edition 2024)
- Chrome or Chromium installed
- macOS or Linux
- 1Password CLI (`op`) for `auth_login` (optional)

## Architecture

```
lynx-mcp
├── server.rs          # MCP tool handlers (rmcp framework)
├── browser/
│   ├── instance.rs    # Chrome lifecycle, CDP commands, intervention system
│   ├── config.rs      # Profile dirs, Chrome path resolution
│   ├── helpers.rs     # Chrome discovery, CDP utilities
│   └── mod.rs         # BrowserManager — instance pool + auto-recovery
├── snapshot/
│   ├── tree.rs        # Accessibility tree → stable refs (persistent monotonic IDs)
│   ├── compact.rs     # Token-efficient output format
│   ├── diff.rs        # Delta between snapshots
│   └── refs.rs        # Ref allocation and tracking
├── auth/
│   ├── form_fill.rs   # Iterative login form detection and filling
│   └── op_cli.rs      # 1Password CLI integration
├── error.rs           # Error types
└── types.rs           # Shared types
```

## License

MIT
