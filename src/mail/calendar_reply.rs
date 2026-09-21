/*! Building iTIP replies (RFC 5546) for calendar invitations. */

use chrono::Utc;

use super::calendar::Event;

/// Participation status for a reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartStat {
    Accepted,
    Tentative,
    Declined,
}

impl PartStat {
    pub fn token(self) -> &'static str {
        match self {
            PartStat::Accepted => "ACCEPTED",
            PartStat::Tentative => "TENTATIVE",
            PartStat::Declined => "DECLINED",
        }
    }

    pub fn verb(self) -> &'static str {
        match self {
            PartStat::Accepted => "Accepted",
            PartStat::Tentative => "Tentatively accepted",
            PartStat::Declined => "Declined",
        }
    }
}

/// Fold a line at 75 octets as RFC 5545 requires.
fn fold(line: &str) -> String {
    const LIMIT: usize = 73;
    if line.len() <= LIMIT {
        return line.to_string();
    }
    let mut out = String::new();
    let mut rest = line;
    let mut first = true;
    while !rest.is_empty() {
        let take = rest
            .char_indices()
            .take_while(|(index, _)| *index < LIMIT)
            .last()
            .map(|(index, ch)| index + ch.len_utf8())
            .unwrap_or(rest.len());
        if !first {
            out.push_str("\r\n ");
        }
        out.push_str(&rest[..take]);
        rest = &rest[take..];
        first = false;
    }
    out
}

/// A `METHOD:REPLY` iCalendar payload for one attendee and status.
pub fn build_reply(event: &Event, partstat: PartStat, attendee: &str) -> Option<String> {
    let uid = event.uid.as_deref()?;
    let organizer = event.organizer.as_deref()?;

    let mut lines = vec![
        "BEGIN:VCALENDAR".to_string(),
        "VERSION:2.0".to_string(),
        "PRODID:-//SolverForge Mail//EN".to_string(),
        "METHOD:REPLY".to_string(),
        "BEGIN:VEVENT".to_string(),
        format!("UID:{uid}"),
        format!(
            "DTSTAMP:{}",
            Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
        ),
    ];
    if let Some(start) = event.start.as_deref() {
        lines.push(format!("DTSTART:{start}"));
    }
    if let Some(end) = event.end.as_deref() {
        lines.push(format!("DTEND:{end}"));
    }
    lines.push(format!("ORGANIZER:mailto:{organizer}"));
    lines.push(format!(
        "ATTENDEE;PARTSTAT={};ROLE=REQ-PARTICIPANT:mailto:{attendee}",
        partstat.token()
    ));
    lines.push("END:VEVENT".to_string());
    lines.push("END:VCALENDAR".to_string());

    Some(
        lines
            .iter()
            .map(|line| fold(line))
            .collect::<Vec<_>>()
            .join("\r\n")
            + "\r\n",
    )
}

#[cfg(test)]
mod tests {
    use super::{build_reply, fold, PartStat};
    use crate::mail::calendar::Event;

    fn event() -> Event {
        Event {
            uid: Some("event-42@example.com".to_string()),
            organizer: Some("alice@example.com".to_string()),
            summary: Some("Standup".to_string()),
            start: Some("20260921T090000Z".to_string()),
            end: Some("20260921T093000Z".to_string()),
            location: None,
        }
    }

    #[test]
    fn builds_a_reply_with_the_requested_status() {
        let reply = build_reply(&event(), PartStat::Accepted, "bob@example.com").expect("reply");
        assert!(reply.starts_with("BEGIN:VCALENDAR\r\n"));
        assert!(reply.contains("METHOD:REPLY"));
        assert!(reply.contains("UID:event-42@example.com"));
        assert!(reply.contains("ORGANIZER:mailto:alice@example.com"));
        assert!(reply
            .contains("ATTENDEE;PARTSTAT=ACCEPTED;ROLE=REQ-PARTICIPANT:mailto:bob@example.com"));
        assert!(reply.ends_with("END:VCALENDAR\r\n"));
    }

    #[test]
    fn declines_and_tentative_use_their_own_tokens() {
        assert!(build_reply(&event(), PartStat::Declined, "bob@example.com")
            .unwrap()
            .contains("PARTSTAT=DECLINED"));
        assert!(
            build_reply(&event(), PartStat::Tentative, "bob@example.com")
                .unwrap()
                .contains("PARTSTAT=TENTATIVE")
        );
    }

    #[test]
    fn refuses_events_without_a_uid_or_organizer() {
        let mut incomplete = event();
        incomplete.uid = None;
        assert!(build_reply(&incomplete, PartStat::Accepted, "bob@example.com").is_none());
    }

    #[test]
    fn long_lines_are_folded() {
        let long = format!("SUMMARY:{}", "x".repeat(200));
        let folded = fold(&long);
        assert!(folded.contains("\r\n "), "continuation lines");
        assert!(folded.lines().all(|line| line.len() <= 74));
    }
}
