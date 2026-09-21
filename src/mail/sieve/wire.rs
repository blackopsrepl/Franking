/*! ManageSieve wire-format helpers. */

use std::io::{Read, Write};
use std::net::TcpStream;

use anyhow::{Context, Result};

use super::SieveScript;

pub(super) fn assemble(name: &str, args: &[String]) -> String {
    let mut line = name.to_string();
    for arg in args {
        line.push(' ');
        line.push_str(arg);
    }
    line
}

/// Parse `"name" ACTIVE` or `"name"` into a script descriptor.
pub(super) fn parse_script(line: &str) -> SieveScript {
    let name = quoted_value(line).unwrap_or_default();
    let active = line
        .split_whitespace()
        .any(|token| token.eq_ignore_ascii_case("ACTIVE"));
    SieveScript { name, active }
}

/// First quoted string in a response line.
pub(super) fn quoted_value(line: &str) -> Option<String> {
    let start = line.find('"')? + 1;
    let rest = &line[start..];
    let end = rest.find('"')?;
    let value = &rest[..end];
    Some(value.replace("\\\"", "\"").replace("\\\\", "\\"))
}

/// Quote a string for the wire.
pub(super) fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Literal size when a line ends with `{n}` or `{n+}`.
pub(super) fn literal_size(line: &str) -> Option<usize> {
    let trimmed = line.trim_end();
    let inner = trimmed.strip_suffix('}')?;
    let digits = inner.strip_suffix('+').unwrap_or(inner);
    let start = digits.rfind('{')?;
    digits[start + 1..].parse().ok()
}

/// Connection transport, upgraded in place when STARTTLS succeeds.
pub(super) enum Transport {
    Tcp(TcpStream),
    Tls(Box<native_tls::TlsStream<TcpStream>>),
}

impl Read for Transport {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Transport::Tcp(stream) => stream.read(buf),
            Transport::Tls(stream) => stream.read(buf),
        }
    }
}

impl Write for Transport {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Transport::Tcp(stream) => stream.write(buf),
            Transport::Tls(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Transport::Tcp(stream) => stream.flush(),
            Transport::Tls(stream) => stream.flush(),
        }
    }
}

/// Open a TLS session over an existing TCP stream.
pub(super) fn tls_connect(
    host: &str,
    stream: TcpStream,
) -> Result<native_tls::TlsStream<TcpStream>> {
    let connector = native_tls::TlsConnector::new().context("TLS init")?;
    connector
        .connect(host, stream)
        .context("TLS handshake with ManageSieve server")
}
