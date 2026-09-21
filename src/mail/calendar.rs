/*! Minimal iCalendar (RFC 5545) invitation extraction for display. */

/// A calendar event extracted from a `text/calendar` part.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Event {
    pub summary: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub location: Option<String>,
}

impl Event {
    pub fn is_empty(&self) -> bool {
        self.summary.is_none() && self.start.is_none() && self.end.is_none()
    }

    pub fn summary_line(&self) -> Option<String> {
        let summary = self
            .summary
            .clone()
            .unwrap_or_else(|| "Untitled event".to_string());
        match self.start.as_deref() {
            Some(start) if !start.is_empty() => Some(format!("{summary} \u{2014} {start}")),
            _ => Some(summary),
        }
    }
}

/// Parse the first VEVENT from an iCalendar payload.
pub fn parse_invitation(text: &str) -> Option<Event> {
    let mut event = Event::default();
    let mut in_event = false;
    let mut found = false;

    for line in unfold(text) {
        let upper = line.to_ascii_uppercase();
        if upper == "BEGIN:VEVENT" {
            in_event = true;
            found = true;
            continue;
        }
        if upper == "END:VEVENT" {
            break;
        }
        if !in_event {
            continue;
        }

        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let property = name.split(';').next().unwrap_or(name).to_ascii_uppercase();
        let value = value.trim().to_string();
        if value.is_empty() {
            continue;
        }
        match property.as_str() {
            "SUMMARY" => event.summary = Some(value),
            "DTSTART" => event.start = Some(value),
            "DTEND" => event.end = Some(value),
            "LOCATION" => event.location = Some(value),
            _ => {}
        }
    }

    found.then_some(event)
}

/// Unfold RFC 5545 continuation lines (a leading space or tab continues).
fn unfold(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw in text.replace("\r\n", "\n").split('\n') {
        if (raw.starts_with(' ') || raw.starts_with('\t')) && !lines.is_empty() {
            if let Some(last) = lines.last_mut() {
                last.push_str(raw.trim_start());
            }
        } else {
            lines.push(raw.to_string());
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::parse_invitation;

    #[test]
    fn extracts_event_fields() {
        let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:Team sync\r\nDTSTART;TZID=UTC:20260413T090000\r\nDTEND:20260413T093000\r\nLOCATION:Room 1\r\nEND:VEVENT\r\nEND:VCALENDAR";
        let event = parse_invitation(ics).unwrap();

        assert_eq!(event.summary.as_deref(), Some("Team sync"));
        assert_eq!(event.start.as_deref(), Some("20260413T090000"));
        assert_eq!(event.location.as_deref(), Some("Room 1"));
        assert_eq!(
            event.summary_line().as_deref(),
            Some("Team sync \u{2014} 20260413T090000")
        );
    }

    #[test]
    fn returns_none_without_event() {
        assert!(parse_invitation("BEGIN:VCALENDAR\r\nEND:VCALENDAR").is_none());
    }
}
