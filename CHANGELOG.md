# Changelog

All notable changes to this project will be documented in this file. See [commit-and-tag-version](https://github.com/absolute-version/commit-and-tag-version) for commit guidelines.

## [2.1.1](https://github.com/blackopsrepl/Franking/compare/v2.1.0...v2.1.1) (2026-09-23)


### Performance Improvements

* **mail:** cut per-row queries and scan caches off the UI thread 076806d
* **mail:** resolve conversation roots with one index 3012b85
* **ui:** stop rebuilding the hint list every frame df4bc32

## [2.1.0](https://github.com/blackopsrepl/Franking/compare/v2.0.1...v2.1.0) (2026-09-23)


### Features

* **mail:** add private notes and display-only subject aliases c48062d
* **mail:** index attachments across cached accounts 8a41e7c
* **mail:** keep account-scoped reply and reference queues 6757fde
* **mail:** make accepted correspondence the quiet notification default a4feaef
* **mail:** page merged and lane views through a growing window 696e271
* **mail:** persist sender decisions per receiving account 5cbe0e2
* **mail:** place a single message in a lane 154d950
* **mail:** quiet conversations and resurface them later 189f5e3
* **mail:** read the selected messages together 4157fe3
* **ui:** route incoming senders into account-scoped mail lanes fbfea7b


### Bug Fixes

* **ci:** fail when a piped cargo check fails 8f11aff
* **db:** migrate installed schemas without resetting user data cb00249
* **mail:** clear the loading note when messages open together 4d4804b
* **mail:** derive conversation anchors when the list is stale 52d629e
* **mail:** refuse unsafe mixed-account mailbox mutations fd33cd8
* **ui:** keep shortcuts discoverable at every terminal width 468bcf7

## [2.0.1](https://github.com/blackopsrepl/Franking/compare/v2.0.0...v2.0.1) (2026-09-22)


### Bug Fixes

* **calendar:** keep the reader's zone out of the process environment a9f1b32

## [2.0.0](https://github.com/blackopsrepl/Franking/compare/v1.0.1...v2.0.0) (2026-09-22)


### ⚠ BREAKING CHANGES

* rename the product to Franking

* rename the product to Franking be02895


### Features

* **accounts:** add and edit IMAP accounts in the app 6088f64
* **accounts:** add app-owned mail account schema 314e1a8
* **accounts:** authorize OAuth2 accounts from the form dd6d035
* **accounts:** auto-detect provider settings in the account form b6b0978
* **accounts:** choose how a connection is protected 6bc945a
* **accounts:** configure ManageSieve explicitly 16d14fa
* **accounts:** persist app-owned OAuth state 61a3977
* **accounts:** set the default account and delete accounts 03c9e1b
* add app-owned mail service and local maildir backend c8b994a
* **app:** answer a search from the cache first b4e6073
* **app:** decide which mail raises a notification df72b6f
* **app:** download every attachment as one archive 7eb070b
* **app:** manage PGP key material in a keys overlay 62fb895
* **app:** rename a Sieve script 31fdacb
* **app:** save and re-run named searches 785ec87
* **attachments:** preview text attachments in app 7a92b7a
* **attachments:** save an attachment to a chosen directory 174f2fa
* **calendar:** answer invitations with an iTIP reply 663ee8a
* **compose:** attach files to outgoing mail f142020
* **compose:** autosave the message in progress 34af60b
* **compose:** list and remove queued attachments 9c1c2b6
* **compose:** pick attachments from the filesystem b2fb4cf
* **compose:** save drafts to the account's Drafts mailbox c76793c
* **contacts:** filter the address book by tag 04afd4b
* **crypto:** unlock PGP secret keys from the TUI c9d1fca
* **folders:** create, rename, and delete mailboxes 6d9bb73
* **identities:** per-identity message signatures 325c958
* **imap:** add the app-owned client and prove it against Dovecot 97978da
* **imap:** delta-sync folders with CONDSTORE and QRESYNC 71314c2
* **imap:** implement listing, fetch, sort, and thread on the new layer 6206b8d
* **imap:** own the response reading policy on imap-codec f4aaa36
* **mail:** add a lossless decoded message model 3a24a5f
* **mail:** add an FTS5-backed local message store e050fb5
* **mail:** add app-owned OAuth2 authorization and refresh 786c910
* **mail:** add native IMAP/SMTP mail service 6161afc
* **mail:** add PGP key management 7c4a5e6
* **mail:** add structured MIME message content 9d39695
* **mail:** cache a whole folder for offline use with S 03f9af3
* **mail:** cache fetched messages and serve listings offline 3b7b58d
* **mail:** check S/MIME certificates against revocation lists d37b9d6
* **mail:** decrypt PGP/MIME encrypted messages ba70399
* **mail:** detect PGP/MIME and S/MIME protection 0f60c3d
* **mail:** encrypt stored drafts to the sender 67e2fc9
* **mail:** extract attachments on the local maildir backend ff324e7
* **mail:** hand invitations to Planner123 instead of answering them 71b55e0
* **mail:** let an identity name its own Sent mailbox 65d24b4
* **mail:** mark a whole folder read with A b34bf9f
* **mail:** order a folder with server-side SORT 6ea8e4f
* **mail:** prefer HTML as the canonical message body ee2db16
* **mail:** preserve threading headers across reply and send 9c24633
* **mail:** report and toggle folder subscriptions 7f8b62d
* **mail:** resolve invitation timezones and surface cancellations 9074572
* **mail:** resolve the Trash mailbox by its RFC 6154 role 31d8cdc
* **mail:** resume drafts from the Drafts folder d5e95b2
* **mail:** reuse IMAP sessions and watch mailboxes with IDLE ae5fd5b
* **mail:** route accounts to the native IMAP/SMTP service 2d9a198
* **mail:** run search on the IMAP server fab80c1
* **mail:** save sent messages to the server Sent mailbox e598ab5
* **mail:** search every account, not just the current one 290f4ea
* **mail:** show calendar invitations 0537614
* **mail:** sign and encrypt outbound mail as PGP/MIME 2a88ac0
* **mail:** sign and encrypt outgoing mail with S/MIME 7b51faf
* **mail:** surface RFC 6154 folder roles in the model and UI fa3593e
* **mail:** surface SPF, DKIM, and DMARC verdicts 4affb18
* **mail:** use server-side THREAD for the threaded listing 3315d24
* **mail:** use STATUS for unread counts and record sync cursors 4e2aae4
* **mail:** verify and decrypt inline OpenPGP db357de
* **mail:** verify and decrypt S/MIME (PKCS[#7](https://github.com/blackopsrepl/Franking/issues/7)) 559f324
* **mail:** verify PGP/MIME detached signatures d9006da
* **outbox:** queue messages that fail to send 15f2845
* **outbox:** schedule messages to send later d2bda74
* **pgp:** sign and encrypt outbound content 44c1076
* **reading:** mark opened messages read on the server 3ecba39
* **search:** add 'to' and 'body' search prefixes 49f113b
* **search:** search every folder from the search prompt 6ef5e99
* **settings:** make the page size and autosave interval configurable 6005830
* **settings:** preferences overlay with a notification toggle b2ee41e
* **setup:** configure remote accounts through the app-owned engine 4808dfb
* **setup:** discover account settings from an email address 3fce13a
* **setup:** resolve account settings from RFC 6186 SRV records 70cd13d
* **setup:** store app-owned remote account definitions 00be464
* **sieve:** manage server-side filter scripts 208b40c
* **sieve:** validate a script before installing it deed96b
* **smime:** show the signer identity and let the user trust it 0b7cd21
* **ui:** add a unified All Inboxes view e5de1fd
* **ui:** add structured message display modes e6a7bff
* **ui:** archive messages e183840
* **ui:** browse, open, and save individual attachments 7b00a38
* **ui:** collapse and expand threads f638d79
* **ui:** collapse quoted lines in the message view 43700ed
* **ui:** copy messages to another folder fa517a4
* **ui:** empty the current folder with a confirmation ab3d6b4
* **ui:** incremental folder search in the sidebar 67df452
* **ui:** jump between unread messages 2784682
* **ui:** list extracted HTML links in the message view 28a7c59
* **ui:** mark a whole thread read e0f15f7
* **ui:** multi-select messages for batch actions 6e550b0
* **ui:** nest replies in the threaded envelope list cdc4362
* **ui:** open links from the message 2731402
* **ui:** order the message list by date, sender, or subject 6c34c14
* **ui:** pick the target folder when moving messages df71ff7
* **ui:** save the loaded message as .eml 2d154bb
* **ui:** search inside the loaded message 1b4ce3c
* **ui:** send a desktop notification on new mail 922381f
* **ui:** show attachment count in the compose action bar 3a4efa7
* **ui:** toggle read/unread with N 2d9c566
* **ui:** toggle the full raw headers in the message view b914864
* **ui:** toggle the raw HTML source of a message bf00b12
* **ui:** undo the last delete, move, or flag 6ff01c8


### Bug Fixes

* **ci:** make the pipeline build the committed dependency set a9da491
* **install:** stop clobbering the SolverForge Linux launcher b931c91
* **lint:** satisfy current clippy lints 690e1c1
* **mail:** count unread with a dedicated server-side search b82a61d
* **mail:** emit CRLF and reject header injection in outgoing mail 74dda71
* **mail:** keep legacy runtime inventory out of the app store ade1fd6
* **mail:** keep the local store in sync with flag and move actions e23fab0
* **mail:** pair S/MIME keys with their matching certificate c1d774b
* **pgp:** verify signatures made by a signing subkey 6fe916c
* **settings:** render every preference row 7f0a706
* **setup:** gate app-owned remote flows until runtime support d351672
* **sieve:** frame literals correctly and bound the ManageSieve reads f22c812
* **sort:** default the message list to newest first 364b5aa
* **ui:** bind the folder sync and mark-read keys 8ae993c
* **ui:** defects found by driving every surface 7214d90
* **ui:** name the account form fields in full aef2988
* **ui:** stop clipping status messages off the status bar 4d33656

# Changelog

All notable changes to this project will be documented in this file. See [commit-and-tag-version](https://github.com/absolute-version/commit-and-tag-version) for commit guidelines.
