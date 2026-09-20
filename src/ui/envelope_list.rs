use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Row, Table};

use crate::app::App;
use crate::keys::View;
use crate::theme::theme;

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let focused = app.view == View::EnvelopeList;

    let border_style = if focused {
        t.border_focused()
    } else {
        t.border()
    };

    let thread_indicator = if app.threaded { " \u{2637}" } else { "" }; // ☷ trigram
    let title = if let Some(ref q) = app.active_query {
        format!(
            " {}{} \u{2014} search: {q} ",
            app.current_folder, thread_indicator
        )
    } else if app.threaded {
        format!(" {}{} ", app.current_folder, thread_indicator)
    } else {
        format!(" {} \u{2014} p{} ", app.current_folder, app.page)
    };

    let block = Block::default()
        .title(title)
        .title_style(if focused {
            t.accent_style().add_modifier(Modifier::BOLD)
        } else {
            t.dimmed()
        })
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.envelopes.is_empty() {
        let empty = Table::new(Vec::<Row>::new(), &[Constraint::Fill(1)]).block(block);
        frame.render_widget(empty, area);
        return;
    }

    let header_cells = ["", "From", "Subject", "Date"]
        .iter()
        .map(|h| Cell::from(*h).style(t.dimmed().add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1);

    let now = Local::now();

    let rows: Vec<Row> = app
        .envelopes
        .iter()
        .map(|env| {
            let base_style = if env.is_flagged() {
                t.flagged()
            } else if !env.is_seen() {
                t.unread()
            } else {
                t.normal()
            };

            let flag_cell = Cell::from(env.flag_icon()).style(base_style);
            let from_cell = Cell::from(truncate(&env.sender_display(), 24)).style(base_style);
            let subject_cell = Cell::from(env.subject.as_str()).style(base_style);
            let date_cell = Cell::from(relative_date(&env.date, &now)).style(t.dimmed());

            Row::new(vec![flag_cell, from_cell, subject_cell, date_cell])
        })
        .collect();

    // Column widths: flag(2), from(24), subject(fill), date(14)
    let widths = [
        Constraint::Length(2),
        Constraint::Length(24),
        Constraint::Fill(1),
        Constraint::Length(14),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(t.selected());

    frame.render_stateful_widget(table, area, &mut app.envelope_state);
}

// Truncate a string to `max` characters, appending \u{2026} if needed.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('\u{2026}'); // …
        out
    }
}

/* Parse a backend date string into a chrono DateTime<Local>.
The native mail layer emits dates like "2026-02-14 10:30:00+00:00". */
fn parse_date(date: &str) -> Option<DateTime<Local>> {
    // Try RFC 3339 / ISO 8601 with timezone offset
    if let Ok(dt) = DateTime::parse_from_str(date, "%Y-%m-%d %H:%M:%S%:z") {
        return Some(dt.with_timezone(&Local));
    }
    // Try without timezone (assume local)
    if let Ok(naive) = NaiveDateTime::parse_from_str(date, "%Y-%m-%d %H:%M:%S") {
        return Local.from_local_datetime(&naive).single();
    }
    // Try RFC 2822 (some IMAP servers)
    if let Ok(dt) = DateTime::parse_from_rfc2822(date) {
        return Some(dt.with_timezone(&Local));
    }
    None
}

/// Format a date as a relative/contextual timestamp.
///
/// - Under 1 min:  "just now"
/// - Under 1 hour: "5m ago"
/// - Today:        "14:30"
/// - Yesterday:    "Yesterday"
/// - This week:    "Mon" / "Tue" (day name)
/// - This year:    "Feb 14"
/// - Older:        "2025-12-01"
pub fn relative_date(date_str: &str, now: &DateTime<Local>) -> String {
    let dt = match parse_date(date_str) {
        Some(dt) => dt,
        None => {
            // Unparseable: return as-is (column will truncate visually)
            return date_str.to_string();
        }
    };

    let duration = now.signed_duration_since(dt);
    let secs = duration.num_seconds();

    if secs < 0 {
        // Future date — just show time
        return dt.format("%H:%M").to_string();
    }

    if secs < 60 {
        return "just now".to_string();
    }

    if secs < 3600 {
        let mins = secs / 60;
        return format!("{mins}m ago");
    }

    let today = now.date_naive();
    let msg_date = dt.date_naive();

    if msg_date == today {
        return dt.format("%H:%M").to_string();
    }

    if msg_date == today.pred_opt().unwrap_or(today) {
        return "Yesterday".to_string();
    }

    // Within last 7 days
    let days_ago = (today - msg_date).num_days();
    if days_ago > 0 && days_ago < 7 {
        return dt.format("%a").to_string(); // Mon, Tue, etc.
    }

    // Same year
    if dt.format("%Y").to_string() == now.format("%Y").to_string() {
        return dt.format("%b %d").to_string(); // Feb 14
    }

    // Older
    dt.format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn now() -> DateTime<Local> {
        // Fixed "now" for deterministic tests
        Local.with_ymd_and_hms(2026, 2, 14, 12, 0, 0).unwrap()
    }

    #[test]
    fn just_now() {
        let n = now();
        let date = (n - Duration::seconds(30))
            .format("%Y-%m-%d %H:%M:%S%:z")
            .to_string();
        assert_eq!(relative_date(&date, &n), "just now");
    }

    #[test]
    fn minutes_ago() {
        let n = now();
        let date = (n - Duration::minutes(5))
            .format("%Y-%m-%d %H:%M:%S%:z")
            .to_string();
        assert_eq!(relative_date(&date, &n), "5m ago");
    }

    #[test]
    fn today_shows_time() {
        let n = now();
        let date = (n - Duration::hours(3))
            .format("%Y-%m-%d %H:%M:%S%:z")
            .to_string();
        assert_eq!(relative_date(&date, &n), "09:00");
    }

    #[test]
    fn yesterday() {
        let n = now();
        let date = (n - Duration::hours(30))
            .format("%Y-%m-%d %H:%M:%S%:z")
            .to_string();
        assert_eq!(relative_date(&date, &n), "Yesterday");
    }

    #[test]
    fn this_week_shows_day_name() {
        let n = now();
        let date = (n - Duration::days(3))
            .format("%Y-%m-%d %H:%M:%S%:z")
            .to_string();
        let result = relative_date(&date, &n);
        // Should be a 3-letter day abbreviation
        assert!(
            ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].contains(&result.as_str()),
            "got: {result}"
        );
    }

    #[test]
    fn same_year_shows_month_day() {
        let n = now();
        let date = (n - Duration::days(30))
            .format("%Y-%m-%d %H:%M:%S%:z")
            .to_string();
        let result = relative_date(&date, &n);
        assert!(result.starts_with("Jan"), "got: {result}");
    }

    #[test]
    fn older_year_shows_full_date() {
        let n = now();
        let old = Local.with_ymd_and_hms(2025, 6, 15, 10, 0, 0).unwrap();
        let date = old.format("%Y-%m-%d %H:%M:%S%:z").to_string();
        assert_eq!(relative_date(&date, &n), "2025-06-15");
    }

    #[test]
    fn unparseable_fallback() {
        let n = now();
        assert_eq!(relative_date("not a date at all", &n), "not a date at all");
    }

    #[test]
    fn truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string() {
        assert_eq!(truncate("hello world", 6), "hello\u{2026}");
    }
}
