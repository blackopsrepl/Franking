/*! ManageSieve session internals: authentication, command framing, and response
reading. A child module, so the client's fields stay private to this module. */

use std::io::{BufRead, BufReader, Read, Write};

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;

use super::wire::{assemble, literal_size, quote, tls_connect, Transport};
use super::{Response, SieveClient};

impl SieveClient {
    pub(super) fn upgrade_to_tls(&mut self) -> Result<()> {
        let Transport::Tcp(tcp) = self.stream.get_ref() else {
            bail!("connection is already encrypted");
        };
        let tcp = tcp.try_clone().context("clone ManageSieve socket")?;
        let tls = tls_connect(&self.host, tcp)?;
        self.stream = BufReader::new(Transport::Tls(Box::new(tls)));
        Ok(())
    }

    pub(super) fn authenticate(&mut self, username: &str, password: &str) -> Result<()> {
        if !self.capabilities.iter().any(|line| {
            line.to_ascii_uppercase().contains("SASL")
                && line.to_ascii_uppercase().contains("PLAIN")
        }) {
            bail!("server does not advertise SASL PLAIN");
        }
        let mut payload = vec![0u8];
        payload.extend_from_slice(username.as_bytes());
        payload.push(0);
        payload.extend_from_slice(password.as_bytes());
        let encoded = base64::engine::general_purpose::STANDARD.encode(&payload);
        self.command("AUTHENTICATE", &["\"PLAIN\"".to_string(), quote(&encoded)])?;
        Ok(())
    }

    pub(super) fn command(&mut self, name: &str, args: &[String]) -> Result<Vec<Response>> {
        self.send_line(&assemble(name, args))?;
        self.read_until_status()
    }

    pub(super) fn literal_command(
        &mut self,
        name: &str,
        args: &[String],
        body: &[u8],
    ) -> Result<Vec<Response>> {
        // A non-synchronizing literal is only allowed when the server said it
        // accepts one (RFC 5804); otherwise the literal is synchronizing.
        let non_synchronizing = self.supports_literal_plus();
        let marker = if non_synchronizing { "+" } else { "" };
        let line = format!("{} {{{}{}}}", assemble(name, args), body.len(), marker);
        self.send_line(&line)?;

        if !non_synchronizing {
            // RFC 5804 has the server answer a synchronizing literal with a
            // continuation line, but Dovecot Pigeonhole takes the literal
            // without one. Probe briefly: a compliant server answers at once,
            // and a lenient server is already waiting for the payload.
            if let Some(line) = self.probe_continuation()? {
                if !line.starts_with('+') {
                    bail!("the server refused the literal: {line}");
                }
            }
        }

        self.stream.get_mut().write_all(body)?;
        // The literal is part of the command line, which still needs its
        // terminating CRLF; without it the server waits for the rest.
        self.stream.get_mut().write_all(b"\r\n")?;
        self.stream.get_mut().flush()?;
        self.read_until_status()
    }

    /// Wait briefly for a literal continuation line, if the server sends one.
    fn probe_continuation(&mut self) -> Result<Option<String>> {
        const PROBE: std::time::Duration = std::time::Duration::from_millis(250);

        self.stream
            .get_ref()
            .set_read_timeout(PROBE)
            .context("set the literal probe timeout")?;
        let read = self.read_line();
        let restored = self
            .stream
            .get_ref()
            .set_read_timeout(crate::mail::session::NETWORK_TIMEOUT)
            .context("restore the ManageSieve read timeout");

        match read {
            Ok(line) => {
                restored?;
                Ok(Some(line))
            }
            Err(error) if is_timeout(&error) => Ok(None),
            Err(error) => {
                restored?;
                Err(error)
            }
        }
    }

    /// Whether the server accepts non-synchronizing literals.
    fn supports_literal_plus(&self) -> bool {
        self.capabilities
            .iter()
            .any(|line| line.to_ascii_uppercase().contains("LITERAL+"))
    }

    pub(super) fn send_line(&mut self, line: &str) -> Result<()> {
        let stream = self.stream.get_mut();
        stream.write_all(line.as_bytes())?;
        stream.write_all(b"\r\n")?;
        stream.flush()?;
        Ok(())
    }

    pub(super) fn read_greeting(&mut self) -> Result<Vec<String>> {
        let responses = self.read_until_status()?;
        Ok(responses
            .into_iter()
            .filter_map(|response| match response {
                Response::Line(line) => Some(line),
                _ => None,
            })
            .collect())
    }

    pub(super) fn read_until_status(&mut self) -> Result<Vec<Response>> {
        let mut responses = Vec::new();
        loop {
            let line = self.read_line()?;
            if let Some(size) = literal_size(&line) {
                responses.push(Response::Literal(self.read_exact(size)?));
                continue;
            }
            let trimmed = line.trim_start().to_string();
            let status = trimmed
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_ascii_uppercase();
            if matches!(status.as_str(), "OK" | "NO" | "BYE") {
                if status != "OK" {
                    return Err(anyhow!("ManageSieve command failed: {trimmed}"));
                }
                responses.push(Response::Status(trimmed));
                return Ok(responses);
            }
            responses.push(Response::Line(line));
        }
    }

    pub(super) fn read_line(&mut self) -> Result<String> {
        let mut line = Vec::new();
        let read = self.stream.read_until(b'\n', &mut line)?;
        if read == 0 {
            bail!("ManageSieve server closed the connection");
        }
        while matches!(line.last(), Some(b'\n' | b'\r')) {
            line.pop();
        }
        Ok(String::from_utf8_lossy(&line).to_string())
    }

    pub(super) fn read_exact(&mut self, size: usize) -> Result<Vec<u8>> {
        let mut buffer = vec![0u8; size];
        self.stream.read_exact(&mut buffer)?;
        Ok(buffer)
    }
}

/// Whether an error is a read timeout rather than a protocol failure.
fn is_timeout(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause.downcast_ref::<std::io::Error>().is_some_and(|io| {
            matches!(
                io.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            )
        })
    })
}
