use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use solverforge_mail::mail::account_store::AccountRecord;
use solverforge_mail::mail::session::{
    map_imap_error, ConnectedImapSession, CredentialProvider, IdleOutcome, SessionPool,
};
use solverforge_mail::mail::MailResult;

#[derive(Debug)]
struct FixedCredentials;

impl CredentialProvider for FixedCredentials {
    fn lookup(&self, _service: &str, _username: &str) -> MailResult<String> {
        Ok("secret".to_string())
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Behavior {
    Normal,
    DropOnSelect,
}

struct FakeImap {
    port: u16,
    connections: Arc<AtomicUsize>,
    logins: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
}

impl FakeImap {
    fn start(behavior: Behavior) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake IMAP");
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();

        let connections = Arc::new(AtomicUsize::new(0));
        let logins = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let dropped = Arc::new(AtomicBool::new(false));

        let thread_connections = Arc::clone(&connections);
        let thread_logins = Arc::clone(&logins);
        let thread_stop = Arc::clone(&stop);
        let thread_dropped = Arc::clone(&dropped);

        thread::spawn(move || {
            while !thread_stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        thread_connections.fetch_add(1, Ordering::SeqCst);
                        let logins = Arc::clone(&thread_logins);
                        let dropped = Arc::clone(&thread_dropped);
                        thread::spawn(move || handle_connection(stream, behavior, logins, dropped));
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
) {
    let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
    let mut writer = stream;

    write_all(&mut writer, "* OK IMAP4rev1 fake ready\r\n");

    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let line = line.trim_end_matches(['\r', '\n']);
        let mut parts = line.splitn(2, ' ');
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
                reply(&mut writer, &tag, "OK LIST completed");
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

fn write_all(writer: &mut TcpStream, data: &str) {
    writer.write_all(data.as_bytes()).unwrap();
    writer.flush().unwrap();
}

fn reply(writer: &mut TcpStream, tag: &str, message: &str) {
    write_all(writer, &format!("{tag} {message}\r\n"));
}

fn account(port: u16) -> AccountRecord {
    AccountRecord {
        name: "work".to_string(),
        backend_kind: "imap".to_string(),
        provider_kind: "generic".to_string(),
        enabled: true,
        is_default: false,
        maildir_path: None,
        imap_host: Some("127.0.0.1".to_string()),
        imap_port: Some(port),
        imap_security: Some("plain".to_string()),
        smtp_host: None,
        smtp_port: None,
        smtp_security: None,
        auth_mode: Some("password".to_string()),
        username: Some("alice".to_string()),
        keyring_imap_secret_id: Some("service".to_string()),
        keyring_smtp_secret_id: None,
    }
}

fn select_inbox(connection: &mut ConnectedImapSession) -> MailResult<()> {
    match connection {
        ConnectedImapSession::Plain(session) => {
            session.select("INBOX").map_err(map_imap_error)?;
        }
        ConnectedImapSession::Tls(session) => {
            session.select("INBOX").map_err(map_imap_error)?;
        }
    }
    Ok(())
}

#[test]
fn session_pool_reuses_one_connection_across_operations() {
    let server = FakeImap::start(Behavior::Normal);
    let pool = SessionPool::with_credentials(Arc::new(FixedCredentials));
    let account = account(server.port);

    for _ in 0..2 {
        pool.with_connection(&account, select_inbox).unwrap();
    }

    assert_eq!(server.connections.load(Ordering::SeqCst), 1);
    assert_eq!(server.logins.load(Ordering::SeqCst), 1);
}

#[test]
fn capabilities_are_probed_on_connect() {
    let server = FakeImap::start(Behavior::Normal);
    let pool = SessionPool::with_credentials(Arc::new(FixedCredentials));
    let account = account(server.port);

    let capabilities = pool
        .with_connection(&account, |connection| {
            Ok(match connection {
                ConnectedImapSession::Plain(session) => {
                    let caps = session.capabilities().map_err(map_imap_error)?;
                    solverforge_mail::mail::session::Capabilities::from_imap(&caps)
                }
                ConnectedImapSession::Tls(session) => {
                    let caps = session.capabilities().map_err(map_imap_error)?;
                    solverforge_mail::mail::session::Capabilities::from_imap(&caps)
                }
            })
        })
        .unwrap();

    assert!(capabilities.idle);
    assert!(capabilities.move_);
    assert!(capabilities.uidplus);
    assert!(capabilities.imap4rev1);
}

#[test]
fn idle_watch_reports_mailbox_change() {
    let server = FakeImap::start(Behavior::Normal);
    let pool = SessionPool::with_credentials(Arc::new(FixedCredentials));
    let account = account(server.port);

    let outcome = pool
        .idle_wait(&account, "INBOX", Duration::from_secs(2))
        .unwrap();

    assert_eq!(outcome, IdleOutcome::MailboxChanged);
}

#[test]
fn transport_failure_is_retried_on_a_fresh_connection() {
    let server = FakeImap::start(Behavior::DropOnSelect);
    let pool = SessionPool::with_credentials(Arc::new(FixedCredentials));
    let account = account(server.port);

    // The first SELECT drops the connection; the pool must reconnect and retry.
    pool.with_connection(&account, select_inbox).unwrap();

    assert_eq!(server.connections.load(Ordering::SeqCst), 2);
    assert_eq!(server.logins.load(Ordering::SeqCst), 2);
}
