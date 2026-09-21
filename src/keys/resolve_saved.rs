/*! Saved-search overlay and naming prompt bindings. */

use crossterm::event::{KeyCode, KeyEvent};

use super::action::Action;

/// The saved-search list.
pub(super) fn resolve_saved_searches(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Action::SavedSearchClose,
        KeyCode::Char('j') | KeyCode::Down => Action::SavedSearchNext,
        KeyCode::Char('k') | KeyCode::Up => Action::SavedSearchPrev,
        KeyCode::Enter => Action::SavedSearchRun,
        KeyCode::Char('d') => Action::SavedSearchDelete,
        _ => Action::None,
    }
}

/// Naming the active query before saving it.
pub(super) fn resolve_save_search(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => Action::SaveSearchSubmit,
        KeyCode::Esc => Action::SaveSearchCancel,
        KeyCode::Backspace => Action::SaveSearchBackspace,
        KeyCode::Char(c) => Action::SaveSearchInput(c),
        _ => Action::None,
    }
}
