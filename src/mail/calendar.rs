/*! Minimal iCalendar (RFC 5545) invitation extraction for display. */

/// A calendar event extracted from a `text/calendar` part.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Event {
    /// `UID` property, needed to reply to the invitation.
    pub uid: Option<String>,
    /// `ORGANIZER` value (usually `mailto:...`).
    pub organizer: Option<String>,
    pub summary: Option<String>,
    /// Event start exactly as the invitation wrote it, for round-tripping.
    pub start: Option<String>,
    pub end: Option<String>,
    /// Timezone identifier of the start (`TZID`), when the invitation named one.
    pub start_tz: Option<String>,
    /// True for an all-day event (`VALUE=DATE`), which has no time of day.
    pub all_day: bool,
    /// True when the invitation cancels the event (`METHOD:CANCEL` or a
    /// cancelled status), which must not be answered.
    pub cancelled: bool,
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
        let line = match self.start_display() {
            Some(start) => format!("{summary} \u{2014} {start}"),
            None => summary,
        };
        Some(if self.cancelled {
            format!("{line} (cancelled)")
        } else {
            line
        })
    }

    /// The start time for display, with its timezone resolved where possible.
    ///
    /// An invitation states the time either in UTC, in a named timezone, or as
    /// a bare local time. A named timezone is converted to the reader's local
    /// time and the identifier is kept, because a bare time in an unknown
    /// timezone would otherwise read as if it were local.
    pub fn start_display(&self) -> Option<String> {
        let start = self.start.as_deref().filter(|start| !start.is_empty())?;
        if self.all_day {
            return Some(format!("{} (all day)", date_display(start)));
        }
        let Some(naive) = parse_naive(start) else {
            return Some(start.to_string());
        };

        let Some(name) = self.timezone_name() else {
            return Some(naive.format("%Y-%m-%d %H:%M").to_string());
        };
        let Ok(tz) = name.parse::<chrono_tz::Tz>() else {
            // An identifier this build does not know: show it rather than
            // presenting the time as if it were local.
            return Some(format!("{} ({name})", naive.format("%Y-%m-%d %H:%M")));
        };
        match naive.and_local_timezone(tz) {
            chrono::LocalResult::Single(at) | chrono::LocalResult::Ambiguous(at, _) => {
                Some(format!(
                    "{} ({})",
                    at.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M"),
                    tz.name()
                ))
            }
            chrono::LocalResult::None => Some(format!("{} ({})", naive, tz.name())),
        }
    }

    /// The timezone identifier of the start: `UTC` when the value ends in `Z`.
    fn timezone_name(&self) -> Option<&str> {
        if self
            .start
            .as_deref()
            .is_some_and(|start| start.ends_with('Z'))
        {
            return Some("UTC");
        }
        self.start_tz.as_deref()
    }
}

/// Parse the first VEVENT from an iCalendar payload.
pub fn parse_invitation(text: &str) -> Option<Event> {
    let mut event = Event::default();
    let mut in_event = false;
    let mut found = false;

    for line in unfold(text) {
        let upper = line.to_ascii_uppercase();
        if upper.starts_with("METHOD:") {
            // METHOD sits on the VCALENDAR, before the event.
            event.cancelled = upper.trim_start_matches("METHOD:").trim() == "CANCEL";
            continue;
        }
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
        // Only the parameter name is case-insensitive; its value is not, and a
        // timezone identifier is case-sensitive.
        let parameters: Vec<(String, Option<String>)> = name
            .split(';')
            .skip(1)
            .map(|parameter| match parameter.split_once('=') {
                Some((key, value)) => (
                    key.trim().to_ascii_uppercase(),
                    Some(value.trim().to_string()),
                ),
                None => (parameter.trim().to_ascii_uppercase(), None),
            })
            .collect();
        let property = name.split(';').next().unwrap_or(name).to_ascii_uppercase();
        let value = value.trim().to_string();
        if value.is_empty() {
            continue;
        }
        let tzid = parameters
            .iter()
            .find(|(key, _)| key == "TZID")
            .and_then(|(_, value)| value.clone());
        let all_day = parameters
            .iter()
            .any(|(key, value)| key == "VALUE" && value.as_deref() == Some("DATE"));

        match property.as_str() {
            "UID" => event.uid = Some(value),
            "ORGANIZER" => event.organizer = Some(value.trim_start_matches("mailto:").to_string()),
            "SUMMARY" => event.summary = Some(value),
            "DTSTART" => {
                event.start = Some(value);
                event.start_tz = tzid;
                event.all_day = all_day;
            }
            "DTEND" => event.end = Some(value),
            "STATUS" => {
                event.cancelled |= value.eq_ignore_ascii_case("CANCELLED");
            }
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

/// Parse an iCalendar date-time (`20260413T090000` or `20260413`).
fn parse_naive(value: &str) -> Option<chrono::NaiveDateTime> {
    let value = value.trim_end_matches('Z');
    chrono::NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S")
        .or_else(|_| {
            chrono::NaiveDate::parse_from_str(value, "%Y%m%d")
                .map(|date| date.and_hms_opt(0, 0, 0).expect("midnight"))
        })
        .ok()
}

/// Render an iCalendar date as `YYYY-MM-DD` when it is parseable.
fn date_display(value: &str) -> String {
    chrono::NaiveDate::parse_from_str(value.trim_end_matches('Z'), "%Y%m%d")
        .map(|date| date.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|_| value.to_string())
}

#[cfg(test)]
mod tests;
