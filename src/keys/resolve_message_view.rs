/*! Key resolution for the message view. */

use crossterm::event::{KeyCode, KeyEvent};

use super::action::Action;

pub(super) fn resolve_message_view(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Action::Back,
        KeyCode::Char('j') | KeyCode::Down => Action::ScrollDown,
        KeyCode::Char('k') | KeyCode::Up => Action::ScrollUp,
        KeyCode::Char(' ') => Action::PageDown,
        KeyCode::Char('r') => Action::Reply,
        KeyCode::Char('R') => Action::ReplyAll,
        KeyCode::Char('f') => Action::Forward,
        KeyCode::Char('d') => Action::Delete,
        KeyCode::Char('a') => Action::DownloadAttachments,
        KeyCode::Char('Z') => Action::DownloadAttachmentsZip,
        KeyCode::Char('N') => Action::ToggleRead,
        KeyCode::Char('L') => Action::OpenFollowup(crate::db::message_markers::Marker::ReplyLater),
        KeyCode::Char('D') => Action::OpenFollowup(crate::db::message_markers::Marker::Saved),
        KeyCode::Char('y') => Action::ToggleMarker(crate::db::message_markers::Marker::ReplyLater),
        KeyCode::Char('Y') => Action::ToggleMarker(crate::db::message_markers::Marker::Saved),
        KeyCode::Char('M') => Action::ToggleMuteConversation,
        KeyCode::Char('b') => Action::OpenResurface,
        KeyCode::Char('x') => Action::OpenPlacePrompt,
        KeyCode::Char('1') => Action::RouteSender(crate::db::sender_routes::Route::Inbox),
        KeyCode::Char('2') => Action::RouteSender(crate::db::sender_routes::Route::Reading),
        KeyCode::Char('3') => Action::RouteSender(crate::db::sender_routes::Route::Receipts),
        KeyCode::Char('4') => Action::RouteSender(crate::db::sender_routes::Route::Blocked),
        KeyCode::Char('5') => Action::RouteSender(crate::db::sender_routes::Route::Screening),
        KeyCode::Char('P') => Action::UnlockPrompt,
        KeyCode::Char('T') => Action::TrustSigner,
        KeyCode::Char('o') => Action::OpenAttachments,
        KeyCode::Char('h') => Action::ToggleHeaders,
        KeyCode::Char('Q') => Action::ToggleQuotes,
        KeyCode::Char('H') => Action::ToggleHtmlSource,
        KeyCode::Char('c') => Action::AddToPlanner,
        KeyCode::Char('s') => Action::SaveMessage,
        KeyCode::Char('z') => Action::Undo,
        KeyCode::Char('e') => Action::Archive,
        KeyCode::Char('C') => Action::CopyMessage,
        KeyCode::Char('l') => Action::OpenLinks,
        KeyCode::Char('/') => Action::SearchMessage,
        KeyCode::Char('n') => Action::NextMatch,
        KeyCode::Char('p') => Action::PrevMatch,
        KeyCode::Char('?') => Action::ToggleHelp,
        KeyCode::Char('g') => Action::JumpTop,
        KeyCode::Char('G') => Action::JumpBottom,
        _ => Action::None,
    }
}
