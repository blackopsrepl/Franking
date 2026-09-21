/*! Transport helpers shared by the legacy and codec-based clients. */

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use native_tls::TlsConnector;

use crate::mail::errors::{MailError, MailResult};

pub const NETWORK_TIMEOUT: Duration = Duration::from_secs(60);

/// A duplex stream used by the app-owned client.
pub trait ReadWrite: Read + Write + Send {}
impl<T: Read + Write + Send> ReadWrite for T {}

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
