//! Live proof that an invitation reaches a real Planner123 database.
//!
//! Skipped unless `FRANKING_PLANNER123_CLI` points at a `planner123-cli`
//! binary, so `cargo test` stays self-contained:
//!
//! ```text
//! FRANKING_PLANNER123_CLI=/path/to/planner123-cli cargo test --test planner_live_test
//! ```
//!
//! The test runs against a scratch XDG data directory, never the user's own
//! calendar database.

use std::path::PathBuf;
use std::process::Command;

use franking::mail::planner123::Planner123;

/// The real CLI, when one is configured.
fn configured_cli() -> Option<PathBuf> {
    std::env::var_os("FRANKING_PLANNER123_CLI").map(PathBuf::from)
}

/// An invitation whose event carries a named timezone.
const INVITATION: &str = "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
METHOD:REQUEST\r\n\
BEGIN:VEVENT\r\n\
UID:mail-live-probe@example.com\r\n\
DTSTAMP:20260413T080000Z\r\n\
DTSTART;TZID=Europe/Berlin:20260415T140000\r\n\
DTEND;TZID=Europe/Berlin:20260415T150000\r\n\
SUMMARY:Mail live probe\r\n\
ORGANIZER:mailto:alice@example.com\r\n\
ATTENDEE;PARTSTAT=NEEDS-ACTION:mailto:bob@example.com\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

#[test]
fn an_invitation_becomes_an_event_in_planner123() {
    let Some(cli) = configured_cli() else {
        return;
    };
    assert!(cli.is_file(), "{} is not a file", cli.display());

    // Point the child process at a scratch database, and take the lock the
    // other environment-dependent tests use.
    let dir = std::env::temp_dir().join(format!("sfm-planner-live-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch directory");
    std::env::set_var("XDG_DATA_HOME", dir.join("data"));
    std::env::set_var("XDG_CONFIG_HOME", dir.join("config"));
    std::env::set_var("FRANKING_PLANNER123_CLI", &cli);

    // A calendar to import into, created through the same CLI.
    let status = Command::new(&cli)
        .args([
            "calendars",
            "create",
            "--name",
            "Mail",
            "--color",
            "#82fb9c",
        ])
        .output()
        .expect("run planner123-cli");
    assert!(status.status.success(), "creating a calendar failed");

    let client = Planner123::discover(None, "Europe/Berlin".to_string()).expect("the CLI");
    let report = client
        .import_invitation(INVITATION.as_bytes())
        .expect("the invitation is imported");
    assert_eq!(report.imported, 1, "one event: {report:?}");
    assert_eq!(report.calendar_name, "Mail");

    // The event is really there, and it kept the organizer's timezone.
    let listed = Command::new(&cli)
        .args(["events", "list"])
        .output()
        .expect("list");
    let events = String::from_utf8_lossy(&listed.stdout);
    assert!(events.contains("Mail live probe"), "{events}");
    assert!(
        events.contains("Europe/Berlin"),
        "the zone survived: {events}"
    );
    assert!(events.contains("2026-04-15"), "the date survived: {events}");

    let _ = std::fs::remove_dir_all(&dir);
}
