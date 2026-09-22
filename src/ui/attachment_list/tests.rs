use ratatui::backend::TestBackend;
use ratatui::Terminal;

use crate::app::App;

use super::render;

fn app_with_attachment() -> App {
    let raw = b"MIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=\"m\"\r\n\r\n--m\r\nContent-Type: text/plain\r\n\r\nhello\r\n--m\r\nContent-Type: application/pdf; name=\"report.pdf\"\r\nContent-Disposition: attachment; filename=\"report.pdf\"\r\nContent-Transfer-Encoding: base64\r\n\r\nAAECAwQ=\r\n--m--\r\n";
    let mut app = App::new(None);
    app.message_content = Some(crate::mail::mime::parse_message(raw).unwrap());
    app
}

fn drawn(app: &App) -> String {
    let backend = TestBackend::new(100, 24);
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
fn renders_attachment_name_and_hints() {
    let app = app_with_attachment();
    let screen = drawn(&app);
    assert!(screen.contains("report.pdf"), "screen: {screen}");
    assert!(screen.contains("application/pdf"));
    assert!(screen.contains("Enter open"));
    assert!(screen.contains("s save"));
}

#[test]
fn renders_with_no_attachments_without_panicking() {
    let app = App::new(None);
    let screen = drawn(&app);
    assert!(screen.contains("Attachments"));
}
