//! The CLI boundary, against a stand-in binary.

use std::io::Write;
use std::path::PathBuf;

use super::Planner123;

/// A stand-in for `planner123-cli` that records its arguments and answers
/// with canned JSON, so the integration is tested without the real binary.
fn fake_cli(dir: &std::path::Path, stdout: &str, exit_ok: bool) -> PathBuf {
    let path = dir.join("planner123-cli");
    let script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > {dir}/args.txt\n{}\nprintf '%s' '{stdout}'\nexit {}\n",
        if exit_ok { "" } else { "echo 'boom' >&2" },
        if exit_ok { 0 } else { 1 },
        dir = dir.display()
    );
    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(script.as_bytes()).unwrap();
    drop(file);
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    std::fs::set_permissions(&path, perms).unwrap();
    path
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sfm-planner-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The environment variable is process-wide, so these tests take a lock.
fn with_cli<T>(dir: &std::path::Path, body: impl FnOnce() -> T) -> T {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    let _guard = LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let previous = std::env::var_os("FRANKING_PLANNER123_CLI");
    std::env::set_var("FRANKING_PLANNER123_CLI", dir.join("planner123-cli"));
    let result = body();
    match previous {
        Some(value) => std::env::set_var("FRANKING_PLANNER123_CLI", value),
        None => std::env::remove_var("FRANKING_PLANNER123_CLI"),
    }
    result
}

#[test]
fn an_invitation_is_written_and_handed_to_the_cli() {
    let dir = scratch("import");
    fake_cli(
        &dir,
        r#"{"data":{"calendar_name":"Personal","imported":1,"skipped":0,"warnings":[]},"status":"ok"}"#,
        true,
    );

    let report = with_cli(&dir, || {
        let client = Planner123::discover(Some("cal-1".to_string()), "Europe/Berlin".to_string())
            .expect("the fake CLI is discovered");
        client.import_invitation(b"BEGIN:VCALENDAR\nEND:VCALENDAR\n")
    })
    .expect("the import succeeds");

    assert_eq!(report.imported, 1);
    assert_eq!(report.calendar_name, "Personal");

    // The CLI was called as documented, with a file that held the payload.
    let args = std::fs::read_to_string(dir.join("args.txt")).unwrap();
    assert!(args.contains("ical"), "{args}");
    assert!(args.contains("import"), "{args}");
    assert!(args.contains("--calendar-id\ncal-1"), "{args}");
    assert!(args.contains("--timezone\nEurope/Berlin"), "{args}");
    let path = args
        .lines()
        .skip_while(|line| *line != "--path")
        .nth(1)
        .expect("the path argument");
    assert!(path.ends_with(".ics"), "{path}");
    assert!(
        !std::path::Path::new(path).exists(),
        "the temporary file is cleaned up"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_refused_import_reports_the_cli_error() {
    let dir = scratch("refused");
    fake_cli(&dir, "", false);

    let error = with_cli(&dir, || {
        let client =
            Planner123::discover(Some("cal-1".to_string()), "UTC".to_string()).expect("discovered");
        client.import_invitation(b"BEGIN:VCALENDAR\nEND:VCALENDAR\n")
    })
    .expect_err("refused");
    assert!(error.to_string().contains("boom"), "{error}");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_default_calendar_comes_from_the_cli() {
    let dir = scratch("default");
    fake_cli(
        &dir,
        r#"{"data":[{"id":"a","is_default":false},{"id":"b","is_default":true}]}"#,
        true,
    );

    let id = with_cli(&dir, || {
        let client =
            Planner123::discover(None, "UTC".to_string()).expect("the fake CLI is discovered");
        client.default_calendar_id()
    })
    .expect("a calendar");
    assert_eq!(id, "b");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_binary_is_reported_as_such() {
    let previous = std::env::var_os("FRANKING_PLANNER123_CLI");
    std::env::set_var("FRANKING_PLANNER123_CLI", "/nonexistent/planner123-cli");
    let found = Planner123::discover(None, "UTC".to_string());
    match previous {
        Some(value) => std::env::set_var("FRANKING_PLANNER123_CLI", value),
        None => std::env::remove_var("FRANKING_PLANNER123_CLI"),
    }
    // Another planner123-cli may be on PATH, which is a valid discovery.
    if let Some(client) = found {
        assert!(
            !client.binary.starts_with("/nonexistent"),
            "an explicit path that does not exist is not used"
        );
    }
}
