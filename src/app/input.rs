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
                    | FocusedField::Files
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
        if self.view == View::Compose && self.compose_handle_attach_list(key) {
            return;
        }
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
            Action::ToggleSelect => self.toggle_select(),
            Action::ClearSelection => self.clear_selection(),
            Action::ToggleFlag => self.toggle_flag(),
            Action::ToggleRead => self.toggle_read(),
            Action::SyncFolder => self.sync_folder(),
            Action::MarkFolderRead => self.mark_folder_read(),
            Action::DownloadAttachments => self.download_attachments(),
            Action::ToggleThread => self.toggle_thread(),
            Action::Search => self.enter_search(),
            Action::SearchSubmit => self.submit_search(),
            Action::ToggleSearchScope => self.toggle_search_scope(),
            Action::SearchCancel => self.cancel_search(),
            Action::SearchInput(c) => self.search_query.push(c),
            Action::SearchBackspace => self.search_backspace(),
            Action::Refresh => self.refresh(),
            Action::SwitchAccount => self.enter_account_picker(),
            Action::ToggleHelp => self.toggle_help(),
            Action::ToggleHeaders => self.toggle_headers(),
            Action::ToggleQuotes => self.toggle_quotes(),
            Action::SaveMessage => self.save_message(),
            Action::Undo => self.undo(),
            Action::Archive => self.archive(),
            Action::SetDefaultAccount => self.set_default_account(),
            Action::DeleteAccount => self.delete_account(),
            Action::AccountNew => self.open_account_new(),
            Action::AccountEdit => self.open_account_edit(),
            Action::AccountEditFieldNext => self.account_form_step(1),
            Action::AccountEditFieldPrev => self.account_form_step(-1),
            Action::AccountEditInput(c) => self.account_form_input(c),
            Action::AccountEditBackspace => self.account_form_backspace(),
            Action::AccountEditToggleDefault => self.account_form_toggle_default(),
            Action::AccountEditSave => self.account_form_save(),
            Action::AccountEditCancel => self.cancel_account_form(),
            Action::AccountEditDiscover => self.discover_account_settings(),
            Action::OpenFilePicker => self.open_file_picker(),
            Action::FilePickerNext => self.file_picker_next(),
            Action::FilePickerPrev => self.file_picker_prev(),
            Action::FilePickerEnter => self.file_picker_enter(),
            Action::FilePickerUp => self.file_picker_up(),
            Action::FilePickerClose => self.close_file_picker(),
            Action::OpenOutbox => self.open_outbox(),
            Action::OutboxNext => self.outbox_next(),
            Action::OutboxPrev => self.outbox_prev(),
            Action::OutboxSend => self.send_outbox_item(),
            Action::OutboxDiscard => self.discard_outbox_item(),
            Action::OutboxClose => self.close_outbox(),
            Action::OpenSettings => self.open_settings(),
            Action::SettingsToggleNotifications => self.toggle_notifications(),
            Action::SettingsClose => self.close_settings(),
            Action::OpenLinks => self.open_links(),
            Action::LinkNext => self.link_next(),
            Action::LinkPrev => self.link_prev(),
            Action::LinkOpen => self.open_selected_link(),
            Action::LinkClose => self.close_links(),
            Action::CollapseThread => self.collapse_thread(),
            Action::ExpandThread => self.expand_thread(),
            Action::EmptyFolder => self.empty_folder(),
            Action::SearchMessage => self.enter_message_search(),
            Action::MessageSearchInput(c) => self.message_search_input(c),
            Action::MessageSearchBackspace => self.message_search_backspace(),
            Action::MessageSearchSubmit => self.submit_message_search(),
            Action::MessageSearchCancel => self.cancel_message_search(),
            Action::NextMatch | Action::PrevMatch => {
                self.step_match(matches!(action, Action::NextMatch))
            }
            Action::FocusFolders => self.view = View::FolderList,
            Action::FocusEnvelopes => self.view = View::EnvelopeList,
            Action::ScrollUp => self.scroll(-1),
            Action::ScrollDown => self.scroll(1),
            Action::MoveMessage => self.enter_move_prompt(),
            Action::CopyMessage => self.enter_copy_prompt(),
            Action::MoveInput(c) => self.move_prompt_input(c),
            Action::MoveNext => self.move_next_candidate(),
            Action::MovePrev => self.move_prev_candidate(),
            Action::MoveBackspace => self.move_prompt_backspace(),
            Action::MoveSubmit => self.submit_move(),
            Action::MoveCancel => self.cancel_move(),
            Action::UnlockPrompt => self.enter_unlock_prompt(),
            Action::UnlockInput(c) => self.unlock_input(c),
            Action::UnlockBackspace => self.unlock_backspace(),
            Action::UnlockSubmit => self.submit_unlock(),
            Action::UnlockCancel => self.cancel_unlock(),
            Action::TrustSigner => self.trust_signer(),
            Action::OpenAttachments => self.open_attachments(),
            Action::AttachmentNext => self.attachment_next(),
            Action::AttachmentPrev => self.attachment_prev(),
            Action::AttachmentOpen => self.open_selected_attachment(),
            Action::AttachmentSave => self.save_selected_attachment(),
            Action::AttachmentClose => self.close_attachments(),
            Action::FolderNew => self.folder_prompt_new(),
            Action::FolderRename => self.folder_prompt_rename(),
            Action::FolderDelete => self.folder_prompt_delete(),
            Action::FolderPromptInput(c) => self.folder_prompt_input(c),
            Action::FolderPromptBackspace => self.folder_prompt_backspace(),
            Action::FolderPromptSubmit => self.submit_folder_prompt(),
            Action::FolderPromptCancel => self.cancel_folder_prompt(),
            Action::OpenSieve => self.open_sieve(),
            Action::SieveNext => self.sieve_next(),
            Action::SievePrev => self.sieve_prev(),
            Action::SieveActivate => self.sieve_activate(),
            Action::SieveDeactivate => self.sieve_deactivate(),
            Action::SieveEdit => self.sieve_edit(),
            Action::SieveNew => self.sieve_new(),
            Action::SieveDelete => self.sieve_delete(),
            Action::SieveSave => self.sieve_save(),
            Action::SieveClose => self.close_sieve(),
            Action::SieveEscape => self.sieve_escape(),
            Action::SieveNameInput(c) => self.sieve_name_input(c),
            Action::SieveNameBackspace => self.sieve_name_backspace(),
            Action::SieveNameSubmit => self.sieve_name_submit(),
            Action::SieveNameCancel => self.sieve_name_cancel(),
            Action::SieveEditorKey(key) => self.sieve_editor_key(key),
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
                self.clear_autosave();
                self.compose_state = None;
                self.pending_draft = None;
                self.view = View::EnvelopeList;
            }
            Action::ComposeCancelDiscard => {
                if let Some(ref mut cs) = self.compose_state {
                    cs.confirm_discard = false;
                }
            }
            Action::ComposeInput(c) => self.compose_input(c),
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
