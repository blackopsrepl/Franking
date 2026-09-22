/*! The reader's own timezone, for showing a foreign time in local terms. */

/// The reader's IANA timezone name, when it can be determined.
///
/// Tries `TZ`, then the distribution's timezone file, then the zoneinfo
/// symlink that `localtime` points at. Falls back to UTC rather than guessing.
pub fn local_timezone_name() -> String {
    let from_env = std::env::var("TZ")
        .ok()
        .and_then(|value| normalize_timezone_name(&value));
    from_env
        .or_else(|| {
            std::fs::read_to_string("/etc/timezone")
                .ok()
                .and_then(|value| normalize_timezone_name(&value))
        })
        .or_else(|| {
            std::fs::read_link("/etc/localtime").ok().and_then(|path| {
                let text = path.to_string_lossy().to_string();
                text.split("zoneinfo/").nth(1).map(str::to_string)
            })
        })
        .unwrap_or_else(|| "UTC".to_string())
}

/// Trim a timezone identifier into the form this build can parse.
///
/// `TZ` may be written with a leading colon or with surrounding whitespace; a
/// value that is empty once trimmed carries no zone.
pub fn normalize_timezone_name(raw: &str) -> Option<String> {
    let name = raw.trim().trim_start_matches(':').trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_timezone_name;

    #[test]
    fn an_explicit_zone_is_normalized() {
        assert_eq!(
            normalize_timezone_name(":Europe/Berlin").as_deref(),
            Some("Europe/Berlin"),
            "a leading colon is stripped"
        );
        assert_eq!(
            normalize_timezone_name("  Europe/Rome \n").as_deref(),
            Some("Europe/Rome")
        );
        assert_eq!(normalize_timezone_name("  "), None);
        assert_eq!(normalize_timezone_name(":"), None);
    }
}
