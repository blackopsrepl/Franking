use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::keys;
use crate::theme::theme;

// Braille spinner frames.
const SPINNER: &[&str] = &[
    "\u{2801}", "\u{2809}", "\u{2819}", "\u{281b}", "\u{281e}", "\u{2836}", "\u{2834}", "\u{2824}",
];

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let persistent_hints = area.height > 1
        && matches!(
            app.view,
            keys::View::EnvelopeList | keys::View::MessageView | keys::View::FolderList
        );
    let area = if persistent_hints {
        let rows = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(area);
        frame.render_widget(
            Paragraph::new(hint_line(app.view, area.width as usize)).style(t.status_bar()),
            rows[1],
        );
        rows[0]
    } else {
        area
    };

    // A status message owns the bar: the hint list is longer than most
    // terminals, so appending it would push the message off-screen.
    if app.view != keys::View::Help && (!app.status_message.is_empty() || app.loading) {
        let mut spans: Vec<Span> = Vec::new();
        if !app.status_message.is_empty() {
            let style = if app.status_is_error {
                t.error()
            } else {
                t.accent_style()
            };
            spans.push(Span::styled(format!(" {} ", app.status_message), style));
        }
        if app.loading {
            let idx = (app.tick_count as usize) % SPINNER.len();
            spans.push(Span::styled(format!(" {} ", SPINNER[idx]), t.spinner()));
        }
        let help_available = !persistent_hints
            && matches!(
                app.view,
                keys::View::EnvelopeList
                    | keys::View::MessageView
                    | keys::View::FolderList
                    | keys::View::Help
            )
            && area.width >= 19;
        let (message_area, help_area) = if help_available {
            let areas =
                Layout::horizontal([Constraint::Fill(1), Constraint::Length(19)]).split(area);
            (areas[0], Some(areas[1]))
        } else {
            (area, None)
        };
        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(t.status_bar()),
            message_area,
        );
        if let Some(help_area) = help_area {
            frame.render_widget(
                Paragraph::new(" : Actions  ? Help ").style(t.status_key()),
                help_area,
            );
        }
        return;
    }

    let paragraph = Paragraph::new(hint_line(app.view, area.width as usize)).style(t.status_bar());
    frame.render_widget(paragraph, area);
}

/// The footer teaches the current task, not the entire command catalogue.
fn hint_line(view: keys::View, width: usize) -> Line<'static> {
    use unicode_width::UnicodeWidthStr;
    let t = theme();
    let main = matches!(
        view,
        keys::View::EnvelopeList | keys::View::MessageView | keys::View::FolderList
    );
    let suffix = if main {
        if width >= 19 {
            " : Actions  ? Help "
        } else if width >= 11 {
            " : Actions "
        } else if width >= 2 {
            " :"
        } else {
            ":"
        }
    } else {
        ""
    };
    let mut used = 0;
    let mut spans = Vec::new();
    for (key, description) in keys::hints(view) {
        if main && matches!(key, ":" | "?") {
            continue;
        }
        let text = format!(" {key} {description} ");
        if used + text.width() + suffix.width() > width {
            break;
        }
        used += text.width();
        spans.push(Span::styled(format!(" {key} "), t.status_key()));
        spans.push(Span::styled(format!("{description} "), t.status_desc()));
    }
    if !suffix.is_empty() && width > 0 {
        spans.push(Span::styled(suffix, t.status_key()));
    }
    Line::from(spans)
}

/// Render the header bar at the top.
pub fn render_header(app: &App, frame: &mut Frame, area: Rect) {
    let t = theme();

    let account_label = app.account_name.as_deref().unwrap_or("(no account)");

    // New mail indicator
    let mail_badge = if app.new_mail_count > 0 {
        format!(" [{}]", app.new_mail_count)
    } else {
        String::new()
    };

    let thread_indicator = if app.threaded { " \u{2637}" } else { "" };

    let title = format!(
        "  \u{f0e0}  {}{mail_badge}          {account_label}  \u{2502}  {}{thread_indicator}  ",
        crate::brand::NAME,
        app.current_folder
    );

    let header = Paragraph::new(title).style(t.header());
    frame.render_widget(header, area);
}

#[cfg(test)]
mod tests {
    use super::hint_line;
    use crate::app::App;
    use crate::keys::View;
    use ratatui::{backend::TestBackend, Terminal};
    use unicode_width::UnicodeWidthStr;

    #[test]
    fn narrow_status_never_cuts_a_shortcut_and_points_to_help() {
        for width in [8, 16, 24, 40, 80, 120] {
            let line = hint_line(View::EnvelopeList, width);
            let text = line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>();
            assert!(text.width() <= width, "{width}: {text}");
            assert!(text.contains(':'), "{width}: {text}");
        }
    }

    #[test]
    fn short_modes_show_all_hints_without_redundant_help() {
        let line = hint_line(View::Settings, 80);
        let text = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert!(text.contains("Esc close"));
        assert!(!text.contains("? Help"));
    }

    #[test]
    fn a_status_message_keeps_help_visible_at_the_right_edge() {
        let mut app = App::new(None);
        app.set_status("No messages.");
        let mut terminal = Terminal::new(TestBackend::new(40, 1)).unwrap();
        terminal
            .draw(|frame| super::render(&app, frame, frame.area()))
            .unwrap();
        let row = (0..40)
            .map(|col| terminal.backend().buffer()[(col, 0)].symbol())
            .collect::<String>();
        assert!(row.contains("No messages."));
        assert!(row.ends_with(" : Actions  ? Help "));
    }
}
