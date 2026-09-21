/*! IDLE on the app-owned client. */

use std::time::{Duration, Instant};

use crate::mail::errors::{MailError, MailResult};

use super::client::ImapClient;
use super::transport::ReadWrite;

pub type Connection = ImapClient<Box<dyn ReadWrite>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleOutcome {
    TimedOut,
    MailboxChanged,
}

/// Select the mailbox whose changes IDLE reports.
pub fn select(client: &mut Connection, folder: &str) -> MailResult<()> {
    let mailbox = imap_types::mailbox::Mailbox::try_from(folder.to_string()).map_err(|error| {
        MailError::invalid_input(format!("invalid mailbox {folder}: {error:?}"))
    })?;
    client
        .run(imap_types::command::CommandBody::Select {
            mailbox,
            parameters: vec![],
        })
        .map_err(|error| MailError::other(error.to_string()))?
        .require_ok("SELECT")?;
    Ok(())
}

/// Wait for a mailbox change, re-issuing IDLE until the deadline passes.
///
/// The selected mailbox is the only one IDLE reports changes for, so callers
/// must select first.
pub fn wait(client: &mut Connection, timeout: Duration) -> MailResult<IdleOutcome> {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(IdleOutcome::TimedOut);
        }
        adjust_read_timeout(client, Some(remaining))?;
        let outcome = client.idle_once();
        adjust_read_timeout(client, Some(super::transport::NETWORK_TIMEOUT))?;

        match outcome {
            Ok(true) => return Ok(IdleOutcome::MailboxChanged),
            // A read timeout is the server staying quiet.
            Ok(false) | Err(_) if Instant::now() >= deadline => return Ok(IdleOutcome::TimedOut),
            Ok(false) => continue,
            Err(error) => {
                if is_timeout(&error) {
                    continue;
                }
                return Err(MailError::connection_dropped(error.to_string()));
            }
        }
    }
}

fn adjust_read_timeout(client: &mut Connection, timeout: Option<Duration>) -> MailResult<()> {
    client
        .set_read_timeout(timeout)
        .map_err(|error| MailError::io(error.to_string()))
}

fn is_timeout(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io| matches!(io.kind(), std::io::ErrorKind::TimedOut))
    })
}
