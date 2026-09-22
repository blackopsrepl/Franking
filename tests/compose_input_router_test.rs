use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use franking::keys::{
    resolve_compose_with_context, Action, ComposeFocus, ComposeKeyContext, EditMode,
};
use pretty_assertions::assert_eq;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

fn ctx(
    focus: ComposeFocus,
    edit_mode: EditMode,
    body_search_active: bool,
    autocomplete_visible: bool,
    confirm_discard_visible: bool,
) -> ComposeKeyContext {
    ComposeKeyContext {
        focus,
        edit_mode,
        body_search_active,
        autocomplete_visible,
        confirm_discard_visible,
    }
}

#[test]
fn precedence_ctrl_shortcuts_override_everything() {
    let contexts = [
        ctx(ComposeFocus::Header, EditMode::Nav, false, false, false),
        ctx(ComposeFocus::Body, EditMode::Insert, false, true, false),
        ctx(ComposeFocus::ActionBar, EditMode::Nav, false, false, false),
    ];
    for c in contexts {
        assert_eq!(
            resolve_compose_with_context(ctrl(KeyCode::Char('c')), c),
            Action::ComposeDiscard
        );
    }
}

#[test]
fn precedence_discard_confirm_blocks_other_layers() {
    let c = ctx(ComposeFocus::Body, EditMode::Insert, false, true, true);
    assert_eq!(
        resolve_compose_with_context(key(KeyCode::Char('y')), c),
        Action::ComposeConfirmDiscard
    );
    assert_eq!(
        resolve_compose_with_context(key(KeyCode::Esc), c),
        Action::ComposeCancelDiscard
    );
    assert_eq!(
        resolve_compose_with_context(key(KeyCode::Char('j')), c),
        Action::None
    );
    assert_eq!(
        resolve_compose_with_context(ctrl(KeyCode::Char('c')), c),
        Action::None
    );
    assert_eq!(
        resolve_compose_with_context(ctrl(KeyCode::Char('l')), c),
        Action::None
    );
}

#[test]
fn precedence_autocomplete_intercepts_navigation_acceptance_keys() {
    let c = ctx(ComposeFocus::Header, EditMode::Insert, false, true, false);
    for code in [
        KeyCode::Down,
        KeyCode::Up,
        KeyCode::Enter,
        KeyCode::Tab,
        KeyCode::Esc,
    ] {
        assert_eq!(
            resolve_compose_with_context(key(code), c),
            Action::EditorKey(key(code))
        );
    }
}

#[test]
fn body_focus_passes_keys_through_except_tab_and_escape() {
    let c = ctx(ComposeFocus::Body, EditMode::Nav, false, false, false);
    assert_eq!(
        resolve_compose_with_context(key(KeyCode::Char('j')), c),
        Action::EditorKey(key(KeyCode::Char('j')))
    );
    assert_eq!(
        resolve_compose_with_context(key(KeyCode::Backspace), c),
        Action::EditorKey(key(KeyCode::Backspace))
    );
    // Esc leaves the message rather than being swallowed by the body editor.
    assert_eq!(
        resolve_compose_with_context(key(KeyCode::Esc), c),
        Action::ComposeDiscard
    );
    assert_eq!(
        resolve_compose_with_context(key(KeyCode::Tab), c),
        Action::ComposeFieldNext
    );
    assert_eq!(
        resolve_compose_with_context(key(KeyCode::BackTab), c),
        Action::ComposeFieldPrev
    );
}

#[test]
fn body_search_tab_clears_search_before_focus_change() {
    let c = ctx(ComposeFocus::Body, EditMode::Nav, true, false, false);
    assert_eq!(
        resolve_compose_with_context(key(KeyCode::Tab), c),
        Action::ComposeLeaveBodyNext
    );
    assert_eq!(
        resolve_compose_with_context(key(KeyCode::BackTab), c),
        Action::ComposeLeaveBodyPrev
    );
}

#[test]
fn non_body_up_down_cycle_fields() {
    let c = ctx(ComposeFocus::Header, EditMode::Nav, false, false, false);
    assert_eq!(
        resolve_compose_with_context(key(KeyCode::Down), c),
        Action::ComposeFieldNext
    );
    assert_eq!(
        resolve_compose_with_context(key(KeyCode::Up), c),
        Action::ComposeFieldPrev
    );
}

#[test]
fn header_text_is_forwarded_to_focused_field() {
    let c = ctx(ComposeFocus::Header, EditMode::Insert, false, false, false);
    assert_eq!(
        resolve_compose_with_context(key(KeyCode::Char('j')), c),
        Action::EditorKey(key(KeyCode::Char('j')))
    );
    assert_eq!(
        resolve_compose_with_context(key(KeyCode::Char('k')), c),
        Action::EditorKey(key(KeyCode::Char('k')))
    );
}
