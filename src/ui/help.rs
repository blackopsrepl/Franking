use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use super::{help_wrap::wrap_lines, util::centered_rect};
use crate::{
    app::App,
    keys::{self, View},
    theme::theme,
};

pub fn render(app: &mut App, frame: &mut Frame) {
    let t = theme();
    let area = centered_rect(85, 85, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(" Help ")
        .title_style(t.popup_title())
        .borders(Borders::ALL)
        .border_style(t.border_focused())
        .style(t.popup());
    let origin = app.previous_view.unwrap_or(View::EnvelopeList);
    let mut lines = vec![
        Line::from(Span::styled(" Everyday mail ", t.accent_style())),
        Line::from(" Inbox shows delivered mail. Folders are server mailboxes."),
        Line::from(" Reply later is a local checklist. Starred shows cached flagged mail."),
        Line::from(" Tab focuses the sidebar; arrows or j/k navigate; Enter opens."),
        Line::from(" : opens searchable Actions. Type a name, then Enter to run it."),
        binding("Ctrl+a / Ctrl+r", "Switch account / refresh (mail views)"),
        binding("Ctrl+q / Ctrl+c", "Quit; in compose, request discard"),
        binding("F1", "Help from any view"),
        Line::from(""),
    ];
    if matches!(
        origin,
        View::EnvelopeList | View::MessageView | View::FolderList
    ) {
        lines.push(Line::from(Span::styled(
            format!(" {:?} shortcuts", origin),
            t.accent_style(),
        )));
        lines.push(binding("j/k or ↑/↓", "Navigate / scroll"));
        if origin != View::FolderList {
            lines.push(binding("g/G or Home/End", "Start / end"));
        }
        for command in keys::commands::commands(origin) {
            if command.key.is_some() {
                lines.push(binding(command.shortcut, command.label));
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(
            " Secondary commands live in Actions, not extra letter bindings.",
        ));
    } else {
        lines.push(Line::from(Span::styled(
            format!(" {:?}", origin),
            t.accent_style(),
        )));
        for (key, label) in keys::hints(origin) {
            lines.push(binding(key, label));
        }
    }
    if origin == View::Compose {
        lines.extend([
            binding("Tab / Shift+Tab", "Next / previous field or action"),
            binding(
                "Enter on action",
                "Activate Send, Attach, Draft, or Schedule",
            ),
            binding("Ctrl+n", "Insert a reusable snippet"),
            binding("Ctrl+f in body", "Find text"),
        ]);
    }
    let lines = wrap_lines(lines, area.width.saturating_sub(2) as usize);
    app.help_max_scroll = lines
        .len()
        .saturating_sub(area.height.saturating_sub(2) as usize)
        .min(u16::MAX as usize) as u16;
    app.help_scroll = app.help_scroll.min(app.help_max_scroll);
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .scroll((app.help_scroll, 0)),
        area,
    );
}

fn binding(key: &str, desc: &str) -> Line<'static> {
    let t = theme();
    Line::from(vec![
        Span::styled(format!("  {key:<20} "), t.header_label()),
        Span::styled(desc.to_owned(), t.normal()),
    ])
}
