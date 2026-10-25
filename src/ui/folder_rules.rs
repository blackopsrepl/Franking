use crate::app::App;
use crate::theme::theme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};

pub fn render(app: &App, frame: &mut Frame) {
    let t = theme();
    let area = frame.area();
    frame.render_widget(Clear, area);
    let layout = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .split(area);
    frame.render_widget(Paragraph::new(
        "Explicit rules move future incoming INBOX mail to server folders while Franking fetches mail.\nExisting mail and legacy triage decisions are never routed. Rules are per receiving account."
    ).style(t.normal()), layout[0]);
    let items: Vec<ListItem> = app
        .folder_rules
        .rules
        .iter()
        .map(|r| ListItem::new(format!(" {} | {} → {}", r.account, r.sender, r.target)))
        .collect();
    if items.is_empty() {
        frame.render_widget(Paragraph::new(
            "No explicit rules. Move one message and check ‘Also move future messages from sender’ to add one."
        ).block(Block::default().borders(Borders::ALL).title(" Folder rules ")),
            layout[1]);
    } else {
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Folder rules — account | sender → server folder "),
            )
            .highlight_style(t.selected())
            .highlight_symbol("▸");
        let mut state = ListState::default();
        state.select(Some(app.folder_rules.index));
        frame.render_stateful_widget(list, layout[1], &mut state);
    }
    let footer = if app.folder_rules.pending_delete.is_some() {
        "Press d again to remove this rule. Existing messages stay where they are."
    } else {
        "↑/↓ select · d remove (confirm twice) · Esc close\nIf the server resets the mailbox, recreate the rule from Move."
    };
    frame.render_widget(Paragraph::new(footer).style(t.dimmed()), layout[2]);
}
