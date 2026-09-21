use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::app::App;
use crate::file_picker::FilePickerState;
use crate::theme::theme;

/// Render the attachment path picker as a full-screen list.
pub fn render(app: &App, frame: &mut Frame) {
    let t = theme();
    let area = frame.area();
    let outer = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .split(area);

    let Some(picker) = app.file_picker.as_ref() else {
        return;
    };

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  Attach a file", t.header()),
            Span::styled(format!("  ·  {}", picker.dir.display()), t.dimmed()),
        ]))
        .style(t.status_bar()),
        outer[0],
    );

    let items: Vec<ListItem> = picker
        .entries
        .iter()
        .map(|path| {
            ListItem::new(Line::from(Span::styled(
                FilePickerState::label(path),
                t.normal(),
            )))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(" Files ", t.popup_title()))
                .borders(Borders::ALL)
                .border_style(t.border_focused()),
        )
        .highlight_style(t.selected())
        .highlight_symbol("▸");
    let mut state = ListState::default();
    state.select(Some(picker.index));
    frame.render_stateful_widget(list, outer[1], &mut state);

    let footer = match picker.error.as_deref() {
        Some(error) => Span::styled(format!(" {error} "), t.error()),
        None => Span::styled(
            " Enter open/attach · Backspace up · j/k move · Esc cancel ",
            t.status_bar(),
        ),
    };
    frame.render_widget(
        Paragraph::new(Line::from(footer)).style(t.status_bar()),
        outer[2],
    );
}
