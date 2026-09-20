use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use super::util::centered_rect;
use crate::app::App;
use crate::theme::theme;

pub fn render(app: &App, frame: &mut Frame) {
    let t = theme();
    let area = centered_rect(70, 80, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Keybindings ")
        .title_style(t.popup_title())
        .borders(Borders::ALL)
        .border_style(t.border_focused())
        .style(t.popup());

    let help_text = vec![
        Line::from(Span::styled(
            "GLOBAL",
            t.accent_style()
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )),
        binding("Ctrl+c / Ctrl+q", "Quit"),
        binding("Ctrl+a", "Switch account"),
        binding("Ctrl+r", "Refresh"),
        binding("c", "Compose new message"),
        binding("?", "Toggle this help"),
        Line::from(""),
        Line::from(Span::styled(
            "ENVELOPE LIST",
            t.accent_style()
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )),
        binding("j / \u{2193}", "Move down"),
        binding("k / \u{2191}", "Move up"),
        binding("g", "Jump to top"),
        binding("G", "Jump to bottom"),
        binding("Enter", "Read message"),
        binding("d", "Delete message"),
        binding("m", "Move to folder"),
        binding("!", "Toggle flagged"),
        binding("/", "Search"),
        binding("t", "Toggle threaded view"),
        binding("n / p", "Next / previous page"),
        binding("Tab", "Focus folder sidebar"),
        Line::from(""),
        Line::from(Span::styled(
            "MESSAGE VIEW",
            t.accent_style()
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )),
        binding("j / \u{2193}", "Scroll down"),
        binding("k / \u{2191}", "Scroll up"),
        binding("Space", "Page down"),
        binding("g", "Scroll to top"),
        binding("G", "Scroll to bottom"),
        binding("r", "Reply"),
        binding("R", "Reply all"),
        binding("f", "Forward"),
        binding("d", "Delete"),
        binding("a", "Download attachments"),
        binding("q / Esc", "Back to list"),
        Line::from(""),
        Line::from(Span::styled(
            "FOLDER SIDEBAR",
            t.accent_style()
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )),
        binding("j / k", "Navigate"),
        binding("Enter", "Select folder"),
        binding("Tab", "Focus envelope list"),
        Line::from(""),
        Line::from(Span::styled(
            "SEARCH",
            t.accent_style()
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )),
        binding("Enter", "Execute search"),
        binding("Esc", "Cancel"),
        Line::from(""),
        Line::from(Span::styled("Query examples:", t.dimmed())),
        Line::from(Span::styled("  subject foo and from bar", t.normal())),
        Line::from(Span::styled("  order by date desc", t.normal())),
        Line::from(Span::styled("  before 2026-01-01", t.normal())),
        Line::from(""),
        Line::from(Span::styled(
            "COMPOSE",
            t.accent_style()
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )),
        binding("c", "New message (from envelope list)"),
        binding("r / R", "Reply / Reply all (from message view)"),
        binding("f", "Forward (from message view)"),
        binding("Tab / Shift+Tab", "Next / previous header field"),
        binding("Esc", "Jump to body (from header fields)"),
        binding("Ctrl+p", "Send message"),
        binding("Ctrl+q", "Discard message"),
        Line::from(""),
        Line::from(Span::styled(
            "ADDRESS BOOK",
            t.accent_style()
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )),
        binding("Ctrl+b", "Open address book"),
        binding("j / k", "Navigate contacts"),
        binding("n", "New contact"),
        binding("e", "Edit selected contact"),
        binding("d", "Delete selected contact"),
        binding("/", "Search contacts"),
        binding("q / Esc", "Close address book"),
        Line::from(""),
        Line::from(Span::styled(
            "CONTACT EDIT FORM",
            t.accent_style()
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )),
        binding("Tab / Shift+Tab", "Next / previous field"),
        binding("Ctrl+p", "Save contact"),
        binding("Esc", "Cancel"),
        Line::from(""),
        Line::from(Span::styled(
            "IDENTITIES",
            t.accent_style()
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )),
        binding("Shift+I", "Open identity manager (from envelope list)"),
        binding("j / k", "Navigate identities"),
        binding("n", "New identity"),
        binding("e / Enter", "Edit selected identity"),
        binding("d", "Delete selected identity"),
        binding("s", "Set selected as default"),
        binding("q / Esc", "Close identity manager"),
        Line::from(""),
        Line::from(Span::styled(
            "IDENTITY EDIT FORM",
            t.accent_style()
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )),
        binding("Tab / Shift+Tab", "Next / previous field"),
        binding("Space / Enter", "Toggle default checkbox"),
        binding("Ctrl+p", "Save identity"),
        binding("Esc", "Cancel"),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.help_scroll, 0));

    frame.render_widget(paragraph, area);
}

fn binding<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    let t = theme();
    Line::from(vec![
        Span::styled(format!("  {key:<18}"), t.header_label()),
        Span::styled(desc, t.normal()),
    ])
}
