# Everyday mail workflow

## One Inbox, ordinary folders

Inbox shows mail delivered to the receiving account's server Inbox. All Inboxes
combines accounts and labels their messages. Opening a folder shows that server
mailbox. Unknown senders do not need approval, and local metadata does not hide,
reclassify, bundle, merge, rename, or reorder delivered mail.

The sidebar also exposes two explicit views:

- **Reply later** is a local checklist. `y` adds/removes the selected message;
  a successful direct reply clears its marker. Read/unread stays independent.
- **Starred (cached)** lists server-flagged messages from cached folders. `!`
  toggles the server flag. Refresh source folders to bring their cached flags
  up to date. This is not a claim to have searched every server mailbox.

These views follow the selected account; opening them from All Inboxes includes
all accounts. Their message references retain source account, folder, and UID.
Local follow-up references are not automatically reconciled after server moves.
Batch actions across accounts or cross-folder local views are refused rather
than risking a mutation against the wrong mailbox.

## A small keyboard surface

Use arrows or `j/k`, Enter to open, and Tab to focus the sidebar. Everyday list
operations have direct keys: compose `c`, search `/`, archive `e`, delete `d`,
move `m`, star `!`, read/unread `N`, Reply later `y`, select Space, undo `z`.
Esc clears selection; it does not quit. In the reader, `r` replies, `R` replies
all, `f` forwards, `a` opens attachments, and Esc returns to the list.

Press **`:` for Actions**, type a command name, use arrows to choose, then Enter.
Esc closes Actions without executing anything or leaving the underlying view.
Threading, sorting, copying, mute, contacts, identities, keys, preferences, and
the outbox live here rather than competing for more letter bindings.
`?` or F1 opens contextual help; F1 also works in compose.

The footer shows only the most useful controls for its current context. It
never attempts to enumerate every action, and it reserves room for Actions.
Transient success messages expire; errors remain visible.

## Moving mail and explicit sender rules

Move normally affects only the selected message(s). Its **Also move future
messages from sender** checkbox starts unchecked every time. Tab toggles it
when moving one message with a parseable sender address; Copy never creates a
rule. The rule belongs to the message's receiving account, not its display name
or the account that happens to be focused elsewhere.

Future-mail rules require an IMAP Inbox sync with UIDVALIDITY and UIDNEXT. They
use that boundary to avoid sweeping older unfetched mail into a new rule. A
mailbox UID reset suspends the rule; recreate it from Move to enable it again.
Local Maildir accounts do not provide this IMAP boundary and cannot create these
rules. Ordinary manual Move remains available.

**Actions → Sender folder rules** shows account, sender, and destination. Press
d twice to remove a rule. Removal stops future routing; it does not move mail
back. Old screening decisions are never imported into these rules.

Rules run when Franking fetches Inbox mail, including unified and threaded
listings. They perform real server moves, not local visibility filters. They
do not run while Franking is closed and are not provider-side spam training.
A failed move is reported instead of pretending the message was filed.

## Notifications

Notifications default to **all Inbox mail**. Preferences offers all, contacts
only, or off. **Actions → Mute / unmute conversation** suppresses notifications
for that conversation without hiding it or changing its ordering. Matching uses
available Message-ID/reply anchors, so a deep reply with no loaded parent may
not be recognised. Off stays off; legacy loud rules cannot override it.

## Existing installations

There is no destructive data migration. Existing mail, accounts, folders,
contacts, snippets, saved searches, and follow-up records remain intact.

Screening lanes, per-message placement, bypass tokens, seen-mail covers,
resurfacing, loud priority, bundles, stages, collections, local conversation
merges, clips, subject aliases, read-together, focus-and-reply, and the separate
Saved shelf are retired from the interface. Their existing local records remain
stored for compatibility, but do not silently affect ordinary mail listings.
Legacy Saved records are not automatically turned into server stars.
Previously stored focused-Inbox notification settings now mean all Inbox mail;
explicit off and contacts-only preferences are retained.

Compose, reply, forwarding, drafts, autosave, snippets, attachments, scheduled
sending, encryption, calendar handoff, search, and server folders remain.
