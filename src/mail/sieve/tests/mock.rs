//! A scripted ManageSieve server that records the commands it receives.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use super::super::*;

/// A scripted ManageSieve server that records the commands it receives.
pub(super) struct MockServer {
    port: u16,
    commands: Arc<Mutex<Vec<String>>>,
    literals: Arc<Mutex<Vec<String>>>,
    fail_next: Arc<Mutex<bool>>,
}

impl MockServer {
    /// A server that does not accept non-synchronizing literals.
    pub(super) fn start() -> Self {
        Self::start_with(false)
    }

    /// A server that advertises LITERAL+, so the client may send `{n+}`.
    pub(super) fn start_with_literal_plus() -> Self {
        Self::start_with(true)
    }

    pub(super) fn start_with(literal_plus: bool) -> Self {
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
                serve(stream, &recorder, &captured, &failure, literal_plus);
            }
        });

        MockServer {
            port,
            commands,
            literals,
            fail_next,
        }
    }

    pub(super) fn config(&self, security: SieveSecurity) -> SieveConfig {
        SieveConfig {
            host: "127.0.0.1".to_string(),
            port: self.port,
            username: "user".to_string(),
            password: "secret".to_string(),
            security,
        }
    }

    pub(super) fn commands(&self) -> Vec<String> {
        self.commands.lock().expect("commands").clone()
    }

    pub(super) fn literals(&self) -> Vec<String> {
        self.literals.lock().expect("literals").clone()
    }

    pub(super) fn fail_next(&self) {
        *self.fail_next.lock().expect("failure") = true;
    }
}

fn serve(
    stream: TcpStream,
    commands: &Arc<Mutex<Vec<String>>>,
    literals: &Arc<Mutex<Vec<String>>>,
    fail_next: &Arc<Mutex<bool>>,
    literal_plus: bool,
) {
    let mut writer = stream.try_clone().expect("clone");
    let greeting = if literal_plus {
        "\"IMPLEMENTATION\" \"mock\"\r\n\"SASL\" \"PLAIN\"\r\n\"SIEVE\" \"fileinto\"\r\n\"LITERAL+\"\r\nOK \"ready\"\r\n"
    } else {
        "\"IMPLEMENTATION\" \"mock\"\r\n\"SASL\" \"PLAIN\"\r\n\"SIEVE\" \"fileinto\"\r\nOK \"ready\"\r\n"
    };
    writer.write_all(greeting.as_bytes()).expect("greeting");

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
            // PUTSCRIPT / CHECKSCRIPT: a synchronizing literal waits for the
            // server's continuation before the payload is written.
            if !line.contains("+}") {
                let _ = writer.write_all(b"+ \"send the literal\"\r\n");
            }
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
