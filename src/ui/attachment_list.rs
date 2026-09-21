use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};

use crate::app::App;
use crate::theme::theme;
use crate::ui::util::centered_rect;

/// Render the attachment list overlay for the current message.
pub fn render(app: &App, frame: &mut Frame) {
    let t = theme();
    let area = frame.area();
    let popup = centered_rect(72, 60, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(Span::styled(
            " Attachments · Enter open · s save · Esc close ",
            t.popup_title(),
        ))
        .borders(Borders::ALL)
        .border_style(t.border_focused())
        .style(t.popup());

    let items: Vec<ListItem> = app
        .message_content
        .as_ref()
        .map(|message| {
            message
                .attachments
                .iter()
                .map(|attachment| {
                    let name = attachment
                        .file_name
                        .clone()
                        .unwrap_or_else(|| "(unnamed)".to_string());
                    let content_type = attachment
                        .content_type
                        .clone()
                        .unwrap_or_else(|| "application/octet-stream".to_string());
                    let disposition = if attachment.is_inline {
                        "inline"
                    } else {
                        "attachment"
                    };
                    ListItem::new(Line::from(Span::styled(
                        format!(
                            " {name}  ·  {content_type}  ·  {disposition}  ·  {} bytes ",
                            attachment.size
                        ),
                        t.normal(),
                    )))
                })
                .collect()
        })
        .unwrap_or_default();

    let list = List::new(items)
        .block(block)
        .highlight_style(t.selected())
        .highlight_symbol("▸");

    let mut state = ListState::default();
    state.select(Some(app.attachment_index));
    frame.render_stateful_widget(list, popup, &mut state);
}

#[cfg(test)]
mod tests;

/// Render the text attachment preview overlay.
pub fn render_preview(app: &App, frame: &mut Frame) {
    let t = theme();
    let popup = centered_rect(90, 80, frame.area());
    frame.render_widget(Clear, popup);

    let (name, text) = match app.attachment_preview.as_ref() {
        Some(preview) => preview,
        None => return,
    };
    let block = Block::default()
        .title(Span::styled(
            format!(" {name} · Esc closes "),
            t.popup_title(),
        ))
        .borders(Borders::ALL)
        .border_style(t.border_focused())
        .style(t.popup());
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let paragraph = Paragraph::new(text.as_str())
        .style(t.normal())
        .wrap(Wrap { trim: false })
        .scroll((app.preview_scroll, 0));
    frame.render_widget(paragraph, inner);
}
