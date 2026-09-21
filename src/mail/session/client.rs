/*! App-owned IMAP command layer.
`imap-codec` encodes the command and decodes the response; this type owns the
transport, tag allocation, literal continuations, and the mapping of failures
into the app's error taxonomy. */

use anyhow::{anyhow, Context, Result};
use imap_codec::encode::{Encoder, Fragment};
use imap_codec::CommandCodec;
use imap_types::command::{Command, CommandBody};
use imap_types::core::LiteralMode;
use imap_types::response::{Capability, Response};

use crate::mail::errors::MailError;

use super::codec::{CommandOutput, ResponseReader};
use super::transport::ReadWrite;

/// An IMAP connection over a specific stream type.
pub struct ImapClient<S: ReadWrite + 'static = Box<dyn ReadWrite>> {
    stream: ResponseReader<S>,
    tag: u64,
}

impl<S: ReadWrite + 'static> ImapClient<S> {
    /// Wrap an established (and already secured) stream.
    pub fn new(stream: S) -> Self {
        Self {
            stream: ResponseReader::new(stream),
            tag: 0,
        }
    }

    /// Take back the raw stream, for upgrading the transport.
    pub fn into_inner(self) -> S {
        self.stream.into_inner()
    }

    /// Box the stream so the concrete transport stops mattering.
    pub fn into_boxed(self) -> ImapClient<Box<dyn ReadWrite>> {
        ImapClient::new(Box::new(self.stream.into_inner()))
    }

    /// Undecodable response lines seen so far.
    pub fn skipped_lines(&self) -> &[String] {
        &self.stream.skipped
    }

    fn next_tag(&mut self) -> String {
        self.tag += 1;
        format!("a{:04}", self.tag)
    }

    /// Send one command and collect its responses.
    pub fn run(&mut self, body: CommandBody<'static>) -> Result<CommandOutput> {
        let tag = self.next_tag();
        let command = Command::new(tag.clone(), body).context("building the command")?;

        for fragment in CommandCodec::default().encode(&command) {
            match fragment {
                Fragment::Line { data } => self.stream.write_all(&data)?,
                Fragment::Literal { data, mode } => {
                    if matches!(mode, LiteralMode::Sync) {
                        // A synchronizing literal waits for the continuation.
                        let continuation = self.stream.read_raw_line()?;
                        if !continuation.starts_with('+') {
                            return Err(anyhow!("server refused the literal: {continuation}"));
                        }
                    }
                    self.stream.write_all(&data)?;
                    self.stream.write_all(b"\r\n")?;
                }
            }
        }

        self.stream.collect(&tag)
    }

    /// Capability list from an untagged CAPABILITY response.
    pub fn capability(&mut self) -> Result<Vec<Capability<'static>>> {
        let output = self
            .run(CommandBody::Capability)?
            .require_ok("CAPABILITY")?;
        let capability = output
            .responses
            .iter()
            .find_map(|response| match response {
                Response::Data(imap_types::response::Data::Capability(list)) => Some(list.clone()),
                _ => None,
            })
            .ok_or_else(|| anyhow!("CAPABILITY returned no capability list"))?;
        Ok(capability.into_iter().collect())
    }

    /// Password login (LOGIN).
    pub fn login(&mut self, username: &str, password: &str) -> Result<()> {
        let username: String = username.to_string();
        let password: String = password.to_string();
        let body = CommandBody::login(username, password).context("LOGIN arguments")?;
        self.run(body)?.require_ok("LOGIN")?;
        Ok(())
    }

    /// SASL PLAIN authentication.
    pub fn authenticate_plain(&mut self, username: &str, password: &str) -> Result<()> {
        let body = CommandBody::authenticate(imap_types::auth::AuthMechanism::Plain);
        let tag = self.next_tag();
        let command = Command::new(tag.clone(), body).context("building AUTHENTICATE")?;
        for fragment in CommandCodec::default().encode(&command) {
            match fragment {
                Fragment::Line { data } => self.stream.write_all(&data)?,
                Fragment::Literal { data, mode } => {
                    if matches!(mode, LiteralMode::Sync) {
                        let continuation = self.stream.read_raw_line()?;
                        if !continuation.starts_with('+') {
                            return Err(anyhow!("server refused SASL: {continuation}"));
                        }
                    }
                    self.stream.write_all(&data)?;
                }
            }
        }

        // Respond to each continuation challenge with the credentials.
        loop {
            let line = self.stream.read_raw_line()?;
            if line.starts_with('+') {
                let payload = base64_plain(username, password);
                self.stream.write_all(payload.as_bytes())?;
                self.stream.write_all(b"\r\n")?;
                continue;
            }
            if line.starts_with(&tag) {
                let upper = line.to_ascii_uppercase();
                if upper.contains(" OK") {
                    return Ok(());
                }
                return Err(
                    MailError::imap_auth_rejected(format!("SASL PLAIN rejected: {line}")).into(),
                );
            }
        }
    }

    /// XOAUTH2 SASL authentication.
    pub fn authenticate_xoauth2(&mut self, username: &str, access_token: &str) -> Result<()> {
        let mechanism = imap_types::auth::AuthMechanism::try_from("XOAUTH2")
            .context("XOAUTH2 mechanism name")?;
        let body = CommandBody::authenticate(mechanism);
        let tag = self.next_tag();
        let command = Command::new(tag.clone(), body).context("building AUTHENTICATE")?;
        for fragment in CommandCodec::default().encode(&command) {
            match fragment {
                Fragment::Line { data } => self.stream.write_all(&data)?,
                Fragment::Literal { data, mode } => {
                    if matches!(mode, LiteralMode::Sync) {
                        let _ = self.stream.read_raw_line()?;
                    }
                    self.stream.write_all(&data)?;
                }
            }
        }

        loop {
            let line = self.stream.read_raw_line()?;
            if line.starts_with('+') {
                let payload = format!("user={username}\x01auth=Bearer {access_token}\x01\x01");
                let encoded = base64_encode(payload.as_bytes());
                self.stream.write_all(encoded.as_bytes())?;
                self.stream.write_all(b"\r\n")?;
                continue;
            }
            if line.starts_with(&tag) {
                if line.to_ascii_uppercase().contains(" OK") {
                    return Ok(());
                }
                return Err(
                    MailError::imap_auth_rejected(format!("XOAUTH2 rejected: {line}")).into(),
                );
            }
        }
    }

    /// Idle the connection until the server reports a change or the read
    /// timeout expires (the transport carries a read timeout).
    pub fn idle_once(&mut self) -> Result<bool> {
        let tag = self.next_tag();
        self.stream
            .write_all(format!("{tag} IDLE\r\n").as_bytes())?;
        let continuation = self.stream.read_raw_line()?;
        if !continuation.starts_with('+') {
            return Err(anyhow!("IDLE was not accepted: {continuation}"));
        }

        let changed = match self.stream.read_raw_line() {
            Ok(line) => {
                if line.starts_with(&tag) {
                    return Ok(false);
                }
                true
            }
            Err(_) => false,
        };

        self.stream.write_all(b"DONE\r\n")?;
        loop {
            let line = self.stream.read_raw_line()?;
            if line.starts_with(&tag) {
                break;
            }
        }
        Ok(changed)
    }

    /// Upgrade a cleartext connection with STARTTLS.
    pub fn starttls(&mut self) -> Result<()> {
        self.run(CommandBody::StartTLS)?.require_ok("STARTTLS")?;
        Ok(())
    }

    /// End the session politely.
    pub fn logout(&mut self) {
        let _ = self.run(CommandBody::Logout);
    }
}

/// SASL PLAIN payload: `\0user\0password`.
fn base64_plain(username: &str, password: &str) -> String {
    let mut payload = Vec::new();
    payload.push(0);
    payload.extend_from_slice(username.as_bytes());
    payload.push(0);
    payload.extend_from_slice(password.as_bytes());
    base64_encode(&payload)
}

fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Parse a tag written by this client.
pub fn tag_for(index: u64) -> String {
    format!("a{index:04}")
}

/// Validate that a string is usable as a tag.
#[cfg(test)]
pub fn valid_tag(value: &str) -> bool {
    imap_types::core::Tag::validate(value.as_bytes()).is_ok()
}

#[cfg(test)]
mod tests {
    use super::{base64_plain, tag_for, valid_tag};

    #[test]
    fn tags_are_padded_and_valid() {
        assert_eq!(tag_for(1), "a0001");
        assert_eq!(tag_for(12345), "a12345");
        assert!(valid_tag(&tag_for(7)));
        assert!(!valid_tag(""));
        assert!(!valid_tag("a b"));
    }

    #[test]
    fn sasl_plain_payload_is_nul_separated() {
        use base64::Engine;
        let encoded = base64_plain("alice", "secret");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .expect("base64");
        assert_eq!(decoded, b"\0alice\0secret");
    }
}
