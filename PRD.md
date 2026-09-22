# PRD: Replace Himalaya CLI With an App-Owned Mail Engine

Status: Draft
Owner: SolverForge Mail
Last Updated: 2026-04-13

## 1. Executive Summary

SolverForge Mail currently delegates mail access, account configuration, and much of auth behavior to the external `himalaya` CLI. That boundary has become a systemic failure point. The application does not own backend discovery, config semantics, auth state, token refresh, or protocol diagnostics. As a result, the app is fragile, difficult to reason about, and hard to support.

This project will replace the Himalaya CLI boundary with an app-owned mail engine implemented in Rust. The application will directly own:

- account models
- auth state and secret references
- OAuth flows and token refresh
- IMAP and SMTP sessions
- MIME parsing and structured message content
- local maildir access for the test backend
- typed diagnostics
- setup and account management workflows

The application will not reimplement mail protocols from raw sockets unless necessary. It will use mature Rust protocol and MIME libraries where practical, while keeping all control-plane decisions inside the app.

## 2. Background and Problem Statement

### 2.1 Current State

Today, SolverForge Mail:

- shells out to `himalaya` for account listing, folder listing, message fetches, flags, moves, deletes, templates, and sending
- depends on external binary discovery
- depends on external config resolution semantics
- depends on external secret backends configured outside the app
- receives opaque errors after subprocess execution

The app also has a local SQLite database for contacts and identities. An earlier attempt introduced DB-backed credentials, but that path was never wired into the live runtime as a real auth authority.

### 2.2 Core Problem

The current architecture creates a split-brain system:

- SolverForge Mail owns the UI and some local state
- Himalaya owns remote account definitions, remote auth behavior, and many operational errors
- the desktop environment or GPG session owns secret availability

This means the app cannot reliably answer basic operational questions:

- Why is this account failing?
- Which config is active?
- Which secret path is actually being used?
- Can the app repair an auth failure itself?
- Can the app guarantee a stable account model across environments?

### 2.3 Why This Must Change

The current model causes:

- auth regressions caused by config/env/path differences
- duplicated setup logic across runtime, shell scripts, and docs
- weak diagnostics because the app sees only subprocess output
- inability to build provider-specific recovery logic cleanly
- inability to implement durable sync and cache semantics with confidence

The app needs a single, coherent mail runtime it controls end to end.

## 3. Product Goal

Build a first-party mail engine for SolverForge Mail that provides a stable, production-grade foundation for account setup, auth, transport, MIME-aware message reading, diagnostics, and sending without depending on the Himalaya CLI or Himalaya config at runtime.

## 4. Product Principles

1. The app owns the truth.
   Account definitions, auth modes, and recovery behavior must be app-defined.

2. Secrets are not generic app data.
   The DB stores metadata and references. Raw secrets belong in OS keyrings unless there is a strong reason to do otherwise.

3. Local and remote backends share one interface.
   Maildir and IMAP/SMTP must implement the same app-facing service boundary.

4. Parse once, choose once, render once.
   MIME parsing and canonical body selection belong in the mail layer, not in the TUI. The UI consumes one terminal-native message document, not competing body modes.

5. Error handling must be typed and actionable.
   No generic "auth failed" buckets when the failure is actually config, secret service, token refresh, or transport.

6. Narrow scope beats fake generality.
   The first release should be excellent for a small set of providers and flows instead of mediocre for all mail setups.

7. No second half-finished auth pivot.
   There will be one account/auth architecture, not two competing ones.

8. No escape-hatch rendering paths.
   If HTML mail is supported, it must be supported by the primary in-app renderer rather than by handing users off to an external viewer.

## 5. Goals

### 5.1 Primary Goals

- Remove all runtime dependency on the Himalaya CLI.
- Remove all runtime dependency on Himalaya config files and env semantics.
- Replace the current string-based message-read path with a structured MIME-first message model.
- Support app-owned setup and operation for:
  - local maildir test account
  - generic IMAP/SMTP with password or app password
  - Gmail OAuth2
  - Outlook OAuth2
  - iCloud app-specific password
- Support `multipart/alternative`, `text/plain`, and `text/html` reads through one deterministic in-TUI rendering path.
- Remove body-mode switching and any external HTML viewer dependency from the message-reading experience.
- Provide typed, user-facing diagnostics for config, keyring, OAuth, TLS, IMAP auth, SMTP auth, and local-backend failures.
- Preserve the current TUI application model wherever practical.

### 5.2 Secondary Goals

- Establish a clean service abstraction for future offline sync and cache work.
- Establish a migration path from existing Himalaya-managed installs.
- Make setup and runtime behavior consistent because both use the same app-owned engine.

## 6. Non-Goals

The following are explicitly out of scope for the first implementation:

- supporting every provider-specific extension or special-case behavior
- full offline-first sync
- server-side search parity across all providers
- embedding a browser engine in the TUI
- full HTML/CSS fidelity inside the terminal
- calendar, contacts sync, or CardDAV/CalDAV
- mobile sync or daemon mode
- rewriting mail protocols from raw sockets when mature Rust libraries exist

## 7. Success Criteria

The project is successful when all of the following are true:

- the app performs no mail operations via subprocess calls to `himalaya`
- the app can bootstrap and operate accounts without `~/.config/himalaya/config.toml`
- setup, runtime, and recovery all use the same account/auth model
- `test` maildir account always works independently of remote auth state
- Gmail, Outlook, iCloud, and generic IMAP/SMTP pass the acceptance matrix
- the app reports failure causes in distinct categories, not a generic auth bucket
- `cargo test` includes integration coverage for mail transport and auth flows

## 8. User and Operational Scenarios

### 8.1 First-Time User

- installs SolverForge Mail
- runs `solverforge-mail --setup`
- adds a Gmail account via OAuth
- sees account status immediately
- launches the app with that account

### 8.2 Existing User With Broken Keyring

- app shows account failure
- diagnostics clearly state secret-service problem instead of auth rejection
- user repairs keyring or re-enters secret in setup

### 8.3 Existing User With Expired OAuth Token

- app detects refresh failure
- user-facing message says reconfigure OAuth
- setup flow repairs token state without editing external config files

### 8.4 Local Test Path

- developer runs app with the local `test` account
- maildir backend works even if network, OAuth, or keyring state is broken

## 9. Scope of the First Deliverable

The first production deliverable will support:

- app-owned accounts stored in SQLite
- secret references stored in SQLite
- secrets stored in OS keyring
- optional iCloud `~/.authinfo.gpg` compatibility path only if deliberately retained
- native maildir implementation for the local test account
- native IMAP read operations
- native SMTP send operations
- shared MIME-first parsed message model for full message reads
- one canonical terminal-native body document for message rendering
- one in-TUI message-reading path for both plain-text and HTML mail
- internal draft/template generation for compose, reply, and forward
- typed error classification
- one interactive setup wizard

## 10. Proposed Architecture

### 10.1 High-Level Architecture

Introduce a new `src/mail/` subsystem that becomes the only app-facing mail layer.

Proposed modules:

- `src/mail/mod.rs`
- `src/mail/service.rs`
- `src/mail/types.rs`
- `src/mail/errors.rs`
- `src/mail/account_store.rs`
- `src/mail/auth/`
- `src/mail/imap/`
- `src/mail/smtp/`
- `src/mail/maildir/`
- `src/mail/cache/`

### 10.2 App Boundary

Define a `MailService` trait that the UI and worker layer depend on.

Required operations:

- list accounts
- probe account health
- list folders
- list envelopes
- read message
- preview message
- move/copy/delete message
- add/remove flags
- download attachments
- create compose draft
- create reply draft
- create forward draft
- send compiled draft

The app must not depend on backend-specific behavior above this layer.

### 10.3 Backend Implementations

Implement:

- `MaildirService`
- `ImapSmtpService`

Optional future implementations:

- JMAP
- Exchange-specific adapters

### 10.4 Worker Integration

The current worker model can stay conceptually intact. The worker should call the new `MailService` instead of the Himalaya adapter.

This keeps the TUI state machine stable while replacing the transport layer underneath.

### 10.5 Diagnostics

Move all mail/runtime failures into typed domain errors.

Required categories:

- backend unavailable
- account not found
- config invalid
- keyring unavailable
- secret missing
- OAuth interactive reconfiguration required
- OAuth refresh failure
- IMAP auth rejected
- SMTP auth rejected
- TLS failure
- transport timeout
- connection dropped
- local maildir failure
- unsupported feature

User-facing messages must be derived from typed errors, not raw command stderr.

### 10.6 Message Model and MIME Ownership

`read message` and `preview message` must return structured message content, not a pre-rendered body string.

Recommended message model:

- parsed headers
- extracted plain-text source part
- extracted HTML source part
- attachment and inline-part metadata
- canonical `BodyDocument` selected once per message

Recommended modules:

- `src/mail/message.rs`
- `src/mail/mime.rs`

Required parser responsibilities:

- parse raw message bytes once
- decode transfer encodings and common charsets
- extract `text/plain` and `text/html`
- identify attachments and inline parts
- choose a deterministic canonical body source
- transform the selected source into a terminal-native `BodyDocument`
- expose one render path to the UI

Recommended initial library stack:

- `mail-parser` for RFC 5322 and MIME parsing plus charset handling
- an HTML parser that preserves document structure and feeds the terminal-native `BodyDocument`

Backend rules:

- Maildir reads raw bytes from disk and feeds the shared MIME parser.
- Any temporary Himalaya compatibility path must export raw message bytes and feed the shared MIME parser rather than parse human-friendly CLI text.
- Native IMAP reads must preserve the same raw-message-to-parser boundary.

UI rules:

- the app stores structured message content plus a canonical `BodyDocument` instead of `String` body text
- the message view renders headers from parsed fields, not by scanning rendered body text
- the message view renders exactly one canonical body representation per message
- there are no user-facing body mode toggles and no external HTML viewer path

Canonical body selection rules:

- if an HTML body exists, HTML is the canonical source
- else if a plain-text body exists, plain text is the canonical source
- else the UI shows an explicit no-renderable-body state
- once selected, the canonical body is not switchable in the UI

## 11. Data Model

### 11.1 Principles

- secrets are stored in the OS keyring by default
- the DB stores metadata, references, and state
- token encryption inside SQLite is deferred unless there is a compelling operational reason

### 11.2 Proposed Tables

`accounts`

- `id`
- `name`
- `backend_kind` (`maildir`, `imap`)
- `provider_kind` (`generic`, `gmail`, `outlook`, `icloud`, `custom`)
- `enabled`
- `is_default`
- `created_at`
- `updated_at`

`account_endpoints`

- `account_id`
- `imap_host`
- `imap_port`
- `imap_security`
- `smtp_host`
- `smtp_port`
- `smtp_security`

`auth_bindings`

- `account_id`
- `auth_mode` (`password`, `app_password`, `oauth2`, `maildir`)
- `username`
- `keyring_imap_secret_id`
- `keyring_smtp_secret_id`
- `oauth_state_id`

`oauth_states`

- `id`
- `account_id`
- `provider_kind`
- `client_id`
- `client_secret_ref`
- `refresh_token_ref`
- `access_token_cached`
- `access_token_expires_at`
- `scopes`
- `token_endpoint`
- `auth_endpoint`

`folder_cache`

- `account_id`
- `remote_id`
- `name`
- `attributes`
- `unread_count`
- `sync_token`
- `updated_at`

`envelope_cache`

- `account_id`
- `folder_remote_id`
- `remote_uid`
- `message_id`
- `subject`
- `sender_display`
- `received_at`
- `flags`
- `thread_hint`
- `updated_at`

Optional later:

- `message_cache`
- `sync_checkpoints`
- `attachment_cache`

### 11.3 Existing Tables

Retain:

- contacts
- contact tags
- identities

Remove:

- legacy credentials table

## 12. Auth Design

### 12.1 Password and App Password Accounts

For generic IMAP/SMTP and iCloud:

- store secret values in OS keyring
- store secret references in DB
- store account endpoints and usernames in DB
- never require external config files

### 12.2 OAuth Accounts

For Gmail and Outlook:

- the app initiates the browser flow
- the app receives and exchanges the authorization code
- the app persists refresh-token references and token metadata
- the app refreshes tokens internally
- the app surfaces reauth requirements explicitly

### 12.3 Key Security Requirements

- do not log secrets or tokens
- do not print secret material in errors
- keep OAuth client configuration scoped by provider and account
- if keyring is unavailable, fail with a typed secret-store error

## 13. Transport and Protocol Strategy

### 13.1 Recommended Strategy

Do not write IMAP, SMTP, MIME, TLS, or OAuth from raw sockets unless forced.

Use mature Rust libraries for:

- IMAP client support
- SMTP sending
- MIME parsing and composition
- OAuth2 flows
- TLS

Candidate library families should be evaluated during implementation, but the decision principle is:

- app-owned control plane
- library-owned protocol details

### 13.2 Required Mail Operations

Read path:

- connect and authenticate
- list folders
- select folder
- list envelopes with pagination
- fetch message body
- fetch attachments
- read flags

Mutation path:

- flag add/remove
- move
- copy
- delete

Send path:

- compose and send new message
- reply
- reply-all
- forward
- attachments

## 14. Compose and Draft Strategy

Today, compose templates come from Himalaya. That must be replaced.

The app will generate drafts internally using:

- selected identity
- reply target metadata
- forward target metadata
- local compose rules

The resulting draft model should be backend-independent and suitable for SMTP send.

This eliminates one of the most brittle Himalaya-specific dependencies.

## 15. Cache and Sync Strategy

### 15.1 Initial Strategy

Start with on-demand fetches and lightweight caches for:

- folder metadata
- envelope summaries

Do not block first delivery on a complex offline sync engine.

### 15.2 Future Strategy

After transport is stable:

- add sync checkpoints
- add UID-based incremental refresh
- add cache invalidation rules
- add optional message-body caching

## 16. Setup and Account Management

### 16.1 Required Product Surface

The app must own:

- create account
- edit account
- list account status
- re-enter secrets
- re-run OAuth
- probe connection health
- delete account
- choose default account

### 16.2 CLI Surface

Minimum commands:

- `solverforge-mail --setup`
- `solverforge-mail --accounts`

Recommended follow-up commands:

- `solverforge-mail --account-add`
- `solverforge-mail --account-edit <name>`
- `solverforge-mail --account-delete <name>`
- `solverforge-mail --account-probe <name>`

## 17. Migration Strategy

### 17.1 Migration Principles

- do not require users to keep Himalaya after migration
- do not depend on partial legacy state indefinitely
- keep migration explicit and auditable

### 17.2 Migration Plan

Phase 1:

- add app-owned account store
- optionally import account definitions from Himalaya config one time
- do not continue to read Himalaya config at runtime

Phase 2:

- migrate secrets by asking the user to confirm or re-enter them
- import only what can be done reliably

Phase 3:

- remove all runtime Himalaya code
- remove any temporary compatibility import code once migration is stable

## 18. Phased Delivery Plan

### Phase 0: Foundation and Boundary

Deliverables:

- `mail` module skeleton
- `MailService` trait
- typed `MailError`
- typed `MessageContent` and canonical message document model
- app and worker routed through the trait

Exit Criteria:

- TUI compiles and runs with the new abstraction
- message-read paths no longer require a flattened `String` body contract
- Himalaya can remain behind a temporary adapter only during this phase

### Phase 1: Local Backend

Deliverables:

- native `MaildirService`
- shared MIME parser wired to maildir reads
- canonical terminal-native body renderer for plain-text and HTML messages
- `test` account owned by the new interface

Exit Criteria:

- local test account works without Himalaya
- HTML-only and `multipart/alternative` fixtures are readable in the TUI
- integration tests cover maildir read path and mutation path

### Phase 2: Account Store and Setup

Deliverables:

- DB schema for accounts and auth bindings
- setup wizard uses app-owned accounts
- no new accounts require Himalaya config

Exit Criteria:

- first-time setup works entirely without Himalaya
- account status/probe is fully internal

### Phase 3: Native IMAP Read Path

Deliverables:

- native folder list
- native envelope list
- native message read
- native message read uses the shared MIME parser boundary
- flag, move, copy, delete operations

Exit Criteria:

- generic IMAP account works end to end for read and basic mutations

### Phase 4: Native SMTP Send Path

Deliverables:

- send new mail
- reply and forward
- attachment send
- internal draft generation

Exit Criteria:

- app can send without Himalaya templates or Himalaya send path

### Phase 5: OAuth Accounts

Deliverables:

- Gmail OAuth
- Outlook OAuth
- token refresh
- token-expiry handling

Exit Criteria:

- Gmail and Outlook work end to end without Himalaya

### Phase 6: Provider Hardening and Cleanup

Deliverables:

- iCloud app-password stabilization
- provider-specific quirk handling
- remove Himalaya adapter and all legacy migration shims

Exit Criteria:

- no subprocess mail operations remain
- no Himalaya runtime dependency remains

## 19. Acceptance Criteria

### 19.1 Functional Acceptance

- `test` account works locally without external dependencies
- generic IMAP/SMTP account can be added, probed, read, and send mail
- Gmail account can complete OAuth and refresh tokens
- Outlook account can complete OAuth and refresh tokens
- iCloud account can authenticate via app password
- HTML-only and `multipart/alternative` messages are readable in the TUI through the same canonical renderer
- users do not switch between competing body modes
- supported HTML mail does not require an external viewer path
- compose, reply, forward, move, delete, and flags work on supported providers

### 19.2 Reliability Acceptance

- reconnect behavior handles transient disconnects
- timeout behavior is bounded and user-visible
- auth failures are distinct from transport failures
- missing keyring is distinct from bad password

### 19.3 Diagnostics Acceptance

- all major failure classes map to typed user-visible diagnostics
- no critical setup flow depends on parsing random stderr text from an external CLI

## 20. Testing Strategy

### 20.1 Unit Tests

- account validation
- auth state transitions
- token refresh logic
- provider preset resolution
- canonical body selection policy
- MIME header and body extraction
- error classification

### 20.2 Integration Tests

- local maildir backend
- IMAP test server
- SMTP test server
- OAuth callback and refresh test harness
- MIME send and parse fixtures
- HTML-only, `multipart/alternative`, quoted-printable, base64, and malformed MIME fixtures

### 20.3 Manual Acceptance Matrix

Required manual test scenarios:

- no accounts configured
- first-time Gmail OAuth bootstrap
- Outlook OAuth reauth after token expiry
- generic password account with wrong password
- missing keyring session
- iCloud app-password flow
- local test account while all remote accounts are broken
- at least one HTML-heavy inbox or fixture account path

## 21. Observability and Supportability

The mail engine should emit structured logs for:

- account selected
- connection opened
- auth mode used
- refresh attempted
- folder sync started/completed
- send started/completed
- typed failure category

Logs must never include:

- passwords
- refresh tokens
- access tokens
- decrypted GPG content

## 22. Risks

### 22.1 Technical Risks

- IMAP provider quirks
- SMTP interoperability issues
- OAuth callback handling complexity
- MIME correctness for replies/forwards
- terminal rendering quality for HTML-derived terminal documents
- Linux secret-service availability

### 22.2 Product Risks

- migration friction for existing users
- scope creep into full offline sync too early
- trying to support too many provider-specific cases in v1

### 22.3 Mitigations

- narrow the provider matrix early
- keep one service boundary
- keep transport and auth typed
- ship in phases with acceptance gates

## 23. Open Questions

These must be resolved before final implementation:

- Should iCloud continue supporting `~/.authinfo.gpg`, or should all secrets move to keyring only?
- Should refresh tokens live only in keyring, or may encrypted references be cached in SQLite?
- Which Rust library stack best balances maturity, maintenance, and TLS behavior for IMAP/SMTP?
- Should message-body caching be part of first delivery or postponed?

## 24. Recommended Immediate Next Steps

1. Approve this PRD as the target architecture.
2. Create an engineering design document for the `MailService`, `MailError`, `MessageContent`, canonical `BodyDocument`, and renderer pipeline.
3. Implement Phase 0 and Phase 1 before touching OAuth.
4. Do not reintroduce any Himalaya-dependent runtime path during the transition.
5. Treat import from Himalaya config as a temporary migration utility, not an architectural dependency.

## 25. Final Decision Statement

SolverForge Mail should stop treating the Himalaya CLI as its production mail engine. The professional path is to own the mail control plane, transport, and MIME-aware message pipeline inside the application, keep the UI mostly intact, use Rust protocol libraries rather than raw protocol implementations, and ship the replacement in narrow, test-gated phases.
