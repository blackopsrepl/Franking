/*! Help must remain reachable and navigable across views and window sizes. */

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};

use crate::app::App;
use crate::compose::{ComposeMode, ComposeState};
use crate::keys::View;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn f1_shows_help_without_losing_compose() {
    let mut app = App::new(Some("test".into()));
    app.compose_state = Some(ComposeState::new(ComposeMode::New, Some("test".into())));
    app.view = View::Compose;
    app.handle_key(key(KeyCode::F(1)));
    assert_eq!(app.view, View::Help);
    app.handle_key(key(KeyCode::F(1)));
    assert_eq!(app.view, View::Compose);
    assert!(app.compose_state.is_some());
}

#[test]
fn help_end_is_reachable_and_can_be_left_at_narrow_width() {
    let mut app = App::new(None);
    app.view = View::Help;
    let mut terminal = Terminal::new(TestBackend::new(48, 20)).unwrap();
    terminal
        .draw(|frame| crate::ui::render(&mut app, frame))
        .unwrap();
    assert!(app.help_max_scroll > 100);
    app.handle_key(key(KeyCode::Char('G')));
    assert_eq!(app.help_scroll, app.help_max_scroll);
    app.handle_key(key(KeyCode::Char('k')));
    assert_eq!(app.help_scroll, app.help_max_scroll - 1);
    app.handle_key(key(KeyCode::Char('g')));
    assert_eq!(app.help_scroll, 0);
}
