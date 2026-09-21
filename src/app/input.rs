/*! Top-level key dispatch. */

use crossterm::event::KeyEvent;

use crate::compose::FocusedField;
use crate::keys::{self, Action, ComposeFocus, ComposeKeyContext, EditMode, View};

use super::model::App;

impl App {
    pub(crate) fn compose_key_context(&self) -> ComposeKeyContext {
        self.compose_state
            .as_ref()
            .map(|cs| ComposeKeyContext {
                focus: match cs.focused {
                    FocusedField::From => ComposeFocus::From,
                    FocusedField::To
                    | FocusedField::Cc
                    | FocusedField::Bcc
                    | FocusedField::Subject => ComposeFocus::Header,
                    FocusedField::Body => ComposeFocus::Body,
                    FocusedField::Send
                    | FocusedField::Draft
                    | FocusedField::Attach
                    | FocusedField::Sign
                    | FocusedField::Encrypt
                    | FocusedField::Discard => ComposeFocus::ActionBar,
                },
                edit_mode: cs.edit_mode,
                body_search_active: cs.body.is_search_active(),
                autocomplete_visible: cs.autocomplete.is_some(),
                confirm_discard_visible: cs.confirm_discard,
            })
            .unwrap_or(ComposeKeyContext {
                focus: ComposeFocus::Header,
                edit_mode: EditMode::Nav,
                body_search_active: false,
                autocomplete_visible: false,
                confirm_discard_visible: false,
            })
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if self.view == View::Compose && self.compose_handle_attach_input(key) {
            return;
        }
        let action = if self.view == View::Compose {
            let ctx = self.compose_key_context();
            keys::resolve_compose_with_context(key, ctx)
        } else {
            keys::resolve(self.view, key)
        };

        match action {
            Action::Quit => self.running = false,
            Action::Back => self.go_back(),
            Action::MoveUp => self.move_selection(-1),
            Action::MoveDown => self.move_selection(1),
            Action::PageUp => self.page_up(),
            Action::PageDown => self.page_down(),
            Action::JumpTop => self.jump_top(),
            Action::JumpBottom => self.jump_bottom(),
            Action::Select => self.select_item(),
            Action::OpenMessage => self.load_message(),
            Action::Compose => self.compose(),
            Action::Reply => self.reply(false),
            Action::ReplyAll => self.reply(true),
            Action::Forward => self.forward(),
            Action::Delete => self.delete(),
            Action::ToggleFlag => self.toggle_flag(),
            Action::ToggleRead => self.toggle_read(),
            Action::SyncFolder => self.sync_folder(),
            Action::MarkFolderRead => self.mark_folder_read(),
            Action::DownloadAttachments => self.download_attachments(),
            Action::ToggleThread => self.toggle_thread(),
            Action::Search => self.enter_search(),
            Action::SearchSubmit => self.submit_search(),
            Action::SearchCancel => self.cancel_search(),
            Action::SearchInput(c) => self.search_query.push(c),
            Action::SearchBackspace => {
                self.search_query.pop();
            }
            Action::Refresh => self.refresh(),
            Action::SwitchAccount => self.enter_account_picker(),
            Action::ToggleHelp => self.toggle_help(),
            Action::FocusFolders => self.view = View::FolderList,
            Action::FocusEnvelopes => self.view = View::EnvelopeList,
            Action::ScrollUp => self.scroll(-1),
            Action::ScrollDown => self.scroll(1),
            Action::MoveMessage => self.enter_move_prompt(),
            Action::MoveInput(c) => self.move_target.push(c),
            Action::MoveBackspace => {
                self.move_target.pop();
            }
            Action::MoveSubmit => self.submit_move(),
            Action::MoveCancel => self.cancel_move(),
            Action::UnlockPrompt => self.enter_unlock_prompt(),
            Action::UnlockInput(c) => self.unlock_input(c),
            Action::UnlockBackspace => self.unlock_backspace(),
            Action::UnlockSubmit => self.submit_unlock(),
            Action::UnlockCancel => self.cancel_unlock(),
            Action::TrustSigner => self.trust_signer(),

            // ── Compose editor ───────────────────────────────────────
            Action::ComposeFieldNext => {
                if let Some(ref mut cs) = self.compose_state {
                    cs.autocomplete = None;
                    cs.focused = cs.focused.next();
                }
            }
            Action::ComposeFieldPrev => {
                if let Some(ref mut cs) = self.compose_state {
                    cs.autocomplete = None;
                    cs.focused = cs.focused.prev();
                }
            }
            Action::ComposeLeaveBodyNext => {
                if let Some(ref mut cs) = self.compose_state {
                    cs.body.clear_search();
                    cs.autocomplete = None;
                    cs.focused = cs.focused.next();
                }
            }
            Action::ComposeLeaveBodyPrev => {
                if let Some(ref mut cs) = self.compose_state {
                    cs.body.clear_search();
                    cs.autocomplete = None;
                    cs.focused = cs.focused.prev();
                }
            }
            Action::ComposeSend => self.compose_send(),
            Action::ComposeDiscard => self.compose_discard(),
            Action::ComposeConfirmDiscard => {
                self.compose_state = None;
                self.pending_draft = None;
                self.view = View::EnvelopeList;
            }
            Action::ComposeCancelDiscard => {
                if let Some(ref mut cs) = self.compose_state {
                    cs.confirm_discard = false;
                }
            }
            Action::ComposeInput(c) => {
                if let Some(ref mut cs) = self.compose_state {
                    // Auto-enter Insert on header text fields when typing.
                    if cs.edit_mode == EditMode::Nav
                        && matches!(
                            cs.focused,
                            FocusedField::To
                                | FocusedField::Cc
                                | FocusedField::Bcc
                                | FocusedField::Subject
                        )
                    {
                        cs.edit_mode = EditMode::Insert;
                    }

                    if cs.edit_mode == EditMode::Insert {
                        if let Some(field) = cs.focused_line_field_mut() {
                            field.push(c);
                            cs.dirty = true;
                        }
                    }
                }
                let is_address = {
                    let cs = self.compose_state.as_ref();
                    cs.map(|cs| {
                        cs.edit_mode == EditMode::Insert
                            && matches!(
                                cs.focused,
                                FocusedField::To | FocusedField::Cc | FocusedField::Bcc
                            )
                    })
                    .unwrap_or(false)
                };
                if is_address {
                    self.update_autocomplete();
                }
            }
            Action::ComposeBackspace => {
                let is_address = {
                    let cs = self.compose_state.as_ref();
                    cs.map(|cs| {
                        cs.edit_mode == EditMode::Insert
                            && matches!(
                                cs.focused,
                                FocusedField::To | FocusedField::Cc | FocusedField::Bcc
                            )
                    })
                    .unwrap_or(false)
                };
                if let Some(ref mut cs) = self.compose_state {
                    if cs.edit_mode == EditMode::Insert {
                        if let Some(field) = cs.focused_line_field_mut() {
                            field.pop();
                        }
                    }
                }
                if is_address {
                    self.update_autocomplete();
                }
            }
            Action::ComposeEnterInsert => {
                self.compose_enter_insert();
            }
            Action::ComposeExitToNav => {
                self.compose_exit_to_nav();
            }

            // ── EditorKey: forwarded to the focused compose field ────
            Action::EditorKey(key_event) => {
                self.handle_editor_key(key_event);
            }

            // ── Contacts ─────────────────────────────────────────────
            Action::OpenContacts => self.open_contacts(),
            Action::ContactNew => self.contact_new(),
            Action::ContactDelete => self.contact_delete(),
            Action::ContactEdit => self.contact_edit_selected(),
            Action::ContactSearch => self.contact_search_start(),
            Action::ContactSearchInput(c) => self.contact_search_input(c),
            Action::ContactSearchBackspace => self.contact_search_backspace(),
            Action::ContactSearchCancel => self.contact_search_cancel(),
            // ── Contact edit form ─────────────────────────────────────
            Action::ContactEditFieldNext => self.contact_edit_field_next(),
            Action::ContactEditFieldPrev => self.contact_edit_field_prev(),
            Action::ContactEditInput(c) => self.contact_edit_input(c),
            Action::ContactEditBackspace => self.contact_edit_backspace(),
            Action::ContactEditSave => self.contact_edit_save(),
            Action::ContactEditCancel => self.contact_edit_cancel(),
            Action::ContactEditActivate => self.contact_edit_activate(),

            // ── Identity list ─────────────────────────────────────────
            Action::OpenIdentities => self.open_identities(),
            Action::IdentityNew => self.identity_new(),
            Action::IdentityEditSelected => self.identity_edit_selected(),
            Action::IdentityDelete => self.identity_delete(),
            Action::IdentitySetDefault => self.identity_set_default(),
            Action::IdentityListUp => self.identity_list_move(-1),
            Action::IdentityListDown => self.identity_list_move(1),
            Action::IdentityListClose => self.identity_list_close(),
            // ── Identity edit form ────────────────────────────────────
            Action::IdentityEditFieldNext => self.identity_edit_field_next(),
            Action::IdentityEditFieldPrev => self.identity_edit_field_prev(),
            Action::IdentityEditInput(c) => self.identity_edit_input(c),
            Action::IdentityEditBackspace => self.identity_edit_backspace(),
            Action::IdentityEditToggle => self.identity_edit_toggle(),
            Action::IdentityEditSave => self.identity_edit_save(),
            Action::IdentityEditCancel => self.identity_edit_cancel(),

            Action::None => {}
        }
    }

    // ── Action handlers ─────────────────────────────────────────────
}
