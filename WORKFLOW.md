# Account-scoped mail triage

Franking's unified inbox is a way to *see* mail from several accounts. It must
not become a shared permission list: the same address may be welcome at a
personal address and unwanted at a work address. Decisions are keyed by the
receiving account and the sender's normalized email address, never by contact
name, domain, or the currently selected account in the UI.

## Attention model

Separate three questions that traditional inboxes collapse:

1. **May I hear from this sender?** Unknown senders wait in **Screening** until
   explicitly accepted or blocked. Blocking is silent and reversible. Spam
   filtering remains the server's job; screening is a personal attention choice.
2. **Where should accepted mail appear?** Route the sender to **Inbox** for
   correspondence, **Reading** for newsletters, or **Receipts** for transactions.
   The user chooses; message content is not classified automatically. A single
   message may be placed elsewhere without changing its sender's future route.
3. **What do I need to do with it?** **Reply later** is a response queue;
   **Saved** is a reference shelf. These are independent of the sender's route
   and of the server's read/unread flag. Completing a reply removes it from the
   response queue, while saving a reference remains an explicit choice.

Inbox should distinguish new from seen correspondence. Reading and Receipts
should be browsable without creating an obligation to clear unread badges.
The default notification rule follows accepted Inbox senders, not every server
INBOX event. Users can choose all mail, contacts only, or off instead.

## Multi-account contract

- **All accounts** combines the same lane across accounts and labels each row
  with its source account. Per-account views remain available for focus.
- Reading, replying, and routing use the row's source account and folder.
  This is not yet true for every mutation: archive and generic move/copy still
  use the selected account or its folder list. Do not treat mixed-account batch
  actions as source-aware until those paths are repaired.
  The UI refuses mixed-account mailbox mutations and batch selection in All
  Inboxes rather than sending them to the wrong account.
- The local test account follows the same policy; the app does not depend on
  any particular provider's folders, rules, or extensions.
- Server mailboxes remain accessible. Triage is a local presentation and
  attention policy, not an IMAP MOVE or a claim that mail was rejected before
  delivery. Search and ordinary folder views can find screened-out mail.
- An address without a trustworthy mailbox in the envelope is left visible for
  manual handling rather than silently grouped by its display name.
- Pagination, offline cache, and account failure must not make a lane falsely
  appear complete. Cross-account loading fails visibly if any account fails,
  and paging grows the per-account window up to a bounded maximum.

The workflow uses additive, reversible local state. Existing accounts,
mailboxes, and cached messages must survive upgrades. Sender decisions can be
revised without changing the server's message history.

## Using mail triage

In an account's INBOX or All Inboxes, `v` cycles through Screening, Inbox,
Reading, Receipts, Blocked, then the unfiltered server inbox. `1` through `5`
route the selected sender to Inbox, Reading, Receipts, Blocked, or Screening,
respectively, from the list or message reader. Routing changes the local view
for all currently fetched messages from that sender in the receiving account;
it does not move mail on the server. `n`/`p` page through a growing per-account
window, so a lane keeps loading older mail as you go. Other server folders and
search are available independently.

Use `y` to add or remove a message from **Reply later**, and `Y` for **Saved**.
`L` opens the reply queue, `D` opens saved mail; press the same key again to
return to the inbox. In All Inboxes these queues include every account and
retain each message's source account and folder. A successful direct reply
removes the message from Reply later; marking it Saved is independent.
References use account, source folder, and UID, guarded by Message-ID where
available. A server-side move that changes the UID does not automatically
retarget a saved reference, so source reconciliation is still required.

## Capability map

This map distinguishes an available Franking behavior from a partial analogue.
An ordinary IMAP folder, flag, or contact tag does not by itself implement a
focused queue, a thread board, or a shared project view.

| User job | Franking today | Ownership or missing behavior |
| --- | --- | --- |
| Decide whether a new sender gets attention; reconsider a refusal | Local Screening and Blocked lanes | Per receiving account and sender mailbox; mail is still delivered to the provider |
| Route accepted correspondence, reading, and transactions | Local Inbox, Reading, Receipts lanes | Sender policy per receiving account; a lane loads a growing per-account window rather than the whole mailbox at once |
| Override one message without changing future sender delivery | `x` places one message in a lane, overriding its sender's route | The override is keyed by account, folder, and UID with a Message-ID guard |
| Separate new correspondence from previously seen threads | Inbox lane groups new before seen by message flag | Thread-level promotion after a fresh reply still needs coherent conversation state |
| Process multiple new messages in one uninterrupted pass | `T` reads selected messages together; `F` steps a reply queue one message at a time | Decisions between reads are limited to reply, done, and navigation |
| Read newsletters as an already-open scrollable stream | `T` in the Reading lane, with nothing selected, streams every loaded newsletter in one scroll | It reads what the lane has loaded, not the whole mailbox |
| Defer a required reply, then work only that queue | `L` lists the queue; `F` opens focus & reply, which shows one message at a time with reply, done, and navigation | Completing a reply advances the queue; source reconciliation after a server-side move remains |
| Keep a reference handy without owing a reply | Local Saved queue, independent of read and reply state | Reconcile moved messages whose server UID changes |
| Resurface mail on a chosen date | `b` sets a resurface delay per conversation; due conversations float to the top of the list | The reminder lives only while mail is listed; a scheduled background sweep is not required for it to apply |
| Suppress future updates to a conversation while retaining its history | `M` quiets a conversation; direct replies no longer notify or outrank read mail | Matching uses Message-ID and In-Reply-To anchors, so a deep reply whose immediate parent is not loaded may not match |
| Collapse a high-volume sender into one row | `B` cycles a sender through One row, expanded, and not bundled; the collapsed row shows the count | Bundling is keyed by account and sender mailbox |
| Track a conversation through custom stages | `Ctrl+g` opens an account-scoped stage board: assign the conversation, create, rename, delete stages, and filter the list by a stage | A conversation is in one stage at a time |
| Keep related threads together without merging them | Search and contact tags, not project collections | Named collection of distinct thread references; separate from folders and saved searches |
| Locally join conversations without changing recipients' threads | RFC threading and collapse, not manual merge | Explicit local mapping; preserve originals and outbound reply headers |
| Read selected messages together in one scroll | `T` reads the selected messages, or the cursor row, as one numbered scroll | Loads each message on demand; there is no combined print or export yet |
| Search and navigate attachments independently of messages | `Ctrl+l` opens a cross-account attachment library over cached mail; Enter opens the source message | Only messages whose raw bytes are cached contribute; inline images are hidden |
| Save small excerpts for quick retrieval | `*` in the reader clips an excerpt keyed to its source account and message; `Ctrl+k` lists clips and Enter copies one to the clipboard | Clips are text only and are captured from the reader, not by arbitrary selection |
| Add private notes to a contact, message, or conversation | Contact notes exist; `i` writes a private note on a message, shown above its body | Notes are per message, not per conversation or with files |
| Rename a subject for local display only | `%` sets a display alias for the conversation; the wire subject is unchanged | The alias is keyed by conversation anchor |
| Insert reusable response text | `Ctrl+n` in compose opens a snippet picker; Enter inserts at the cursor, `s` saves the current body under a name | Snippets are global, not per account |
| Control interruption per contact or thread | Focused-Inbox default, all/contacts/off preferences, `M` to quiet a conversation and `+` to make it always notify | Overrides key on Message-ID anchors, so a deep reply whose parent is not loaded may not match |
| Avoid remote tracking pixels | HTML-to-terminal-text rendering makes no image request | No tracker detection/report; don't claim comprehensive remote-content blocking |
| Let a trusted unknown sender bypass screening | Preferences holds a revocable per-account bypass token; a whole-word subject match lifts Screening into the Inbox only | An explicit Blocked route still wins over the token |
| Identify spam separately from screening | Provider junk mailbox and Sieve controls | A blocked sender is an attention decision, not spam training or SMTP rejection |
| Combine accounts or focus on one | All Inboxes, account picker, scoped search | Unified rows need source-aware mutations and complete per-account paging |
| Send from multiple addresses | Per-account identities with From selection | Identity, signatures, and Sent mailbox stay tied to their transport account |
| Compose, reply, forward, attach, draft, and schedule send | Available | Outgoing controls are separate from incoming triage |
| Send one response to many unrelated messages | `R` on a selection opens one message addressed to every selected sender | Recipients are de-duplicated and reviewed in compose before sending |
| Send oversized files by hosted download link | Ordinary SMTP attachments | Requires external file hosting and lifecycle, not larger MIME payloads |
| Auto-reply when away | Not available in the TUI | Provider-side rule or an always-running service, per account |
| Publish mail or share live threads/projects by link | Save a message as `.eml`, not publishing | Requires hosting, access control, revocation, and redaction |
| Collaborate with comments and shared mailboxes | Local personal accounts and mail UI | Multi-user service, permissions, and shared conversation history |
| Calendar and private journal | Invitation handoff to Planner123 | Planner123 integration is not an in-client calendar or journal |
| Security of hosted accounts | Keyring auth, OAuth, PGP, S/MIME | Hosted login, MFA, and security-key promises are provider-specific |
| Decorative cover for previously seen mail | A preference covers previously seen Inbox mail; `V` lifts it for the session | Presentation only; delivery and follow-up are unchanged |

The largest gap is the lifecycle *after* screening: reply and reference queues,
quiet threads, resurfacing, and work across multiple messages. Those need
durable message/thread identifiers and a source-aware cross-account UI before
the local route lanes can become the primary mail workflow. Provider-hosted
sharing, large-file delivery, and multi-user collaboration are separate
service products, not features a local IMAP client can reproduce by renaming
folders.
