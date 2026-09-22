mod mock;

use super::*;
use mock::MockServer;

#[test]
fn connects_and_reads_capabilities() {
    let server = MockServer::start();
    let client = SieveClient::connect(&server.config(SieveSecurity::Plain)).expect("connect");
    assert!(client
        .capabilities()
        .iter()
        .any(|line| line.contains("mock")));
    assert!(!client.supports_starttls());
}

#[test]
fn lists_scripts_with_active_state() {
    let server = MockServer::start();
    let mut client = SieveClient::connect(&server.config(SieveSecurity::Plain)).expect("connect");
    let scripts = client.scripts().expect("scripts");
    assert_eq!(
        scripts,
        vec![
            SieveScript {
                name: "main".to_string(),
                active: true
            },
            SieveScript {
                name: "backup".to_string(),
                active: false
            },
        ]
    );
    assert!(server.commands().iter().any(|c| c == "LISTSCRIPTS"));
}

#[test]
fn fetches_a_script_body_from_a_literal() {
    let server = MockServer::start();
    let mut client = SieveClient::connect(&server.config(SieveSecurity::Plain)).expect("connect");
    let body = client.get_script("main").expect("get");
    assert_eq!(body, "require [\"fileinto\"];\n");
    assert!(server.commands().iter().any(|c| c == "GETSCRIPT \"main\""));
}

#[test]
fn put_script_sends_a_sized_literal() {
    let server = MockServer::start();
    let mut client = SieveClient::connect(&server.config(SieveSecurity::Plain)).expect("connect");
    let body = "if header :contains \"from\" \"alice\" { fileinto \"Alice\"; }";
    client.put_script("main", body).expect("put");

    let command = server
        .commands()
        .into_iter()
        .find(|c| c.starts_with("PUTSCRIPT"))
        .expect("putscript");
    assert!(command.starts_with("PUTSCRIPT \"main\" {"));
    assert_eq!(server.literals(), vec![body.to_string()]);
}

#[test]
fn activates_and_deactivates_scripts() {
    let server = MockServer::start();
    let mut client = SieveClient::connect(&server.config(SieveSecurity::Plain)).expect("connect");
    client.set_active(Some("main")).expect("activate");
    client.set_active(None).expect("deactivate");

    let commands = server.commands();
    assert!(commands.iter().any(|c| c == "SETACTIVE \"main\""));
    assert!(commands.iter().any(|c| c == "SETACTIVE \"\""));
}

#[test]
fn deletes_a_script() {
    let server = MockServer::start();
    let mut client = SieveClient::connect(&server.config(SieveSecurity::Plain)).expect("connect");
    client.delete_script("backup").expect("delete");
    assert!(server
        .commands()
        .iter()
        .any(|c| c == "DELETESCRIPT \"backup\""));
}

#[test]
fn checking_a_script_sends_the_literal() {
    let server = MockServer::start();
    let mut client = SieveClient::connect(&server.config(SieveSecurity::Plain)).expect("connect");
    client.check_script("if true { stop; }").expect("check");
    assert!(server
        .commands()
        .iter()
        .any(|c| c.starts_with("CHECKSCRIPT")));
    assert_eq!(server.literals(), vec!["if true { stop; }".to_string()]);
}

#[test]
fn surfaces_server_failures() {
    let server = MockServer::start();
    let mut client = SieveClient::connect(&server.config(SieveSecurity::Plain)).expect("connect");
    server.fail_next();
    let error = client.scripts().expect_err("expected failure");
    assert!(error.to_string().contains("script error"));
}

#[test]
fn quotes_and_parses_wire_values() {
    assert_eq!(quote("plain"), "\"plain\"");
    assert_eq!(quote("a\"b"), "\"a\\\"b\"");
    assert_eq!(quoted_value("  \"main\" ACTIVE"), Some("main".to_string()));
    assert_eq!(quoted_value("no quotes"), None);
    assert_eq!(literal_size("PUTSCRIPT \"main\" {12+}"), Some(12));
    assert_eq!(literal_size("{7}"), Some(7));
    assert_eq!(literal_size("OK"), None);
    assert_eq!(
        parse_script("\"main\" ACTIVE"),
        SieveScript {
            name: "main".to_string(),
            active: true
        }
    );
}

#[test]
fn renames_a_script_and_reports_server_refusals() {
    let server = MockServer::start();
    let mut client = SieveClient::connect(&server.config(SieveSecurity::Plain)).expect("connect");

    client
        .rename_script("old-name", "new-name")
        .expect("rename");
    assert!(
        server
            .commands()
            .iter()
            .any(|command| command == "RENAMESCRIPT \"old-name\" \"new-name\""),
        "the client sends RENAMESCRIPT: {:?}",
        server.commands()
    );

    // A refusal from the server surfaces as an error.
    let error = client
        .rename_script("missing", "other")
        .expect_err("refused");
    assert!(error.to_string().contains("no such script"), "{error}");
}

#[test]
fn a_server_with_literal_plus_takes_the_non_synchronizing_form() {
    let server = MockServer::start_with_literal_plus();
    let mut client = SieveClient::connect(&server.config(SieveSecurity::Plain)).expect("connect");

    client.put_script("main", "keep;\n").expect("put");
    let command = server
        .commands()
        .iter()
        .find(|command| command.starts_with("PUTSCRIPT"))
        .expect("putscript")
        .clone();
    assert!(
        command.ends_with("+}"),
        "a server that accepts one is sent a non-synchronizing literal: {command}"
    );
    assert_eq!(server.literals(), vec!["keep;\n".to_string()]);
}
