/*! Calendar invitation replies: build a draft reply for review. */

use crate::compose::{ComposeMode, ComposeState};
use crate::keys::View;
use crate::mail::calendar_reply::{build_reply, PartStat};

use super::model::App;

impl App {
    /// Open the response prompt when the message holds a replyable invitation.
    pub(crate) fn open_invite_reply(&mut self) {
        let invitation = self
            .message_content
            .as_ref()
            .and_then(|message| message.invitation());
        if invitation.as_ref().is_some_and(|event| event.cancelled) {
            self.set_status("This invitation was cancelled; there is nothing to answer.");
            return;
        }
        let Some(event) = invitation.filter(|event| event.is_repliable()) else {
            self.set_status("This message has no replyable invitation.");
            return;
        };
        self.invite_pending = Some(event);
        self.view = View::InviteReply;
    }

    pub(crate) fn cancel_invite_reply(&mut self) {
        self.invite_pending = None;
        self.view = View::MessageView;
    }

    /// Build a reply draft addressed to the organizer and open it in compose.
    pub(crate) fn respond_invitation(&mut self, partstat: PartStat) {
        let Some(event) = self.invite_pending.take() else {
            return;
        };
        self.view = View::MessageView;

        let attendee = self
            .identity_email()
            .unwrap_or_else(|| "unknown@invalid".to_string());
        let Some(reply) = build_reply(&event, partstat, &attendee) else {
            self.set_error("The invitation is missing a UID or organizer.");
            return;
        };

        let path = match write_reply_file(&event, &reply) {
            Ok(path) => path,
            Err(error) => {
                self.set_error(&format!("Could not write the reply: {error}"));
                return;
            }
        };

        let organizer = event.organizer.clone().unwrap_or_default();
        let summary = event
            .summary
            .clone()
            .unwrap_or_else(|| "Invitation".to_string());

        let mut state = ComposeState::new(ComposeMode::New, self.acct_owned());
        self.load_identities_into(&mut state);
        state.to = organizer;
        state.subject = format!("{}: {summary}", partstat.verb());
        state.body = crate::compose_editor::ComposeEditor::from_text(&format!(
            "{} the invitation \"{summary}\".\n",
            partstat.verb()
        ));
        state.attachments = vec![path.display().to_string()];
        state.dirty = true;
        self.compose_state = Some(state);
        self.view = View::Compose;
        self.set_status("Review the calendar reply and press Send.");
    }

    /// Address the current account's default identity would send from.
    fn identity_email(&self) -> Option<String> {
        let account = self.acct_owned();
        let conn = self.db.as_ref()?;
        let identity = match account {
            Some(account) => crate::identities::get_default(conn, &account)
                .ok()
                .flatten(),
            None => None,
        };
        identity.map(|identity| identity.email)
    }
}

/// Write the reply payload under a unique name in the temp directory.
fn write_reply_file(
    event: &crate::mail::calendar::Event,
    reply: &str,
) -> std::io::Result<std::path::PathBuf> {
    let stem = event
        .uid
        .as_deref()
        .map(crate::mail::attachments::safe_file_name)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "invitation".to_string());
    let path = std::env::temp_dir().join(format!("{stem}.reply.ics"));
    std::fs::write(&path, reply)?;
    Ok(path)
}
