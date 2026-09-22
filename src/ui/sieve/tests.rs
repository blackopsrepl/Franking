use ratatui::backend::TestBackend;
use ratatui::Terminal;

use crate::app::App;
use crate::keys::View;
use crate::mail::sieve::SieveScript;

use super::render;

fn app_with_scripts() -> App {
    let mut app = App::new(None);
    app.view = View::SieveScripts;
    app.sieve.scripts = vec![
        SieveScript {
            name: "main".to_string(),
            active: true,
        },
        SieveScript {
            name: "backup".to_string(),
            active: false,
        },
    ];
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
fn renders_scripts_with_active_marker() {
    let app = app_with_scripts();
    let screen = drawn(&app);
    assert!(screen.contains("main"), "screen: {screen}");
    assert!(screen.contains("backup"));
    assert!(screen.contains("active"));
    assert!(screen.contains("inactive"));
    assert!(screen.contains("2 script(s)"));
}

#[test]
fn renders_the_editor_header() {
    let mut app = app_with_scripts();
    app.view = View::SieveEdit;
    app.sieve.editor_name = "main".to_string();
    app.sieve.editor = Some(crate::compose_editor::ComposeEditor::from_text(
        "require [\"fileinto\"];\n",
    ));
    let screen = drawn(&app);
    assert!(screen.contains("editing main"));
    assert!(screen.contains("fileinto"));
}
