/*! Key resolution for the account list and account form. */

use crossterm::event::{KeyCode, KeyEvent};

use super::action::Action;

pub(super) fn resolve_account_list(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc => Action::Back,
        KeyCode::Char('q') => Action::Back,
        KeyCode::Char('j') | KeyCode::Down => Action::MoveDown,
        KeyCode::Char('k') | KeyCode::Up => Action::MoveUp,
        KeyCode::Enter => Action::Select,
        KeyCode::Char('s') => Action::SetDefaultAccount,
        KeyCode::Char('d') => Action::DeleteAccount,
        KeyCode::Char('a') => Action::AccountNew,
        KeyCode::Char('e') => Action::AccountEdit,
        _ => Action::None,
    }
}

pub(super) fn resolve_attachment_view(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Action::PreviewScrollDown,
        KeyCode::Char('k') | KeyCode::Up => Action::PreviewScrollUp,
        KeyCode::Char(' ') | KeyCode::PageDown => Action::PreviewScrollDown,
        KeyCode::Esc | KeyCode::Char('q') => Action::PreviewClose,
        _ => Action::None,
    }
}

pub(super) fn resolve_invite(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('a') => {
            Action::InviteRespond(crate::mail::calendar_reply::PartStat::Accepted)
        }
        KeyCode::Char('t') => {
            Action::InviteRespond(crate::mail::calendar_reply::PartStat::Tentative)
        }
        KeyCode::Char('d') => {
            Action::InviteRespond(crate::mail::calendar_reply::PartStat::Declined)
        }
        KeyCode::Esc | KeyCode::Char('q') => Action::InviteCancel,
        _ => Action::None,
    }
}

pub(super) fn resolve_schedule(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => Action::ScheduleSubmit,
        KeyCode::Esc => Action::ScheduleCancel,
        KeyCode::Backspace => Action::ScheduleBackspace,
        KeyCode::Char(c) => Action::ScheduleInput(c),
        _ => Action::None,
    }
}

pub(super) fn resolve_settings(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char(' ') | KeyCode::Enter => Action::SettingsToggleNotifications,
        KeyCode::Char('j') | KeyCode::Down => Action::SettingsNext,
        KeyCode::Char('k') | KeyCode::Up => Action::SettingsPrev,
        KeyCode::Esc | KeyCode::Char('q') => Action::SettingsClose,
        _ => Action::None,
    }
}

pub(super) fn resolve_outbox(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Action::OutboxNext,
        KeyCode::Char('k') | KeyCode::Up => Action::OutboxPrev,
        KeyCode::Enter => Action::OutboxSend,
        KeyCode::Char('d') => Action::OutboxDiscard,
        KeyCode::Esc | KeyCode::Char('q') => Action::OutboxClose,
        _ => Action::None,
    }
}

pub(super) fn resolve_file_picker(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Action::FilePickerNext,
        KeyCode::Char('k') | KeyCode::Up => Action::FilePickerPrev,
        KeyCode::Enter => Action::FilePickerEnter,
        KeyCode::Backspace | KeyCode::Left => Action::FilePickerUp,
        KeyCode::Esc | KeyCode::Char('q') => Action::FilePickerClose,
        _ => Action::None,
    }
}

pub(super) fn resolve_account_edit(key: KeyEvent) -> Action {
    if key
        .modifiers
        .contains(crossterm::event::KeyModifiers::CONTROL)
        && key.code == KeyCode::Char('d')
    {
        return Action::AccountEditDiscover;
    }
    match key.code {
        KeyCode::Tab => Action::AccountEditFieldNext,
        KeyCode::BackTab => Action::AccountEditFieldPrev,
        KeyCode::Enter => Action::AccountEditSave,
        KeyCode::Esc => Action::AccountEditCancel,
        KeyCode::Backspace => Action::AccountEditBackspace,
        KeyCode::Char(' ') => Action::AccountEditToggleDefault,
        KeyCode::Char(c) => Action::AccountEditInput(c),
        _ => Action::None,
    }
}
