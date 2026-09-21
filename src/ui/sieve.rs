use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::app::App;
use crate::keys::hints;
use crate::theme::theme;

/// Render the Sieve script browser or the script editor.
pub fn render(app: &App, frame: &mut Frame) {
    match app.view {
        crate::keys::View::SieveEdit => render_editor(app, frame),
        _ => {
            render_scripts(app, frame);
            if app.view == crate::keys::View::SieveName {
                let area = frame.area();
                render_name_prompt(
                    app,
                    frame,
                    Rect {
                        x: area.x,
                        y: area.y + area.height.saturating_sub(1),
                        width: area.width,
                        height: 1,
                    },
                );
            }
        }
    }
}

fn render_scripts(app: &App, frame: &mut Frame) {
    let t = theme();
    let area = frame.area();
    let outer = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .split(area);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  SolverForge Mail", t.header()),
            Span::styled("  ·  Sieve filters", t.dimmed()),
        ]))
        .style(t.status_bar()),
        outer[0],
    );

    let items: Vec<ListItem> = app
        .sieve
        .scripts
        .iter()
        .enumerate()
        .map(|(index, script)| {
            let marker = if script.active {
                "● active  "
            } else {
                "○ inactive"
            };
            let pending = app.sieve.pending_delete.as_deref() == Some(script.name.as_str());
            let suffix = if pending {
                "  · press d to confirm"
            } else {
                ""
            };
            let style = if script.active {
                t.unread()
            } else {
                t.normal()
            };
            ListItem::new(Line::from(Span::styled(
                format!(
                    " {marker}  {} {}{suffix} ",
                    if index == app.sieve.index { "▸" } else { " " },
                    script.name
                ),
                style,
            )))
        })
        .collect();

    let title = if app.sieve.scripts.is_empty() {
        " Sieve filters · none yet (n creates one) ".to_string()
    } else {
        format!(" Sieve filters · {} script(s) ", app.sieve.scripts.len())
    };
    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(title, t.popup_title()))
                .borders(Borders::ALL)
                .border_style(t.border_focused()),
        )
        .highlight_style(t.selected());

    let mut state = ListState::default();
    state.select(Some(app.sieve.index));
    frame.render_stateful_widget(list, outer[1], &mut state);

    render_status(app, frame, outer[2]);
}

fn render_editor(app: &App, frame: &mut Frame) {
    let t = theme();
    let area = frame.area();
    let outer = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .split(area);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  SolverForge Mail", t.header()),
            Span::styled(
                format!("  ·  editing {}", app.sieve.editor_name),
                t.dimmed(),
            ),
        ]))
        .style(t.status_bar()),
        outer[0],
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(t.border_focused())
        .title(Span::styled(" Sieve script ", t.popup_title()));
    let inner = block.inner(outer[1]);
    frame.render_widget(block, outer[1]);
    if let Some(editor) = app.sieve.editor.as_ref() {
        frame.render_widget(editor.textarea(), inner);
    }
    render_status(app, frame, outer[2]);
}

fn render_status(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    if app.status_message.is_empty() {
        let pairs = hints(app.view);
        let mut spans = Vec::new();
        for (key, desc) in pairs {
            spans.push(Span::styled(format!(" {key} "), t.status_key()));
            spans.push(Span::styled(desc, t.status_desc()));
        }
        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(t.status_bar()),
            area,
        );
    } else {
        let style = if app.status_is_error {
            t.error()
        } else {
            t.status_bar()
        };
        frame.render_widget(
            Paragraph::new(format!(" {} ", app.status_message)).style(style),
            area,
        );
    }
}

/// Render the new-script name prompt in the status bar.
pub fn render_name_prompt(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let cursor_char = if app.tick_count % 4 < 2 {
        "\u{2588}"
    } else {
        " "
    };
    let spans = vec![
        Span::styled(" Script name: ", t.status_key()),
        Span::styled(format!("{}{cursor_char}", app.sieve.name), t.search_input()),
        Span::styled("  (Enter to create, Esc to cancel)", t.dimmed()),
    ];
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(t.status_bar()),
        area,
    );
}

#[cfg(test)]
mod tests;
