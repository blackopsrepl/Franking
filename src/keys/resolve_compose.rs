/*! Compose key resolution. */

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::action::Action;
use super::compose_context::{ComposeFocus, ComposeKeyContext};

/// Resolve compose keys with compose-state context.
///
/// Compose is focus-driven: the shell owns modal overlays, field cycling, and
/// action-bar activation, while the focused field handles its own editing.
///
/// Priority order (highest first):
/// 1) discard-confirm modal interception
/// 2) global compose shortcuts (`Ctrl+C` / `Ctrl+Q`)
/// 3) autocomplete popup navigation/accept keys
/// 4) compose shell controls (`Tab`, `Shift+Tab`, action-bar `Enter` / `Esc`)
/// 5) passthrough to the focused compose field
pub fn resolve_compose_with_context(key: KeyEvent, ctx: ComposeKeyContext) -> Action {
    // Discard confirmation modal owns key handling while visible.
    if ctx.confirm_discard_visible {
        return match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => Action::ComposeConfirmDiscard,
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => Action::ComposeCancelDiscard,
            _ => Action::None,
        };
    }
    // Allow Ctrl+C / Ctrl+Q globally in compose as quit-discard
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') | KeyCode::Char('q') => Action::ComposeDiscard,
            _ => Action::EditorKey(key),
        };
    }
    // If autocomplete popup is open, let app-level popup handler own navigation
    // and acceptance keys.
    if ctx.autocomplete_visible {
        match key.code {
            KeyCode::Down | KeyCode::Up | KeyCode::Enter | KeyCode::Tab | KeyCode::Esc => {
                return Action::EditorKey(key);
            }
            _ => {}
        }
    }
    match key.code {
        KeyCode::Tab if ctx.focus == ComposeFocus::Body && ctx.body_search_active => {
            Action::ComposeLeaveBodyNext
        }
        KeyCode::BackTab if ctx.focus == ComposeFocus::Body && ctx.body_search_active => {
            Action::ComposeLeaveBodyPrev
        }
        KeyCode::Tab => Action::ComposeFieldNext,
        KeyCode::BackTab => Action::ComposeFieldPrev,
        KeyCode::Down if ctx.focus != ComposeFocus::Body => Action::ComposeFieldNext,
        KeyCode::Up if ctx.focus != ComposeFocus::Body => Action::ComposeFieldPrev,
        KeyCode::Enter if ctx.focus == ComposeFocus::ActionBar => Action::ComposeEnterInsert,
        KeyCode::Esc if ctx.focus == ComposeFocus::ActionBar => Action::ComposeExitToNav,
        // Esc leaves the message from anywhere but the action bar, where it
        // steps back to the body. Without this, abandoning a reply meant
        // tabbing through the whole action bar to reach Discard, and Esc in the
        // body did nothing at all.
        KeyCode::Esc
            if matches!(
                ctx.focus,
                ComposeFocus::Header | ComposeFocus::From | ComposeFocus::Body
            ) =>
        {
            Action::ComposeDiscard
        }
        _ => Action::EditorKey(key),
    }
}
