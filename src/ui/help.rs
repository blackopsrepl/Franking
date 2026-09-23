use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use super::help_wrap::wrap_lines;
use super::util::centered_rect;
use crate::app::App;
use crate::theme::theme;

pub fn render(app: &mut App, frame: &mut Frame) {
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
        binding("s / d (accounts)", "Set default / delete account"),
        binding(
            "a / e (accounts)",
            "Add / edit an account (password, app password, or OAuth2)",
        ),
        binding("Ctrl+r", "Refresh"),
        binding("c", "Compose new message"),
        binding("?", "Toggle this help"),
        binding("F1", "Open or close help from any view"),
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
        binding("e", "Archive message"),
        binding("m", "Move to folder"),
        binding("C", "Copy to folder"),
        binding("!", "Toggle flagged"),
        binding("N", "Toggle read / unread"),
        binding("S", "Cache the folder for offline use"),
        binding("A", "Mark the folder read"),
        binding("/", "Search"),
        binding("s", "Open saved searches"),
        binding("Space", "Select / deselect message"),
        binding("u", "Clear selection"),
        binding("z", "Undo the last delete, move, or flag"),
        binding("> / <", "Next / previous unread message"),
        binding("o", "Cycle the message-list ordering"),
        binding("r", "Mark the whole thread read"),
        binding("[ / ]", "Collapse / expand a thread"),
        binding("E", "Empty the current folder (twice to confirm)"),
        binding(
            "v",
            "Cycle Screening / Inbox / Reading / Receipts / Blocked",
        ),
        binding(
            "1-5",
            "Route selected sender: Inbox / Reading / Receipts / Blocked / Screening",
        ),
        binding("L / D", "Open Reply later / Saved"),
        binding("y / Y", "Mark reply later / save for reference"),
        binding("M", "Quiet or unquiet the selected conversation"),
        binding("+", "Always notify for the selected conversation"),
        binding(
            "b",
            "Resurface the conversation after a delay (blank clears)",
        ),
        binding(
            "x",
            "Place one message in a lane, overriding its sender's route",
        ),
        binding("i", "Write a private note on the selected message"),
        binding("%", "Rename the subject for you only (blank restores)"),
        binding("T", "Read the selected messages together in one scroll"),
        binding(
            "V",
            "Lift or replace the cover over previously seen Inbox mail",
        ),
        binding(
            "F",
            "Focus & reply: work the Reply later queue one message at a time",
        ),
        binding("O", "Open the outbox"),
        binding("P", "Open preferences"),
        binding("K", "Open key material"),
        binding("I", "Manage this account's identities"),
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
        binding("e", "Archive"),
        binding("C", "Copy to folder"),
        binding("N", "Toggle read / unread"),
        binding("Z", "Save all attachments in an archive"),
        binding("z", "Undo the last action"),
        binding("L/D", "Open Reply later / Saved (again to close)"),
        binding("y/Y", "Toggle reply later / saved on the selected message"),
        binding("M", "Quiet or unquiet the selected conversation"),
        binding("+", "Always notify for the selected conversation"),
        binding(
            "b",
            "Resurface the conversation after a delay (blank clears)",
        ),
        binding(
            "x",
            "Place one message in a lane, overriding its sender's route",
        ),
        binding("i", "Write a private note on the selected message"),
        binding("%", "Rename the subject for you only (blank restores)"),
        binding(
            "1/2/3/4/5",
            "Route sender: Inbox/Reading/Receipts/Blocked/Screening",
        ),
        binding("a", "Download attachments"),
        binding("P", "Unlock PGP with passphrase"),
        binding("T", "Trust the S/MIME signer certificate"),
        binding("o", "Open the attachment list"),
        binding("h", "Toggle all headers"),
        binding("Q", "Collapse or expand quoted lines"),
        binding("H", "Toggle the raw HTML source"),
        binding("c", "Add a calendar invitation to Planner123"),
        binding("/", "Find text in the message"),
        binding("l", "Open the link list"),
        binding("n / p", "Next / previous match"),
        binding("s", "Save the message as .eml"),
        binding("\u{002a}", "Clip an excerpt from this message"),
        binding("q / Esc", "Back to list"),
        Line::from(""),
        Line::from(Span::styled(
            "FOLDER SIDEBAR",
            t.accent_style()
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )),
        binding("j / k", "Navigate"),
        binding("Enter", "Select folder"),
        binding("n", "Create folder"),
        binding("r", "Rename folder"),
        binding("d", "Delete folder"),
        binding("F", "Manage Sieve filters"),
        binding("s", "Toggle subscription when supported"),
        binding("letters", "Jump to a matching folder"),
        binding("Tab", "Focus envelope list"),
        Line::from(""),
        Line::from(Span::styled(
            "SEARCH",
            t.accent_style()
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )),
        binding("Enter", "Execute search"),
        binding("Tab", "Cycle folder / all folders / all accounts"),
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
        binding("Ctrl+f", "Search inside the message body"),
        binding("Esc", "Jump to body (from header fields)"),
        binding("Tab to Send, Enter", "Send message"),
        binding("Ctrl+q", "Discard message"),
        Line::from(""),
        Line::from(Span::styled(
            "ADDRESS BOOK",
            t.accent_style()
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )),
        binding("Ctrl+b", "Open address book"),
        binding("Ctrl+l", "Open the cross-account attachment library"),
        binding("Ctrl+k", "Open saved text clips"),
        binding("Ctrl+n (compose)", "Insert a reusable snippet"),
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
        binding("Tab to Save, Enter", "Save contact"),
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
        binding("Tab to Save, Enter", "Save identity"),
        binding("Esc", "Cancel"),
    ];

    let inner_width = area.width.saturating_sub(2);
    let inner_height = area.height.saturating_sub(2) as usize;
    let help_text = wrap_lines(help_text, inner_width as usize);
    app.help_max_scroll = help_text
        .len()
        .saturating_sub(inner_height)
        .min(u16::MAX as usize) as u16;
    app.help_scroll = app.help_scroll.min(app.help_max_scroll);
    let paragraph = Paragraph::new(help_text)
        .block(block)
        .scroll((app.help_scroll, 0));

    frame.render_widget(paragraph, area);
}

fn binding<'a>(key: &'a str, desc: &'a str) -> Line<'a> {
    let t = theme();
    Line::from(vec![
        Span::styled(format!("  {key:<18}  "), t.header_label()),
        Span::styled(desc, t.normal()),
    ])
}
