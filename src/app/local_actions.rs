/*! Dispatch for local follow-up, attachment-library and snippet actions.
Kept apart from the main key match so the top-level dispatcher stays small. */

use crate::keys::Action;

use super::model::App;

impl App {
    /// Handle a local action. Returns false when it is not one.
    pub(crate) fn try_local_action(&mut self, action: &Action) -> bool {
        match action {
            Action::OpenReplyLater => self.open_reply_later(),
            Action::ToggleReplyLater => self.toggle_reply_later(),
            Action::ToggleMuteConversation => self.toggle_mute_conversation(),
            Action::OpenAttachmentLibrary => self.open_attachment_library(),
            Action::AttachmentLibraryNext => self.attachment_library_next(),
            Action::AttachmentLibraryPrev => self.attachment_library_prev(),
            Action::AttachmentLibraryOpen => self.open_attachment_library_item(),
            Action::AttachmentLibraryClose => self.close_attachment_library(),
            Action::OpenSnippets => self.open_snippets(),
            Action::SnippetsNext => self.snippets_next(),
            Action::SnippetsPrev => self.snippets_prev(),
            Action::SnippetsInsert => self.insert_snippet(),
            Action::SnippetNew => self.begin_save_snippet(),
            Action::SnippetsDelete => self.delete_snippet(),
            Action::SnippetsClose => self.snippets_close(),
            Action::SnippetNameInput(c) => self.snippet_name_input(*c),
            Action::SnippetNameBackspace => self.snippet_name_backspace(),
            Action::SnippetNameSubmit => self.submit_snippet_name(),
            Action::SnippetNameCancel => self.cancel_snippet_name(),
            _ => return false,
        }
        true
    }
}
