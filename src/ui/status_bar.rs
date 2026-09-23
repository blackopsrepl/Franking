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
        let help_available = matches!(
            app.view,
            keys::View::EnvelopeList
                | keys::View::MessageView
                | keys::View::FolderList
                | keys::View::Help
        ) && area.width >= 9;
        let (message_area, help_area) = if help_available {
            let areas =
                Layout::horizontal([Constraint::Fill(1), Constraint::Length(8)]).split(area);
            (areas[0], Some(areas[1]))
        } else {
            (area, None)
        };
        frame.render_widget(
            Paragraph::new(Line::from(spans)).style(t.status_bar()),
            message_area,
        );
        if let Some(help_area) = help_area {
            frame.render_widget(Paragraph::new(" ? Help ").style(t.status_key()), help_area);
        }
        return;
    }

    let paragraph = Paragraph::new(hint_line(app.view, area.width as usize)).style(t.status_bar());
    frame.render_widget(paragraph, area);
}

/// Fit whole shortcuts only; keep the route to complete help visible when
/// the terminal cannot show every binding in a single row.
fn hint_line(view: keys::View, width: usize) -> Line<'static> {
    use unicode_width::UnicodeWidthStr;

    let t = theme();
    let hints = keys::hints(view);
    let help = if width >= 8 { "  ? Help" } else { "?" };
    let reserve = help.width().min(width);
    let mut used = 0;
    let mut shown = 0;
    let mut spans = Vec::new();
    for &(key, description) in hints {
        let key_span = format!(" {key} ");
        let needed = key_span.width() + description.width() + if shown > 0 { 2 } else { 0 };
        if used + needed + reserve > width {
            break;
        }
        if shown > 0 {
            spans.push(Span::styled("  ", t.status_bar()));
        }
        spans.push(Span::styled(key_span, t.status_key()));
        spans.push(Span::styled(description, t.status_desc()));
        used += needed;
        shown += 1;
    }
    if shown < hints.len() {
        let remaining = format!(" +{}", hints.len() - shown);
        if used + remaining.width() + reserve <= width {
            spans.push(Span::styled(remaining, t.dimmed()));
        }
        if reserve > 0 {
            spans.push(Span::styled(help, t.status_key()));
        }
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
            assert!(text.contains('?'), "{width}: {text}");
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
        assert!(row.ends_with(" ? Help "));
    }
}
