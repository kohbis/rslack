# AGENTS.md

This file provides guidance for AI coding agents working on this project.

## Project Overview

**rslack** is a CLI tool for posting and reading Slack messages, written in Rust.

- Interactive channel selection with vim-like navigation
- Multi-line message editor
- Read messages from channels
- Upload images to channels
- Command-line options for direct channel and message specification

## Tech Stack

- **Language**: Rust (Edition 2021)
- **Async Runtime**: Tokio
- **HTTP Client**: reqwest
- **CLI Parser**: clap (with derive feature)
- **Terminal UI**: termion, rpos

## Project Structure

```
src/
├── lib.rs              # Library entry point, module exports
├── bin/
│   └── rslack.rs       # Binary entry point
├── config/
│   └── mod.rs          # Configuration management (token, etc.)
├── console/
│   ├── mod.rs          # Console module exports
│   ├── channel_selector.rs  # Interactive channel selection UI
│   ├── editor.rs       # Multi-line message editor
│   ├── message_viewer.rs    # Message display
│   └── table.rs        # Table rendering
├── option/
│   └── mod.rs          # CLI option parsing (clap)
└── slack/
    └── mod.rs          # Slack API client
```

## Development Guidelines

### Building

```bash
cargo build          # Debug build
cargo build --release # Release build
```

### Testing

```bash
cargo test           # Run all tests
```

Tests are unit tests inside each module. Slack API calls are mocked with `mockito`, serving JSON from `tests/fixtures/`. Tests that modify environment variables use `#[serial]` (`serial_test`) so they don't race.

### Code Style

- CI runs `cargo fmt -- --check` and `cargo test`; run both before committing
- Follow clippy suggestions (not enforced in CI)
- Use `anyhow` for error handling

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| `anyhow` | Error handling |
| `clap` | CLI argument parsing |
| `reqwest` | HTTP client for Slack API |
| `serde` / `serde_json` | JSON serialization |
| `termion` | Terminal manipulation |
| `tokio` | Async runtime |

## Slack API Integration

- Uses Slack Web API
- Requires User Token Scopes: `channels:read`, `channels:history`, `chat:write`, `files:write`
- Token is configured via `RSLACK_TOKEN` env var (takes precedence) or `$XDG_CONFIG_HOME/rslack/config` (default `~/.config/rslack/config`)

## Common Tasks

### Adding a new CLI option

1. Add the option to `src/option/mod.rs` using clap derive macros
2. Handle the option in `src/bin/rslack.rs`

### Adding a new Slack API endpoint

1. Add the API call in `src/slack/mod.rs`
2. Define request/response structs with serde derive

### Modifying the interactive UI

- Channel selection: `src/console/channel_selector.rs`
- Message editor: `src/console/editor.rs`
- Message display: `src/console/message_viewer.rs`
