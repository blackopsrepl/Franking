use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use super::*;

/// A scripted ManageSieve server that records the commands it receives.
struct MockServer {
    port: u16,
    commands: Arc<Mutex<Vec<String>>>,
    literals: Arc<Mutex<Vec<String>>>,
    fail_next: Arc<Mutex<bool>>,
}

impl MockServer {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let commands = Arc::new(Mutex::new(Vec::new()));
        let literals = Arc::new(Mutex::new(Vec::new()));
        let fail_next = Arc::new(Mutex::new(false));

        let recorder = commands.clone();
        let captured = literals.clone();
        let failure = fail_next.clone();
        thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                serve(stream, &recorder, &captured, &failure);
            }
        });

        MockServer {
            port,
            commands,
            literals,
            fail_next,
        }
    }

    fn config(&self, security: SieveSecurity) -> SieveConfig {
        SieveConfig {
            host: "127.0.0.1".to_string(),
            port: self.port,
            username: "user".to_string(),
            password: "secret".to_string(),
            security,
        }
    }

    fn commands(&self) -> Vec<String> {
        self.commands.lock().expect("commands").clone()
    }

    fn literals(&self) -> Vec<String> {
        self.literals.lock().expect("literals").clone()
    }

    fn fail_next(&self) {
        *self.fail_next.lock().expect("failure") = true;
    }
}

fn serve(
    stream: TcpStream,
    commands: &Arc<Mutex<Vec<String>>>,
    literals: &Arc<Mutex<Vec<String>>>,
    fail_next: &Arc<Mutex<bool>>,
) {
    let mut writer = stream.try_clone().expect("clone");
    writer
        .write_all(b"\"IMPLEMENTATION\" \"mock\"\r\n\"SASL\" \"PLAIN\"\r\n\"SIEVE\" \"fileinto\"\r\nOK \"ready\"\r\n")
        .expect("greeting");

    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).expect("read") == 0 {
            return;
        }
        let line = line.trim_end_matches(['\r', '\n']).to_string();
        if line.is_empty() {
            continue;
        }
        commands.lock().expect("commands").push(line.clone());

        if *fail_next.lock().expect("failure") {
            *fail_next.lock().expect("failure") = false;
            let _ = writer.write_all(b"NO \"script error\"\r\n");
            continue;
        }

        let upper = line.to_ascii_uppercase();
        if upper.starts_with("AUTHENTICATE") {
            let _ = writer.write_all(b"OK \"logged in\"\r\n");
        } else if upper.starts_with("LISTSCRIPTS") {
            let _ = writer.write_all(b"\"main\" ACTIVE\r\n\"backup\"\r\nOK \"listed\"\r\n");
        } else if upper.starts_with("GETSCRIPT") {
            let body = "require [\"fileinto\"];\n";
            let _ = writer.write_all(format!("{{{}}}\r\n", body.len()).as_bytes());
            let _ = writer.write_all(body.as_bytes());
            let _ = writer.write_all(b"OK \"got\"\r\n");
        } else if let Some(size) = literal_size(&line) {
            // PUTSCRIPT / CHECKSCRIPT: consume the literal payload.
            let mut body = vec![0u8; size];
            reader.read_exact(&mut body).expect("literal");
            literals
                .lock()
                .expect("literals")
                .push(String::from_utf8_lossy(&body).to_string());
            let _ = writer.write_all(b"OK \"stored\"\r\n");
        } else if upper.starts_with("RENAMESCRIPT") {
            // Scripted refusal for one name, so the error path is exercised.
            if line.contains("missing") {
                let _ = writer.write_all(b"NO \"no such script\"\r\n");
            } else {
                let _ = writer.write_all(b"OK \"renamed\"\r\n");
            }
        } else if upper.starts_with("SETACTIVE") || upper.starts_with("DELETESCRIPT") {
            let _ = writer.write_all(b"OK \"done\"\r\n");
        } else if upper.starts_with("LOGOUT") {
            let _ = writer.write_all(b"OK \"bye\"\r\n");
            return;
        } else {
            let _ = writer.write_all(b"NO \"unknown command\"\r\n");
        }
    }
}

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
