/*! Dispatch for the account-scoped local workflow actions.
Kept apart from the main key match so the top-level dispatcher stays small. */

use crate::keys::Action;

use super::model::App;

impl App {
    /// Handle a local-workflow action. Returns false when it is not one.
    pub(crate) fn try_local_action(&mut self, action: &Action) -> bool {
        match action {
            Action::RouteSender(route) => self.route_selected_sender(*route),
            Action::OpenFollowup(marker) => self.open_followup(*marker),
            Action::ToggleMarker(marker) => self.toggle_marker(*marker),
            Action::ToggleMuteConversation => self.toggle_mute_conversation(),
            Action::ToggleLoudConversation => self.toggle_loud_conversation(),
            Action::OpenResurface => self.open_resurface_prompt(),
            Action::ResurfaceInput(c) => self.resurface_input(*c),
            Action::ResurfaceBackspace => self.resurface_backspace(),
            Action::ResurfaceSubmit => self.submit_resurface(),
            Action::ResurfaceCancel => self.cancel_resurface(),
            Action::OpenPlacePrompt => self.open_place_prompt(),
            Action::RouteMessage(route) => self.place_message(*route),
            Action::PlaceCancel => self.cancel_place(),
            Action::OpenNote => self.open_note_prompt(),
            Action::OpenSubjectAlias => self.open_subject_alias_prompt(),
            Action::AnnotationInput(c) => self.annotation_input(*c),
            Action::AnnotationBackspace => self.annotation_backspace(),
            Action::AnnotationSubmit => self.submit_annotation(),
            Action::AnnotationCancel => self.cancel_annotation(),
            Action::OpenAttachmentLibrary => self.open_attachment_library(),
            Action::AttachmentLibraryNext => self.attachment_library_next(),
            Action::AttachmentLibraryPrev => self.attachment_library_prev(),
            Action::AttachmentLibraryOpen => self.open_attachment_library_item(),
            Action::AttachmentLibraryClose => self.close_attachment_library(),
            Action::OpenReadTogether => self.open_read_together(),
            Action::OpenBypassPrompt => self.open_bypass_prompt(),
            Action::BypassInput(c) => self.bypass_input(*c),
            Action::BypassBackspace => self.bypass_backspace(),
            Action::BypassSubmit => self.submit_bypass(),
            Action::BypassGenerate => self.bypass_generate(),
            Action::BypassCancel => self.cancel_bypass(),
            Action::ToggleCoverReveal => self.toggle_cover_reveal(),
            Action::OpenFocusReply => self.open_focus_reply(),
            Action::FocusNext => self.focus_next(),
            Action::FocusPrev => self.focus_prev(),
            Action::FocusDone => self.focus_done(),
            Action::FocusReply => self.focus_reply(false),
            Action::FocusReplyAll => self.focus_reply(true),
            _ => return false,
        }
        true
    }
}

impl App {
    /// Handle a snippet action. Returns false when it is not one.
    pub(crate) fn try_snippet_action(&mut self, action: &Action) -> bool {
        match action {
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

impl App {
    /// Handle a clip action. Returns false when it is not one.
    pub(crate) fn try_clip_action(&mut self, action: &Action) -> bool {
        match action {
            Action::OpenClips => self.open_clips(),
            Action::ClipsNext => self.clips_next(),
            Action::ClipsPrev => self.clips_prev(),
            Action::ClipCopy => self.copy_clip(),
            Action::ClipDelete => self.delete_clip(),
            Action::ClipsClose => self.close_clips(),
            Action::OpenClipPrompt => self.open_clip_prompt(),
            Action::ClipInput(c) => self.clip_input(*c),
            Action::ClipBackspace => self.clip_backspace(),
            Action::ClipSubmit => self.submit_clip(),
            Action::ClipCancel => self.cancel_clip(),
            _ => return false,
        }
        true
    }
}
