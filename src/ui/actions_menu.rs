use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};

use crate::{app::App, theme::theme};

pub fn render(app: &App, frame: &mut Frame) {
    let Some(menu) = app.actions_menu.as_ref() else {
        return;
    };
    let t = theme();
    let area = super::util::centered_rect(85, 80, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(" Actions ")
        .borders(Borders::ALL)
        .border_style(t.border_focused())
        .style(t.popup());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let parts = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(inner);
    frame.render_widget(
        Paragraph::new(format!(" Find: {}", menu.query)).style(t.normal()),
        parts[0],
    );
    let commands = menu.matches();
    if commands.is_empty() {
        frame.render_widget(
            Paragraph::new(" No matching actions").style(t.dimmed()),
            parts[1],
        );
    } else {
        let rows: Vec<ListItem> = commands
            .iter()
            .map(|command| {
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {}", command.label), t.normal()),
                    Span::styled(format!("  {}", command.shortcut), t.dimmed()),
                ]))
            })
            .collect();
        let mut state = ListState::default().with_selected(Some(menu.index));
        frame.render_stateful_widget(
            List::new(rows).highlight_style(t.selected()),
            parts[1],
            &mut state,
        );
    }
    frame.render_widget(
        Paragraph::new(" Type to filter · ↑/↓ choose · Enter run · Esc back").style(t.dimmed()),
        parts[2],
    );
}
