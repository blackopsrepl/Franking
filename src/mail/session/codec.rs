/*! Response reading for the app-owned IMAP command layer.
`imap-codec` supplies the grammar; this module owns the policy around it. The
legacy `imap` crate aborted (or panicked) on an untagged line it could not
parse, which desynchronized the reader; here an unparseable line is counted,
logged, and skipped, and the next valid line still parses. */

use std::io::{BufRead, BufReader, Read, Write};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use imap_codec::decode::Decoder;
use imap_codec::encode::Encoder;
use imap_codec::{CommandCodec, ResponseCodec};
use imap_types::command::Command;
use imap_types::response::{Response, Status};

use crate::mail::errors::{MailError, MailResult};

use super::transport::ReadWrite;

/// Bytes read from the socket at a time.
const READ_CHUNK: usize = 8 * 1024;

/// A tagged completion of a command.
#[derive(Debug)]
pub struct Completion {
    pub status: imap_types::response::StatusKind,
    pub code: Option<imap_types::response::Code<'static>>,
    pub text: String,
}

impl Completion {
    pub fn is_ok(&self) -> bool {
        matches!(self.status, imap_types::response::StatusKind::Ok)
    }

    /// DEBUG-formatted response code, for diagnostics.
    pub fn code_debug(&self) -> String {
        self.code
            .as_ref()
            .map(|code| format!("{code:?}"))
            .unwrap_or_default()
    }
}

/// Everything a command produced: untagged data plus the tagged completion.
#[derive(Debug, Default)]
pub struct CommandOutput {
    /// Untagged data responses.
    pub responses: Vec<Response<'static>>,
    /// Untagged status responses, whose response codes carry UIDVALIDITY,
    /// UIDNEXT, HIGHESTMODSEQ, APPENDUID, COPYUID, and friends.
    pub untagged: Vec<imap_types::response::StatusBody<'static>>,
    pub completion: Option<Completion>,
    /// Lines that did not decode, kept for diagnostics.
    pub skipped: Vec<String>,
}

impl CommandOutput {
    /// Untagged data responses only.
    pub fn data(&self) -> impl Iterator<Item = &Response<'static>> {
        self.responses.iter()
    }

    /// Untagged status text, for diagnostics.
    pub fn untagged_text(&self) -> Vec<String> {
        self.untagged
            .iter()
            .map(|body| body.text.to_string())
            .collect()
    }

    /// First untagged response code matching `wanted`.
    pub fn untagged_code<T>(
        &self,
        wanted: impl Fn(&imap_types::response::Code<'static>) -> Option<T>,
    ) -> Option<T> {
        self.untagged
            .iter()
            .find_map(|body| body.code.as_ref().and_then(&wanted))
    }

    /// First response code from any source matching `wanted`.
    pub fn any_code<T>(
        &self,
        wanted: impl Fn(&imap_types::response::Code<'static>) -> Option<T>,
    ) -> Option<T> {
        if let Some(found) = self.untagged_code(&wanted) {
            return Some(found);
        }
        self.completion
            .as_ref()
            .and_then(|completion| completion.code.as_ref())
            .and_then(&wanted)
    }

    /// Fail unless the server completed the command with OK.
    pub fn require_ok(self, what: &str) -> MailResult<Self> {
        match self.completion.as_ref() {
            Some(completion) if completion.is_ok() => Ok(self),
            Some(completion) => Err(MailError::other(format!(
                "{what} failed: {} {}",
                completion.text,
                completion.code_debug()
            ))),
            None => Err(MailError::connection_dropped(format!(
                "{what} produced no completion; skipped {} undecodable line(s)",
                self.skipped.len()
            ))),
        }
    }
}

/// Reads and writes IMAP over one stream, decoding each response.
pub struct ResponseReader<R: Read + Write> {
    reader: BufReader<R>,
    buffer: Vec<u8>,
    /// Undecodable lines seen so far, newest last (bounded).
    pub skipped: Vec<String>,
}

impl<R: Read + Write> ResponseReader<R> {
    pub fn new(inner: R) -> Self {
        Self {
            reader: BufReader::new(inner),
            buffer: Vec::new(),
            skipped: Vec::new(),
        }
    }

    /// The wrapped stream, for upgrading the transport.
    pub fn into_inner(self) -> R {
        self.reader.into_inner()
    }

    /// Write bytes straight to the socket (the reader only buffers reads).
    pub fn write_all(&mut self, bytes: &[u8]) -> Result<()> {
        self.reader
            .get_mut()
            .write_all(bytes)
            .context("writing to the IMAP server")?;
        self.reader.get_mut().flush().context("flush")?;
        Ok(())
    }

    /// Read raw bytes until at least one more byte is buffered.
    fn fill(&mut self) -> Result<bool> {
        let mut chunk = [0u8; READ_CHUNK];
        let read = self
            .reader
            .read(&mut chunk)
            .context("reading from the IMAP server")?;
        if read == 0 {
            return Ok(false);
        }
        self.buffer.extend_from_slice(&chunk[..read]);
        Ok(true)
    }

    /// Read a whole line (including CRLF) straight from the socket, for use
    /// while a literal is being sent or a continuation is expected.
    /// Adjust the read timeout of the underlying stream.
    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()>
    where
        R: ReadWrite,
    {
        self.reader.get_ref().set_read_timeout(timeout)
    }

    pub fn read_raw_line(&mut self) -> Result<String> {
        let mut line = Vec::new();
        let read = self.reader.read_until(b'\n', &mut line)?;
        if read == 0 {
            return Err(anyhow!("IMAP server closed the connection"));
        }
        Ok(String::from_utf8_lossy(&line).trim_end().to_string())
    }

    /// Decode the next response, skipping lines that do not parse.
    pub fn next_response(&mut self) -> Result<Response<'static>> {
        loop {
            match ResponseCodec::default().decode_static(&self.buffer) {
                Ok((rest, response)) => {
                    let consumed = self.buffer.len() - rest.len();
                    self.buffer.drain(..consumed);
                    return Ok(response);
                }
                // A literal announced but not yet fully buffered: read on.
                Err(imap_codec::decode::ResponseDecodeError::LiteralFound { .. })
                | Err(imap_codec::decode::ResponseDecodeError::Incomplete) => {
                    if !self.fill()? {
                        return Err(anyhow!(
                            "IMAP server closed the connection mid-response: {:?}",
                            String::from_utf8_lossy(&self.buffer)
                        ));
                    }
                }
                Err(imap_codec::decode::ResponseDecodeError::Failed) => {
                    // Skip exactly one line and keep going: an unknown untagged
                    // line must never desynchronize or abort the session.
                    let end = self
                        .buffer
                        .iter()
                        .position(|byte| *byte == b'\n')
                        .map(|index| index + 1);
                    match end {
                        Some(end) => {
                            let line = String::from_utf8_lossy(&self.buffer[..end])
                                .trim_end()
                                .to_string();
                            log_skipped(&line);
                            push_bounded(&mut self.skipped, line);
                            self.buffer.drain(..end);
                        }
                        None => {
                            // Partial line: wait for the rest.
                            if !self.fill()? {
                                return Err(anyhow!(
                                    "IMAP server closed the connection during an undecodable line"
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Run one command and collect its responses up to the tagged completion.
    ///
    /// `tag` must be the tag already written with the command.
    pub fn collect(&mut self, tag: &str) -> Result<CommandOutput> {
        let mut output = CommandOutput::default();
        loop {
            let response = self.next_response()?;
            match response {
                Response::Status(Status::Tagged(tagged)) => {
                    // Correlate by tag instead of asserting: a stale response
                    // from a previous command is a protocol error, not a panic.
                    if tagged.tag.as_ref() != tag {
                        return Err(MailError::other(format!(
                            "IMAP response tag '{}' does not match command tag '{tag}'",
                            tagged.tag.as_ref()
                        ))
                        .into());
                    }
                    output.completion = Some(Completion {
                        status: tagged.body.kind,
                        code: tagged.body.code,
                        text: tagged.body.text.to_string(),
                    });
                    output.skipped = std::mem::take(&mut self.skipped);
                    return Ok(output);
                }
                Response::Status(Status::Untagged(body)) => output.untagged.push(body),
                Response::Status(Status::Bye(bye)) => {
                    return Err(MailError::connection_dropped(format!(
                        "server closed the session: {}",
                        bye.text
                    ))
                    .into());
                }
                // Greeting and any other non-status response terminate a command
                // read only via the tagged completion below.
                other => output.responses.push(other),
            }
        }
    }

    /// Encode a command into wire fragments, tagging it.
    pub fn encode_command(
        &self,
        tag: &str,
        command: Command<'_>,
    ) -> Result<Vec<imap_codec::encode::Fragment>> {
        let _ = CommandCodec::default();
        let tagged = Command::new(tag.to_string(), command.body).context("tagging command")?;
        Ok(CommandCodec::default().encode(&tagged).collect())
    }
}

fn push_bounded(lines: &mut Vec<String>, line: String) {
    const MAX: usize = 32;
    if lines.len() == MAX {
        lines.remove(0);
    }
    lines.push(line);
}

fn log_skipped(line: &str) {
    eprintln!("IMAP: skipping undecodable response line: {line}");
}

#[cfg(test)]
mod tests;
