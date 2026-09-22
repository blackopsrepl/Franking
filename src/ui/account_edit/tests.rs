//! Buffer tests for the account form.

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use crate::account_edit::AccountEditState;
use crate::app::App;

use super::render;

fn drawn(app: &App) -> String {
    let backend = TestBackend::new(90, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| render(app, frame)).unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>()
}

#[test]
fn the_form_shows_the_validation_error() {
    let mut app = App::new(None);
    let mut state = AccountEditState::new();
    state.name = "dovecot".to_string();
    // No SMTP host: saving must say so rather than doing nothing.
    state.error = Some("SMTP host is required.".to_string());
    app.account_edit_state = Some(state);

    let screen = drawn(&app);
    assert!(
        screen.contains("SMTP host is required."),
        "the error is visible in the form: {screen}"
    );
}
