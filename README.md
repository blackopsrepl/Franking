<div align="center">

  <img src="assets/mascot.png" alt="SolverForge Mail mascot" width="320" />

  <br />

  [![CI](https://github.com/blackopsrepl/solverforge-mail/actions/workflows/ci.yml/badge.svg?style=for-the-badge)](https://github.com/blackopsrepl/solverforge-mail/actions/workflows/ci.yml)
  [![Version](https://img.shields.io/badge/version-v0.2.0-00E6A8?style=for-the-badge)](https://github.com/blackopsrepl/solverforge-mail)
  [![Rust](https://img.shields.io/badge/rust-stable-orange?style=for-the-badge)](https://www.rust-lang.org)
  [![Built With Ratatui](https://img.shields.io/badge/built%20with-ratatui-5A54FF?style=for-the-badge)](https://ratatui.rs/)

</div>

# SolverForge Mail

A spiffy ratatui-based TUI email client with an app-owned mail layer, native
maildir support, and a temporary Himalaya migration adapter for legacy
accounts.

## Quick Start

```bash
# Run from source with a specific account
cargo run -- --account test
cargo run -- --account icloud

# Set up accounts
cargo run -- --setup
```

## Features

- **Non-blocking I/O** - Background workers for all mail operations
- **Relative timestamps** - "2h ago", "Yesterday", "Mon"
- **Threading support** - Press `t` to toggle threaded view
- **Auto-refresh** - New mail check every 60 seconds
- **Folder unread counts** - Shows (3) badge on folders
- **Mouse support** - Click to select, scroll wheel works
- **Multi-account** - Switch with Ctrl+a
- **Fast keyboard navigation** - j/k and g/G in list/message views, plus direct multiline editing in compose
- **Smart error handling** - Typed mail diagnostics and clean user-facing errors
- **Structured message reader** - MIME-aware message content with `Auto`, `Plain`, and `HTML` modes
- **External HTML open** - Safe argv-based browser handoff for full-fidelity HTML viewing
- **Address book** - Contacts with name, email, phone, org, notes, tags
- **Contact import** - vCard (.vcf) and Google CSV import
- **Auto-harvest contacts** - Captured from sent/received mail
- **Sender identities** - Multiple From addresses per account with default
- **Local SQLite database** - Contacts and identities stored in `~/.local/share/solverforge/mail.db`
- **App-owned account store** - Accounts, endpoints, auth bindings, and secret references live in SQLite
- **Keyring-backed secrets** - Password and app-password flows store secret IDs in the app and raw secrets in the OS keyring

## Keybindings

### Global
- `Ctrl+c` / `Ctrl+q` - Quit
- `Ctrl+a` - Switch account
- `Ctrl+r` - Refresh
- `?` - Help

### Envelope List
- `j`/`k` - Navigate up/down
- `Enter` - Read message
- `c` - Compose new
- `d` - Delete
- `m` - Move to folder
- `!` - Toggle flag
- `t` - Toggle threaded view
- `/` - Search
- `Tab` - Focus folders
- `Ctrl+b` - Open contacts
- `I` - Open identities

### Message View
- `j`/`k` - Scroll
- `q`/`Esc` - Back to list
- `r` - Reply
- `R` - Reply all
- `f` - Forward
- `d` - Delete
- `a` - Download attachments
- `1` / `2` / `3` - `Auto` / `Plain` / `HTML`
- `o` - Open HTML externally

### Compose View
- `Tab` / `Shift+Tab` - Next/previous compose field
- `Up` / `Down` in headers or action bar - Previous/next compose field
- Typing in header text fields edits them directly
- `Enter` on the `From` field cycles identities
- `Enter` on header text fields advances to the next compose field
- `Enter` on action buttons activates the focused action
- `Esc` on the action bar returns focus to the body
- `Ctrl+c` / `Ctrl+q` - Discard compose
- In **Body** focus: type directly in the multiline editor
- `Ctrl+f` in the body opens in-body search; `Enter`/`F3` repeats forward and `Shift+F3` repeats backward
- Discard confirmation modal: `y` confirms, `n`/`Esc` cancels

### Mouse
- Scroll wheel - Navigate/scroll
- Left click - Select folder/envelope
- Right click - Go back (in message view)

## Input Architecture (Compose)

Compose input is resolved in two layers:

1. **Context builder (`App::compose_key_context`)** maps runtime compose state into a compact context:
   - Focus bucket: `From` / `Header` / `Body` / `ActionBar`
   - Popup flags: autocomplete visible, discard-confirm visible
2. **Contextual resolver (`resolve_compose_with_context`)** applies deterministic priority rules:
   - Discard-confirm modal interception (`y`, `n`, `Esc`)
   - Global compose shortcuts (`Ctrl+c`, `Ctrl+q`)
   - Autocomplete navigation/accept interception
   - Compose shell controls (`Tab`, `Shift+Tab`, non-body `Up`/`Down`, action-bar activation)
   - Passthrough to the focused compose field

### Why this design
- Keeps compose ownership explicit while the body editor stays focused on text editing.
- Keeps compose behavior explicit and testable with a single resolver function.
- Makes modal interactions predictable by using a clear precedence order.
- Tracks `dirty` from actual text mutations instead of inferring it from raw body keys.

### Best-practice target outcome
For this use case, the ideal architecture is:
- A **single authoritative input router per view** (Compose already follows this pattern).
- State modeled as explicit focus buckets + overlays.
- Pure key-resolution functions with unit tests for each mode interaction.
- Minimal side effects in key resolver; side effects happen in `App` action handlers.

### One-pass, low-regression delivery strategy
To improve safely in one pass:
1. Keep behavior changes isolated to the compose resolver (`resolve_compose_with_context`).
2. Encode precedence explicitly (modal > shortcut > popup > compose shell > focused-field passthrough).
3. Add regression tests for each precedence boundary.
4. Avoid moving side-effectful logic into resolver code.

## Account Setup

The app supports multiple account types:

### Test Account (Local Maildir)
Already configured with sample emails:
```bash
cargo run -- --account test
```

### Real Accounts
Run the setup wizard:
```bash
cargo run -- --setup
```

Supported setup flows inside the wizard:
- **Generic IMAP/SMTP**: Creates or updates an app-owned account definition plus keyring secret IDs
- **iCloud**: App-specific password flow, with optional `~/.authinfo.gpg` compatibility rewrite
- **Gmail/Outlook**: Temporary Himalaya-backed OAuth bootstrap while native OAuth transport is still pending
- **Auth source of truth**: SQLite + OS keyring for password/app-password accounts; temporary Himalaya adapter for legacy/OAuth accounts

## Architecture

- **TEA pattern** - The Elm Architecture (Model, Update, View)
- **Async worker pool** - Background threads for all I/O
- **Channel-based IPC** - mpsc for result passing
- **Mail service boundary** - `MailService` trait isolates UI/worker code from transport details
- **Shared MIME parser** - Raw messages are parsed once and rendered as structured content
- **Theme support** - Reads SolverForge colors.toml
- **Zero dependencies** on async runtime (no tokio)

## Troubleshooting

### No accounts working
Check the local maildir test account first. If this fails, the backend/runtime is broken rather than remote auth:
```bash
cargo run -- --account test
```

SolverForge Mail expects:
- no external dependencies for the local `test` maildir account
- the system keyring/`secret-tool` for password and app-password setup flows
- only the Himalaya backend binary/config for legacy accounts and the temporary OAuth bootstrap path

### Authentication errors
- **iCloud**: Need an app-specific password, not the Apple ID password. If your config uses `auth.cmd`, verify `~/.authinfo.gpg` decrypts in this session.
- **Gmail/Outlook**: OAuth bootstrap is still temporary. Re-run `himalaya account configure <account>` if the legacy OAuth token expires.
- **Password-based IMAP/SMTP**: Verify the stored keyring secret IDs exist and your desktop secret service is unlocked.
- **Local `test` account failing**: This is not an auth issue. Fix backend discovery, config loading, or local maildir paths first.

### Keyring issues
```bash
# Check if keyring is accessible
secret-tool store --label="test" service test user test
# (enter any password)
secret-tool lookup service test user test
```

### GPG-backed auth
```bash
# Check that the iCloud auth file decrypts in this session
gpg -q --for-your-eyes-only -d ~/.authinfo.gpg | head
```

## Development

```bash
# Build
cargo build --release

# Test
cargo test

# Local CI-style validation
make ci

# Interactive setup wizard
cargo run -- --setup

# Run with specific account
cargo run -- --account test
```

## CI Status

GitHub Actions now runs the same core validation as local development:

```bash
make ci
```

## Files

```
solverforge-mail/
├── setup-accounts.sh         # Interactive account setup wizard
├── src/
│   ├── main.rs              # Entry point, terminal setup, CLI modes
│   ├── app.rs               # TEA state machine
│   ├── setup.rs             # Interactive account setup wizard
│   ├── worker.rs            # Background thread pool
│   ├── event.rs             # Terminal event handling
│   ├── keys.rs              # Keybinding definitions
│   ├── theme.rs             # Color theme loader
│   ├── himalaya/
│   │   ├── client.rs        # Temporary Himalaya CLI migration adapter
│   │   ├── config.rs        # Backend discovery and config hints
│   │   ├── diagnostics.rs   # Shared error classification
│   │   └── types.rs         # JSON types
│   ├── mail/
│   │   ├── service.rs       # App-facing mail service boundary
│   │   ├── message.rs       # Structured message content + display modes
│   │   ├── mime.rs          # Shared raw-message MIME parser
│   │   ├── maildir.rs       # Native local maildir backend
│   │   └── account_store.rs # App-owned account metadata store
│   └── ui/
│       ├── envelope_list.rs # Email list with relative dates
│       ├── folder_list.rs   # Sidebar with unread counts
│       ├── message_view.rs  # Structured message reader
│       ├── account_list.rs  # Account switcher
│       └── ...              # Other UI components
└── tests/                   # 54 comprehensive tests
```
