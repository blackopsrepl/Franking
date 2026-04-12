use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::App;
use crate::mail::MessageDisplayMode;
use crate::theme::theme;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let current_mode = app.resolved_message_display_mode();

    let block = Block::default()
        .title(format!(" Message [{}] ", display_mode_label(current_mode)))
        .title_style(t.accent_style().add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(t.border_focused());

    let Some(message) = app.current_message() else {
        let paragraph = Paragraph::new("No message loaded.")
            .block(block)
            .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
        return;
    };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(mode_hint_line(app, current_mode));
    lines.push(Line::from(""));

    for header in &message.headers {
        lines.push(Line::from(vec![
            Span::styled(format!("{}: ", header.name), t.header_label()),
            Span::styled(header.value.clone(), t.header_value()),
        ]));
    }

    lines.push(Line::from(Span::styled(
        "\u{2500}".repeat(area.width.saturating_sub(4) as usize),
        t.dimmed(),
    )));

    for raw_line in app
        .render_message_body(area.width.saturating_sub(4) as usize)
        .lines()
    {
        let style = if raw_line.starts_with('>') {
            t.dimmed()
        } else {
            t.normal()
        };
        lines.push(Line::from(Span::styled(raw_line.to_string(), style)));
    }

    if !message.attachments.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Attachments", t.header_label())));
        for attachment in &message.attachments {
            let name = attachment
                .file_name
                .clone()
                .unwrap_or_else(|| "(unnamed)".to_string());
            let content_type = attachment
                .content_type
                .clone()
                .unwrap_or_else(|| "application/octet-stream".to_string());
            let inline = if attachment.is_inline {
                "inline"
            } else {
                "attachment"
            };
            lines.push(Line::from(Span::styled(
                format!(
                    "  {name} [{content_type}, {inline}, {} bytes]",
                    attachment.size
                ),
                t.normal(),
            )));
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.message_scroll, 0));

    frame.render_widget(paragraph, area);
}

fn mode_hint_line(app: &App, current_mode: MessageDisplayMode) -> Line<'static> {
    let t = theme();
    let message = app.current_message().expect("message must exist");
    let mut spans = vec![Span::styled("Modes: ", t.header_label())];

    spans.extend(mode_span(
        "1 Auto",
        current_mode == MessageDisplayMode::Auto,
        true,
    ));
    spans.push(Span::raw("  "));
    spans.extend(mode_span(
        "2 Plain",
        current_mode == MessageDisplayMode::Plain,
        message.has_plain_body(),
    ));
    spans.push(Span::raw("  "));
    spans.extend(mode_span(
        "3 HTML",
        current_mode == MessageDisplayMode::Html,
        message.has_html_body(),
    ));

    if message.has_html_body() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled("[o] Open externally", t.header_value()));
    }

    Line::from(spans)
}

fn mode_span(label: &str, selected: bool, available: bool) -> Vec<Span<'static>> {
    let t = theme();
    let style = if !available {
        t.dimmed()
    } else if selected {
        t.accent_style().add_modifier(Modifier::BOLD)
    } else {
        t.header_value()
    };
    vec![Span::styled(label.to_string(), style)]
}

fn display_mode_label(mode: MessageDisplayMode) -> &'static str {
    match mode {
        MessageDisplayMode::Auto => "Auto",
        MessageDisplayMode::Plain => "Plain",
        MessageDisplayMode::Html => "HTML",
    }
}
