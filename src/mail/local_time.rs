/*! The reader's own timezone, for showing a foreign time in local terms. */

/// The reader's IANA timezone name, when it can be determined.
///
/// Tries `TZ`, then the distribution's timezone file, then the zoneinfo
/// symlink that `localtime` points at. Falls back to UTC rather than guessing.
pub fn local_timezone_name() -> String {
    if let Ok(name) = std::env::var("TZ") {
        let name = name.trim().trim_start_matches(':');
        if !name.is_empty() {
            return name.to_string();
        }
    }
    std::fs::read_to_string("/etc/timezone")
        .map(|value| value.trim().to_string())
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            std::fs::read_link("/etc/localtime").ok().and_then(|path| {
                let text = path.to_string_lossy().to_string();
                text.split("zoneinfo/").nth(1).map(str::to_string)
            })
        })
        .unwrap_or_else(|| "UTC".to_string())
}

#[cfg(test)]
mod tests {
    use super::local_timezone_name;

    #[test]
    fn an_explicit_zone_wins() {
        let previous = std::env::var_os("TZ");
        std::env::set_var("TZ", ":Europe/Berlin");
        let name = local_timezone_name();
        match previous {
            Some(value) => std::env::set_var("TZ", value),
            None => std::env::remove_var("TZ"),
        }
        assert_eq!(name, "Europe/Berlin", "a leading colon is stripped");
    }
}
