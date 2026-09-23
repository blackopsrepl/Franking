# Account-scoped mail triage

Franking's unified inbox is a way to *see* mail from several accounts. It must
not become a shared permission list: the same address may be welcome at a
personal address and unwanted at a work address. Decisions are keyed by the
receiving account and the sender's normalized email address, never by contact
name, domain, or the currently selected account in the UI.

## Intent

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
Notifications should follow the attention policy, not every server INBOX event.

## Multi-account contract

- **All accounts** combines the same lane across accounts and labels each row
  with its source account. Per-account views remain available for focus.
- Every action on a combined row uses that row's source account and source
  folder, including reading, replying, and updating its sender's decision.
- The local test account follows the same policy; the app does not depend on
  any particular provider's folders, rules, or extensions.
- Server mailboxes remain accessible. Triage is a local presentation and
  attention policy, not an IMAP MOVE or a claim that mail was rejected before
  delivery. Search and ordinary folder views can find screened-out mail.
- An address without a trustworthy mailbox in the envelope is left visible for
  manual handling rather than silently grouped by its display name.
- Pagination, offline cache, and account failure must not make a lane falsely
  appear complete. A cross-account lane needs explicit per-account loading
  errors and consistent paging before it can replace the existing inbox.

The experiment starts with additive, reversible local state. Existing accounts,
mailboxes, and cached messages must survive upgrades. Sender decisions can be
revised without changing the server's message history.
