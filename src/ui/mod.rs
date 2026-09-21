mod account_edit;
mod account_list;
pub mod action_bar;
mod attachment_list;
mod compose;
mod contact_edit;
mod contacts;
mod envelope_list;
mod file_picker;
mod folder_list;
mod folder_prompt;
mod help;
mod identity_edit;
mod identity_list;
mod invite;
mod links;
mod message_view;
mod move_prompt;
mod outbox;
mod passphrase_prompt;
mod pgp_keys;
mod saved_searches;
mod search;
mod settings;
mod sieve;
mod status_bar;
pub mod util;

use ratatui::prelude::*;

use crate::app::App;
use crate::keys::View;

/// Main render dispatch — called once per frame.
pub fn render(app: &mut App, frame: &mut Frame) {
    let area = frame.area();
    app.last_terminal_height = area.height;
    app.last_terminal_width = area.width;

    // ── Full-screen takeover views ───────────────────────────────
    match app.view {
        View::Compose => {
            compose::render(app, frame);
            return;
        }
        View::SieveScripts | View::SieveName | View::SieveEdit => {
            sieve::render(app, frame);
            return;
        }
        View::FilePicker => {
            file_picker::render(app, frame);
            return;
        }
        View::SavedSearches => {
            saved_searches::render(app, frame);
            return;
        }
        View::Keys | View::KeysPrompt => {
            pgp_keys::render(app, frame);
            return;
        }
        View::Contacts | View::ContactSearch => {
            contacts::render(app, frame);
            return;
        }
        View::ContactEdit | View::IdentityList | View::IdentityEdit => {
            // Fall through to normal layout; overlay rendered below.
        }
        _ => {}
    }

    // Top-level vertical layout: header (1), main body (fill), status bar (1)
    let outer = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .split(area);

    // ── Header ──────────────────────────────────────────────────
    status_bar::render_header(app, frame, outer[0]);

    // ── Main body ───────────────────────────────────────────────
    match app.view {
        View::MessageView => {
            message_view::render(app, frame, outer[1]);
        }
        _ => {
            let main =
                Layout::horizontal([Constraint::Length(22), Constraint::Fill(1)]).split(outer[1]);
            folder_list::render(app, frame, main[0]);
            envelope_list::render(app, frame, main[1]);
        }
    }

    // ── Status / search / move bar ──────────────────────────────
    match app.view {
        View::Search => search::render(app, frame, outer[2]),
        View::MovePrompt => move_prompt::render(app, frame, outer[2]),
        View::PassphrasePrompt => passphrase_prompt::render(app, frame, outer[2]),
        View::FolderPrompt => folder_prompt::render(app, frame, outer[2]),
        View::SchedulePrompt => outbox::render_schedule_prompt(app, frame, outer[2]),
        View::InviteReply => invite::render_prompt(app, frame, outer[2]),
        View::MessageSearch => message_view::render_search_prompt(app, frame, outer[2]),
        _ => status_bar::render(app, frame, outer[2]),
    }

    // ── Overlays ────────────────────────────────────────────────
    if app.view == View::AccountList {
        account_list::render(app, frame);
    }
    if app.view == View::AccountEdit {
        account_edit::render(app, frame);
    }
    if app.view == View::Help {
        help::render(app, frame);
    }
    if app.view == View::AttachmentList {
        attachment_list::render(app, frame);
    }
    if app.view == View::LinkList {
        links::render(app, frame);
    }
    if app.view == View::Outbox {
        outbox::render(app, frame);
    }
    if app.view == View::AttachmentView {
        attachment_list::render_preview(app, frame);
    }
    if app.view == View::Settings {
        settings::render(app, frame);
    }
    if app.view == View::ContactEdit {
        contact_edit::render(app, frame);
    }
    if app.view == View::IdentityList {
        identity_list::render(app, frame);
    }
    if app.view == View::IdentityEdit {
        identity_edit::render(app, frame);
    }
}
