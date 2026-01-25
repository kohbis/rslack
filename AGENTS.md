# AGENTS.md

This file provides guidance for AI coding agents working on this project.

## Project Overview

**rslack** is a CLI tool for posting and reading Slack messages, written in Rust.

- Interactive channel selection with vim-like navigation
- Multi-line message editor
- Read messages from channels
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

Test fixtures are located in `tests/fixtures/`.

### Code Style

- Follow standard Rust conventions (rustfmt, clippy)
- Use `anyhow` for error handling
- Async functions should use `async`/`await` with Tokio runtime

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
- Requires User Token Scopes: `channels:read`, `channels:history`, `chat:write`
- Token is configured via `RSLACK_TOKEN` env var or `~/.rslack` config file

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
