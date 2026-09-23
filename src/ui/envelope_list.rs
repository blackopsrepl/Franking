use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Row, Table};

use crate::app::App;
use crate::keys::View;
use chrono::Local;
use std::fmt::Write as _;

use crate::theme::theme;

mod dates;
mod threading;

use dates::relative_date;
use threading::thread_depths;

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let t = theme();
    let focused = app.view == View::EnvelopeList;

    let border_style = if focused {
        t.border_focused()
    } else {
        t.border()
    };

    let thread_indicator = if app.threaded { " \u{2637}" } else { "" }; // ☷ trigram
    let title = if let Some(marker) = app.followup_lane {
        format!(" {} · {} ", app.current_folder, marker.label())
    } else if let Some(filter) = app.stage_filter.as_ref() {
        format!(" {} \u{00b7} stage: {filter} ", app.current_folder)
    } else if let Some(filter) = app.collection_filter.as_ref() {
        format!(" {} \u{00b7} collection: {filter} ", app.current_folder)
    } else if let Some(lane) = app.triage_lane {
        let cover = if app.covered_count > 0 {
            format!(" · {} covered (V)", app.covered_count)
        } else {
            String::new()
        };
        format!(" {} · {lane:?} · p{}{cover} ", app.current_folder, app.page)
    } else if let Some(ref q) = app.active_query {
        format!(
            " {}{} \u{2014} search: {q} ",
            app.current_folder, thread_indicator
        )
    } else if app.threaded {
        format!(" {}{} ", app.current_folder, thread_indicator)
    } else {
        format!(" {} \u{2014} p{} ", app.current_folder, app.page)
    };
    let title = if app.selected.is_empty() {
        title
    } else {
        format!("{}{} selected ", title.trim_end(), app.selected.len())
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

    let depths = if app.threaded {
        thread_depths(&app.envelopes)
    } else {
        Vec::new()
    };
    let root_keys = if app.threaded {
        app.thread_root_keys()
    } else {
        Vec::new()
    };

    let rows: Vec<Row> = app
        .envelopes
        .iter()
        .enumerate()
        .map(|(index, env)| {
            let quiet_lane = matches!(
                app.triage_lane,
                Some(
                    crate::db::sender_routes::Route::Reading
                        | crate::db::sender_routes::Route::Receipts
                )
            );
            let muted = app.muted_ids.contains(&env.id);
            let loud = app.loud_ids.contains(&env.id);
            let resurfaced = app.resurfaced_ids.contains(&env.id);
            let base_style = if muted {
                t.dimmed()
            } else if env.is_flagged() {
                t.flagged()
            } else if !env.is_seen() && !quiet_lane {
                t.unread()
            } else {
                t.normal()
            };

            let marker = if app.selected.contains(&env.id) {
                "\u{2713}"
            } else {
                " "
            };
            let icon = if quiet_lane && !env.is_flagged() {
                " "
            } else {
                env.flag_icon()
            };
            let flag_cell = Cell::from(format!("{marker} {icon}")).style(base_style);
            let sender = if app.is_unified_inbox() {
                format!(
                    "{} · {}",
                    env.account.as_deref().unwrap_or("?"),
                    env.sender_display()
                )
            } else {
                env.sender_display()
            };
            let from_cell = Cell::from(truncate(&sender, 24)).style(base_style);
            let depth = depths.get(index).copied().unwrap_or(0);
            // One base string, then one prefix, so a row costs few allocations.
            let base = if let Some(alias) = app.subject_aliases.get(&env.id) {
                format!("{alias}  ({})", env.subject)
            } else if depth > 0 {
                format!("{}\u{21b3} {}", "  ".repeat(depth), env.subject)
            } else {
                env.subject.clone()
            };
            let mut prefix = String::new();
            let bundle_count = app.bundled_reps.get(&env.id).copied().unwrap_or(0);
            if bundle_count > 1 {
                let _ = write!(prefix, "\u{2261} {bundle_count} \u{00b7} ");
            } else if resurfaced {
                prefix.push_str("\u{25F7} resurfaced \u{00b7} ");
            } else if loud {
                prefix.push_str("loud \u{00b7} ");
            } else if muted {
                prefix.push_str("quiet \u{00b7} ");
            }
            let inbox_group = app.triage_lane == Some(crate::db::sender_routes::Route::Inbox)
                && (index == 0 || app.envelopes[index - 1].is_seen() != env.is_seen());
            if inbox_group {
                prefix.push_str(if env.is_seen() {
                    "Seen \u{00b7} "
                } else {
                    "New \u{00b7} "
                });
            }
            let collapsed = root_keys
                .get(index)
                .and_then(|key| app.collapsed_threads.get(key));
            if collapsed.is_some() {
                prefix.push('\u{25b8}');
                prefix.push(' ');
            }
            let mut subject = if prefix.is_empty() {
                base
            } else {
                prefix + &base
            };
            if let Some(hidden) = collapsed {
                let _ = write!(subject, "  (+{})", hidden.len());
            }
            let subject_cell = Cell::from(subject).style(base_style);
            let date_cell = Cell::from(relative_date(&env.date, &now)).style(t.dimmed());

            Row::new(vec![flag_cell, from_cell, subject_cell, date_cell])
        })
        .collect();

    // Column widths: select+flag(3), from(24), subject(fill), date(14)
    let widths = [
        Constraint::Length(3),
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
pub(super) fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('\u{2026}'); // …
        out
    }
}
