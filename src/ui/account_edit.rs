/* Account add/edit form UI. */

use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::account_edit::AccountField;
use crate::app::App;
use crate::theme::theme;
use crate::ui::action_bar::{render_action_bar, Button};

/// Render the account form as a centered popup overlay.
pub fn render(app: &App, frame: &mut Frame) {
    let t = theme();
    let area = frame.area();

    let Some(ref state) = app.account_edit_state else {
        return;
    };
    frame.render_widget(Clear, area);

    let popup_w = area.width.min(60);
    let popup_h = 24u16;
    let popup = Rect {
        x: area.x + area.width.saturating_sub(popup_w) / 2,
        y: area.y + area.height.saturating_sub(popup_h) / 2,
        width: popup_w,
        height: popup_h,
    };

    let block = Block::default()
        .title(Span::styled(
            if state.editing {
                " Edit Account "
            } else {
                " New Account "
            },
            t.popup_title(),
        ))
        .borders(Borders::ALL)
        .border_style(t.border_focused())
        .style(t.popup());
    frame.render_widget(block.clone(), popup);
    let inner = block.inner(popup);

    let masked = "*".repeat(state.password.chars().count());
    let fields: [(AccountField, &str, &str); 8] = [
        (AccountField::Name, "Name    ", state.name.as_str()),
        (AccountField::Username, "Login   ", state.username.as_str()),
        (AccountField::ImapHost, "IMAP    ", state.imap_host.as_str()),
        (AccountField::ImapPort, "IMAP pt ", state.imap_port.as_str()),
        (AccountField::SmtpHost, "SMTP    ", state.smtp_host.as_str()),
        (AccountField::SmtpPort, "SMTP pt ", state.smtp_port.as_str()),
        (AccountField::Password, "Password", masked.as_str()),
        (
            AccountField::Default,
            "Default ",
            if state.is_default {
                "[x] set as default"
            } else {
                "[ ] set as default"
            },
        ),
    ];

    for (i, (field, label, value)) in fields.iter().enumerate() {
        let row_y = inner.y + i as u16 * 2;
        if row_y + 1 >= inner.y + inner.height {
            break;
        }
        let focused = state.focused == *field;
        let label_style = if focused {
            t.header_label().add_modifier(Modifier::BOLD)
        } else {
            t.dimmed()
        };
        frame.render_widget(
            Paragraph::new(Span::styled(*label, label_style)),
            Rect {
                x: inner.x,
                y: row_y,
                width: inner.width,
                height: 1,
            },
        );
        let input_style = if focused {
            t.search_input()
        } else {
            t.normal()
        };
        frame.render_widget(
            Paragraph::new(Span::styled(*value, input_style)),
            Rect {
                x: inner.x,
                y: row_y + 1,
                width: inner.width,
                height: 1,
            },
        );
    }

    let hint_height = 1u16;
    frame.render_widget(
        Paragraph::new(Span::styled(
            "Ctrl+D detects settings from the login address · Tab moves · Enter saves",
            t.dimmed(),
        )),
        Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(3),
            width: inner.width,
            height: hint_height,
        },
    );

    let error_height = if state.error.is_some() { 1 } else { 0 };
    if let Some(ref error) = state.error {
        frame.render_widget(
            Paragraph::new(Span::styled(error.clone(), t.error())),
            Rect {
                x: inner.x,
                y: inner.y + inner.height.saturating_sub(2),
                width: inner.width,
                height: 1,
            },
        );
    }

    let bar = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(1 + error_height),
        width: inner.width,
        height: 1,
    };
    let buttons = [
        Button {
            label: "Save",
            focused: state.focused == AccountField::Save,
            disabled: false,
        },
        Button {
            label: "Cancel",
            focused: state.focused == AccountField::Cancel,
            disabled: false,
        },
    ];
    let focused_idx = match state.focused {
        AccountField::Save => Some(0usize),
        AccountField::Cancel => Some(1),
        _ => None,
    };
    render_action_bar(
        frame,
        bar,
        crate::keys::EditMode::Insert,
        &buttons,
        focused_idx,
    );
}
