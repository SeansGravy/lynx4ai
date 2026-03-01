# AGENTS.md — lynx4ai

## Overview
Rust MCP server for Chrome browser automation via CDP (Chrome DevTools Protocol).
Reads the web through the accessibility tree instead of pixels — optimized for AI agents.

## Build & Run
```bash
cargo build --release
# Binary at ./target/release/lynx4ai
# Communicates via stdio (MCP JSON-RPC protocol)
```

## Architecture
- `rmcp` for MCP server (stdio transport)
- `chromiumoxide` for Chrome CDP
- All tracing goes to STDERR — NEVER use println!() (corrupts MCP stdio)
- Accessibility tree snapshots with stable element refs (e0, e1, e2...)
- Password manager CLI (`op` default) for credential retrieval

## Key Patterns
- `BrowserManager` holds instances behind `Arc<RwLock<HashMap>>`
- `RefMap` maps element refs to `BackendNodeId` for click/type resolution
- Snapshot diff caches last tree per instance
- Auth form fill is iterative (handles multi-page logins)
- Actions auto-dismiss overlays before interacting

## Testing
```bash
cargo test                    # Unit tests
cargo test -- --ignored       # Integration tests (needs Chrome)
cargo clippy -- -D warnings   # Lint
```

## Environment Variables
| Var | Default | Purpose |
|-----|---------|---------|
| `LYNX_HEADLESS` | `true` | Headless or headed Chrome |
| `LYNX_CHROME_PATH` | auto-detect | Chrome binary path |
| `LYNX_PROFILE_DIR` | `~/.lynx4ai/profiles` | Persistent session storage |
| `LYNX_EVAL_ENABLED` | `true` | Enable/disable JS eval tool |
| `LYNX_AUTH_PROVIDER` | `op` | Password manager CLI |
| `RUST_LOG` | `lynx4ai=info` | Tracing filter |

## MCP Tools (16 total)
- **Instance**: `instance_create`, `instance_list`, `instance_destroy`
- **Navigation**: `navigate`, `snapshot`, `text`, `screenshot`, `pdf`
- **Actions**: `click`, `type_text`, `press`, `upload_file`
- **Helpers**: `eval`, `dismiss_overlays`, `wait_for_stable`
- **Auth**: `auth_login`
