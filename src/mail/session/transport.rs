/*! Transport helpers for the app-owned client. */

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use native_tls::{TlsConnector, TlsStream};

use crate::mail::errors::{MailError, MailResult};

pub const NETWORK_TIMEOUT: Duration = Duration::from_secs(60);

/// A duplex stream used by the app-owned client.
///
/// The read timeout is part of the interface because IDLE must honour the
/// caller's deadline rather than the transport's default.
pub trait ReadWrite: Read + Write + Send {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()>;
}

impl ReadWrite for TcpStream {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        TcpStream::set_read_timeout(self, timeout)
    }
}

impl ReadWrite for Box<dyn ReadWrite> {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        (**self).set_read_timeout(timeout)
    }
}

impl ReadWrite for TlsStream<TcpStream> {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        self.get_ref().set_read_timeout(timeout)
    }
}

pub fn connect_tcp(host: &str, port: u16) -> MailResult<TcpStream> {
    let stream = TcpStream::connect((host, port)).map_err(|err| {
        if err.kind() == std::io::ErrorKind::TimedOut {
            MailError::transport_timeout(err.to_string())
        } else {
            MailError::io(err.to_string())
        }
    })?;
    stream
        .set_read_timeout(Some(NETWORK_TIMEOUT))
        .map_err(|err| MailError::io(err.to_string()))?;
    stream
        .set_write_timeout(Some(NETWORK_TIMEOUT))
        .map_err(|err| MailError::io(err.to_string()))?;
    Ok(stream)
}

pub fn tls_connector() -> MailResult<TlsConnector> {
    TlsConnector::builder()
        .build()
        .map_err(|err| MailError::tls_failure(err.to_string()))
}
