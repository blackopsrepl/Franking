//! A minimal in-process IMAP server for exercising session behavior.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Behavior {
    Normal,
    DropOnSelect,
}

pub(crate) struct FakeImap {
    pub(crate) port: u16,
    pub(crate) connections: Arc<AtomicUsize>,
    pub(crate) logins: Arc<AtomicUsize>,
    pub(crate) appended: Arc<Mutex<Vec<u8>>>,
    stop: Arc<AtomicBool>,
}

impl FakeImap {
    pub(crate) fn start(behavior: Behavior) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake IMAP");
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();

        let connections = Arc::new(AtomicUsize::new(0));
        let logins = Arc::new(AtomicUsize::new(0));
        let appended = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let dropped = Arc::new(AtomicBool::new(false));

        let thread_connections = Arc::clone(&connections);
        let thread_logins = Arc::clone(&logins);
        let thread_appended = Arc::clone(&appended);
        let thread_stop = Arc::clone(&stop);
        let thread_dropped = Arc::clone(&dropped);

        thread::spawn(move || {
            while !thread_stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        thread_connections.fetch_add(1, Ordering::SeqCst);
                        let logins = Arc::clone(&thread_logins);
                        let dropped = Arc::clone(&thread_dropped);
                        let appended = Arc::clone(&thread_appended);
                        thread::spawn(move || {
                            handle_connection(stream, behavior, logins, dropped, appended)
                        });
                    }
                    Err(ref error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            port,
            connections,
            logins,
            appended,
            stop,
        }
    }
}

impl Drop for FakeImap {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn handle_connection(
    stream: TcpStream,
    behavior: Behavior,
    logins: Arc<AtomicUsize>,
    dropped: Arc<AtomicBool>,
    appended: Arc<Mutex<Vec<u8>>>,
) {
    let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
    let mut writer = stream;

    write_all(&mut writer, "* OK IMAP4rev1 fake ready\r\n");

    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let raw_line = line.trim_end_matches(['\r', '\n']).to_string();
        let mut parts = raw_line.splitn(2, ' ');
        let tag = parts.next().unwrap_or("x").to_string();
        let rest = parts.next().unwrap_or("").to_uppercase();
        let command = rest.split_whitespace().next().unwrap_or("");

        match command {
            "LOGIN" => {
                logins.fetch_add(1, Ordering::SeqCst);
                reply(&mut writer, &tag, "OK LOGIN completed");
            }
            "CAPABILITY" => {
                write_all(&mut writer, "* CAPABILITY IMAP4rev1 IDLE MOVE UIDPLUS\r\n");
                reply(&mut writer, &tag, "OK CAPABILITY completed");
            }
            "SELECT" | "EXAMINE" => {
                if behavior == Behavior::DropOnSelect && !dropped.swap(true, Ordering::SeqCst) {
                    // Simulate a dropped connection mid-operation, once.
                    return;
                }
                write_all(&mut writer, "* 1 EXISTS\r\n");
                reply(&mut writer, &tag, "OK [READ-WRITE] SELECT completed");
            }
            "LIST" => {
                write_all(&mut writer, "* LIST () \"/\" \"INBOX\"\r\n");
                write_all(&mut writer, "* LIST (\\Sent) \"/\" \"Sent\"\r\n");
                reply(&mut writer, &tag, "OK LIST completed");
            }
            "APPEND" => {
                let size = literal_size(&raw_line).unwrap_or(0);
                write_all(&mut writer, "+ Ready for literal data\r\n");
                let mut buffer = vec![0u8; size];
                if reader.read_exact(&mut buffer).is_err() {
                    return;
                }
                appended.lock().unwrap().extend_from_slice(&buffer);
                reply(&mut writer, &tag, "OK [APPENDUID 1 42] APPEND completed");
            }
            "IDLE" => {
                write_all(&mut writer, "+ idling\r\n");
                thread::sleep(Duration::from_millis(50));
                write_all(&mut writer, "* 1 EXISTS\r\n");
                let mut done = String::new();
                let _ = reader.read_line(&mut done);
                reply(&mut writer, &tag, "OK IDLE terminated");
            }
            "NOOP" => reply(&mut writer, &tag, "OK NOOP completed"),
            "LOGOUT" => {
                write_all(&mut writer, "* BYE\r\n");
                reply(&mut writer, &tag, "OK LOGOUT completed");
                return;
            }
            _ => reply(&mut writer, &tag, "OK completed"),
        }
    }
}

/// Parse the synchronizing literal size from a command line like `APPEND "Sent" {42}`.
fn literal_size(line: &str) -> Option<usize> {
    let start = line.rfind('{')? + 1;
    let end = line[start..].find('}')? + start;
    line[start..end].trim_end_matches('+').trim().parse().ok()
}

fn write_all(writer: &mut TcpStream, data: &str) {
    writer.write_all(data.as_bytes()).unwrap();
    writer.flush().unwrap();
}

fn reply(writer: &mut TcpStream, tag: &str, message: &str) {
    write_all(writer, &format!("{tag} {message}\r\n"));
}
