/*! Parsing human-friendly send delays and computing send times. */

use chrono::{Duration, Local, SecondsFormat};

/// Parse a delay such as `30m`, `2h`, `1d`, or a bare number of minutes.
///
/// Returns the number of seconds the message should wait.
pub fn parse_delay(input: &str) -> Option<i64> {
    let text = input.trim().to_ascii_lowercase();
    if text.is_empty() {
        return None;
    }
    let (digits, multiplier) = match text.chars().last()? {
        'm' => (&text[..text.len() - 1], 60),
        'h' => (&text[..text.len() - 1], 60 * 60),
        'd' => (&text[..text.len() - 1], 24 * 60 * 60),
        _ => (text.as_str(), 60),
    };
    let amount: i64 = digits.trim().parse().ok()?;
    if amount <= 0 {
        return None;
    }
    Some(amount * multiplier)
}

/// RFC 3339 timestamp `seconds` from now, in local time.
pub fn send_time_in(seconds: i64) -> String {
    (Local::now() + Duration::seconds(seconds)).to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Human description of when a queued message will go out.
pub fn describe_send_after(send_after: Option<&str>, now: &str) -> String {
    let Some(after) = send_after else {
        return "queued".to_string();
    };
    if after <= now {
        return "due".to_string();
    }
    after.to_string()
}

#[cfg(test)]
mod tests {
    use super::{parse_delay, send_time_in};

    #[test]
    fn parses_minutes_hours_and_days() {
        assert_eq!(parse_delay("30"), Some(1800), "bare numbers are minutes");
        assert_eq!(parse_delay("30m"), Some(1800));
        assert_eq!(parse_delay("2h"), Some(7200));
        assert_eq!(parse_delay("1d"), Some(86400));
        assert_eq!(parse_delay(" 15M "), Some(900));
    }

    #[test]
    fn rejects_empty_zero_and_garbage() {
        assert_eq!(parse_delay(""), None);
        assert_eq!(parse_delay("0"), None);
        assert_eq!(parse_delay("later"), None);
        assert_eq!(parse_delay("-5m"), None);
    }

    #[test]
    fn computes_a_future_timestamp() {
        let now = chrono::Local::now();
        let stamp = send_time_in(3600);
        let parsed = chrono::DateTime::parse_from_rfc3339(&stamp).expect("rfc3339");
        let delta = parsed.signed_duration_since(now).num_seconds();
        assert!((3500..=3700).contains(&delta), "delta was {delta}");
    }
}
