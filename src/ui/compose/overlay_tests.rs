//! Buffer tests for the compose overlays.

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::overlays::{render_attach_prompt, render_discard_confirm, render_error};

fn drawn(render: impl FnOnce(&mut ratatui::Frame)) -> String {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(render).unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>()
}

#[test]
fn the_discard_prompt_says_what_to_press() {
    let screen = drawn(|frame| render_discard_confirm(frame, frame.area()));
    assert!(screen.contains("Discard message?"), "{screen}");
    assert!(
        screen.contains("discard") && screen.contains("keep"),
        "the prompt names both answers: {screen}"
    );
}

#[test]
fn the_send_failure_dialog_shows_the_reason() {
    let screen = drawn(|frame| {
        render_error("550 mailbox unavailable", frame, frame.area());
    });
    assert!(screen.contains("Send failed"), "{screen}");
    assert!(
        screen.contains("550 mailbox unavailable"),
        "the server's reason is shown: {screen}"
    );
}

#[test]
fn the_attach_prompt_shows_what_is_typed() {
    let screen = drawn(|frame| render_attach_prompt("/tmp/report.pdf", frame, frame.area()));
    assert!(screen.contains("Attach file"), "{screen}");
    assert!(screen.contains("/tmp/report.pdf"), "{screen}");
}
