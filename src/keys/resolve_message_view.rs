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
        KeyCode::Char('N') => Action::ToggleRead,
        KeyCode::Char('P') => Action::UnlockPrompt,
        KeyCode::Char('T') => Action::TrustSigner,
        KeyCode::Char('o') => Action::OpenAttachments,
        KeyCode::Char('h') => Action::ToggleHeaders,
        KeyCode::Char('Q') => Action::ToggleQuotes,
        KeyCode::Char('v') => Action::OpenInviteReply,
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
