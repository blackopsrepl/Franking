//! Live ManageSieve test against a real Pigeonhole server.
//!
//! Skipped unless `FRANKING_SIEVE_TEST_ADDR` is set; `scripts/live-test.sh`
//! (or `make live-test`) starts a Dovecot container with Pigeonhole and exports
//! that address, so the script path is verified against a real server rather
//! than only against a scripted stand-in.

use franking::mail::sieve::{SieveClient, SieveConfig, SieveSecurity};

/// The address the harness exported, if any.
fn address() -> Option<(String, u16)> {
    let value = std::env::var("FRANKING_SIEVE_TEST_ADDR").ok()?;
    let (host, port) = value.rsplit_once(':')?;
    Some((host.to_string(), port.parse().ok()?))
}

fn config(host: &str, port: u16) -> SieveConfig {
    SieveConfig {
        host: host.to_string(),
        port,
        // The container authenticates any user against its password.
        username: std::env::var("FRANKING_IMAP_TEST_USER").unwrap_or_else(|_| "test".to_string()),
        password: std::env::var("FRANKING_IMAP_TEST_PASSWORD")
            .unwrap_or_else(|_| "password".to_string()),
        security: SieveSecurity::Plain,
    }
}

#[test]
fn scripts_round_trip_against_a_real_sieve_server() {
    let Some((host, port)) = address() else {
        return;
    };
    let mut client = SieveClient::connect(&config(&host, port)).expect("connect to ManageSieve");
    assert!(
        client
            .capabilities()
            .iter()
            .any(|line| line.contains("SIEVE")),
        "Pigeonhole announces SIEVE: {:?}",
        client.capabilities()
    );

    let name = format!("sfm-live-{}", std::process::id());
    let body = "require [\"fileinto\"];\n\n# solverforge live probe\nkeep;\n";

    // A script must compile before it is stored, exactly as the app does it.
    client
        .check_script(body)
        .expect("CHECKSCRIPT accepts the script");
    client.put_script(&name, body).expect("PUTSCRIPT stores it");
    assert!(
        client
            .scripts()
            .expect("LISTSCRIPTS")
            .iter()
            .any(|script| script.name == name),
        "the stored script is listed"
    );
    assert_eq!(
        client.get_script(&name).expect("GETSCRIPT").trim_end(),
        body.trim_end(),
        "the script comes back unchanged"
    );

    // Renaming keeps the body, and activating makes it the active script.
    let renamed = format!("{name}-renamed");
    client.rename_script(&name, &renamed).expect("RENAMESCRIPT");
    client.set_active(Some(&renamed)).expect("SETACTIVE");
    let scripts = client.scripts().expect("LISTSCRIPTS");
    let active = scripts
        .iter()
        .find(|script| script.name == renamed)
        .expect("the renamed script is listed");
    assert!(active.active, "the renamed script is active: {scripts:?}");

    // Activating nothing, then deleting, leaves the server as it started.
    client.set_active(None).expect("SETACTIVE with no script");
    client.delete_script(&renamed).expect("DELETESCRIPT");
    assert!(
        !client
            .scripts()
            .expect("LISTSCRIPTS")
            .iter()
            .any(|script| script.name == renamed),
        "the script is gone"
    );

    client.logout();
}
