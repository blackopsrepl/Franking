/*! ManageSieve client (RFC 5804).
Talks to a Sieve script server to list, fetch, replace, activate, and delete
server-side filter scripts. Supports implicit TLS, STARTTLS, and plaintext,
with SASL PLAIN authentication. */

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;

mod wire;
use wire::{assemble, literal_size, parse_script, quote, quoted_value, tls_connect, Transport};

/// How the ManageSieve connection is protected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SieveSecurity {
    /// TLS from the first byte.
    Tls,
    /// Plaintext greeting followed by STARTTLS (the usual port 4190 case).
    StartTls,
    /// No transport security; for local test servers only.
    Plain,
}

/// Connection parameters for a ManageSieve server.
#[derive(Debug, Clone)]
pub struct SieveConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub security: SieveSecurity,
}

/// One server-side script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SieveScript {
    pub name: String,
    pub active: bool,
}

/// An authenticated ManageSieve session.
pub struct SieveClient {
    stream: BufReader<Transport>,
    host: String,
    capabilities: Vec<String>,
}

/// One response line, literal body, or final status.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Response {
    Line(String),
    Literal(Vec<u8>),
    Status(String),
}

impl SieveClient {
    /// Connect and authenticate.
    pub fn connect(config: &SieveConfig) -> Result<Self> {
        let tcp = TcpStream::connect((config.host.as_str(), config.port))
            .with_context(|| format!("connect to {}:{}", config.host, config.port))?;
        tcp.set_nodelay(true).ok();

        let transport = match config.security {
            SieveSecurity::Tls => Transport::Tls(Box::new(tls_connect(&config.host, tcp)?)),
            SieveSecurity::StartTls | SieveSecurity::Plain => Transport::Tcp(tcp),
        };
        let mut client = SieveClient {
            stream: BufReader::new(transport),
            host: config.host.clone(),
            capabilities: Vec::new(),
        };

        client.capabilities = client.read_greeting()?;

        if config.security == SieveSecurity::StartTls {
            if !client.supports_starttls() {
                bail!("server does not advertise STARTTLS");
            }
            client.command("STARTTLS", &[])?;
            client.upgrade_to_tls()?;
            client.capabilities = client.read_greeting()?;
        }

        client.authenticate(&config.username, &config.password)?;
        Ok(client)
    }

    /// Capability lines as advertised by the server.
    pub fn capabilities(&self) -> &[String] {
        &self.capabilities
    }

    /// Whether the server advertises STARTTLS.
    pub fn supports_starttls(&self) -> bool {
        self.capabilities
            .iter()
            .any(|line| line.to_ascii_uppercase().contains("STARTTLS"))
    }

    /// List scripts with their active state.
    pub fn scripts(&mut self) -> Result<Vec<SieveScript>> {
        let responses = self.command("LISTSCRIPTS", &[])?;
        Ok(responses
            .into_iter()
            .filter_map(|response| match response {
                Response::Line(line) => Some(parse_script(&line)),
                _ => None,
            })
            .collect())
    }

    /// Fetch a script's source.
    pub fn get_script(&mut self, name: &str) -> Result<String> {
        let responses = self.command("GETSCRIPT", &[quote(name)])?;
        for response in responses {
            match response {
                Response::Literal(bytes) => return Ok(String::from_utf8_lossy(&bytes).to_string()),
                Response::Line(line) => {
                    if let Some(value) = quoted_value(&line) {
                        return Ok(value);
                    }
                }
                Response::Status(_) => {}
            }
        }
        bail!("server returned no script body for {name}")
    }

    /// Replace (or create) a script.
    pub fn put_script(&mut self, name: &str, body: &str) -> Result<()> {
        self.literal_command("PUTSCRIPT", &[quote(name)], body.as_bytes())?;
        Ok(())
    }

    /// Activate a script, or deactivate all when `name` is `None`.
    pub fn set_active(&mut self, name: Option<&str>) -> Result<()> {
        let argument = match name {
            Some(name) => quote(name),
            None => "\"\"".to_string(),
        };
        self.command("SETACTIVE", &[argument])?;
        Ok(())
    }

    /// Delete a script.
    pub fn delete_script(&mut self, name: &str) -> Result<()> {
        self.command("DELETESCRIPT", &[quote(name)])?;
        Ok(())
    }

    /// Rename a script.
    pub fn rename_script(&mut self, from: &str, to: &str) -> Result<()> {
        self.command("RENAMESCRIPT", &[quote(from), quote(to)])?;
        Ok(())
    }

    /// Check that a script compiles.
    pub fn check_script(&mut self, body: &str) -> Result<()> {
        self.literal_command("CHECKSCRIPT", &[], body.as_bytes())?;
        Ok(())
    }

    /// End the session politely.
    pub fn logout(&mut self) {
        let _ = self.send_line("LOGOUT");
    }

    fn upgrade_to_tls(&mut self) -> Result<()> {
        let Transport::Tcp(tcp) = self.stream.get_ref() else {
            bail!("connection is already encrypted");
        };
        let tcp = tcp.try_clone().context("clone ManageSieve socket")?;
        let tls = tls_connect(&self.host, tcp)?;
        self.stream = BufReader::new(Transport::Tls(Box::new(tls)));
        Ok(())
    }

    fn authenticate(&mut self, username: &str, password: &str) -> Result<()> {
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

    fn command(&mut self, name: &str, args: &[String]) -> Result<Vec<Response>> {
        self.send_line(&assemble(name, args))?;
        self.read_until_status()
    }

    fn literal_command(
        &mut self,
        name: &str,
        args: &[String],
        body: &[u8],
    ) -> Result<Vec<Response>> {
        let line = format!("{} {{{}+}}", assemble(name, args), body.len());
        self.send_line(&line)?;
        self.stream.get_mut().write_all(body)?;
        self.stream.get_mut().flush()?;
        self.read_until_status()
    }

    fn send_line(&mut self, line: &str) -> Result<()> {
        let stream = self.stream.get_mut();
        stream.write_all(line.as_bytes())?;
        stream.write_all(b"\r\n")?;
        stream.flush()?;
        Ok(())
    }

    fn read_greeting(&mut self) -> Result<Vec<String>> {
        let responses = self.read_until_status()?;
        Ok(responses
            .into_iter()
            .filter_map(|response| match response {
                Response::Line(line) => Some(line),
                _ => None,
            })
            .collect())
    }

    fn read_until_status(&mut self) -> Result<Vec<Response>> {
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

    fn read_line(&mut self) -> Result<String> {
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

    fn read_exact(&mut self, size: usize) -> Result<Vec<u8>> {
        let mut buffer = vec![0u8; size];
        self.stream.read_exact(&mut buffer)?;
        Ok(buffer)
    }
}

#[cfg(test)]
mod tests;
