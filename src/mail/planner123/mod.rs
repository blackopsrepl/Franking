/*! Handing a message's calendar invitation to Planner123.
Planner123 owns calendars, events, and scheduling; this client only knows an
invitation arrived by mail. The hand-off goes through `planner123-cli`, its
JSON automation entrypoint, so the two programs stay independent and the
calendar side keeps its own storage and sync. */

use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;

use super::errors::{MailError, MailResult};

/// Environment variables that name the Planner123 CLI explicitly.
const BINARY_VARS: [&str; 2] = ["FRANKING_PLANNER123_CLI", "PLANNER123_CLI"];

/// What Planner123 reported about an import.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub calendar_name: String,
    pub imported: usize,
    pub skipped: usize,
    /// Events Planner123 imported with reservations, e.g. unsupported data.
    pub warnings: Vec<String>,
}

impl ImportReport {
    /// A one-line summary for the status bar.
    pub fn summary(&self) -> String {
        let mut text = format!(
            "Added {} event(s) to Planner123 ({}).",
            self.imported, self.calendar_name
        );
        if self.skipped > 0 {
            text.push_str(&format!(" {} skipped.", self.skipped));
        }
        if let Some(warning) = self.warnings.first() {
            text.push_str(&format!(" {warning}"));
        }
        text
    }
}

/// The Planner123 command-line client.
#[derive(Debug, Clone)]
pub struct Planner123 {
    binary: PathBuf,
    /// Calendar to import into; `None` asks Planner123 for its default.
    calendar_id: Option<String>,
    /// Default timezone for events that do not carry one.
    timezone: String,
}

impl Planner123 {
    /// Locate the CLI, honouring an explicit path before `PATH`.
    pub fn discover(calendar_id: Option<String>, timezone: String) -> Option<Self> {
        let binary = BINARY_VARS
            .iter()
            .filter_map(std::env::var_os)
            .map(PathBuf::from)
            .find(|path| path.is_file())
            .or_else(|| which("planner123-cli"))?;
        Some(Self {
            binary,
            calendar_id,
            timezone,
        })
    }

    /// Import an iCalendar payload, returning what Planner123 reported.
    pub fn import_invitation(&self, ics: &[u8]) -> MailResult<ImportReport> {
        let calendar_id = match self.calendar_id.clone() {
            Some(id) => id,
            None => self.default_calendar_id()?,
        };
        let path = write_temporary_ics(ics)?;
        let output = Command::new(&self.binary)
            .args([
                "ical",
                "import",
                "--calendar-id",
                &calendar_id,
                "--path",
                &path.to_string_lossy(),
                "--timezone",
                &self.timezone,
            ])
            .output()
            .map_err(|error| {
                MailError::other(format!("cannot run {}: {error}", self.binary.display()))
            });
        let _ = std::fs::remove_file(&path);
        let output = output?;

        if !output.status.success() {
            return Err(MailError::other(format!(
                "Planner123 refused the import: {}",
                first_line(&String::from_utf8_lossy(&output.stderr))
            )));
        }
        parse_import(&String::from_utf8_lossy(&output.stdout))
    }

    /// The calendar Planner123 uses when no target was chosen.
    pub fn default_calendar_id(&self) -> MailResult<String> {
        let output = Command::new(&self.binary)
            .args(["calendars", "list"])
            .output()
            .map_err(|error| {
                MailError::other(format!("cannot run {}: {error}", self.binary.display()))
            })?;
        if !output.status.success() {
            return Err(MailError::other(format!(
                "Planner123 cannot list calendars: {}",
                first_line(&String::from_utf8_lossy(&output.stderr))
            )));
        }
        parse_default_calendar(&String::from_utf8_lossy(&output.stdout))
    }
}

/// Write the invitation beside other temporary files, for the CLI to read.
fn write_temporary_ics(ics: &[u8]) -> MailResult<PathBuf> {
    let dir = std::env::temp_dir();
    let name = format!("franking-invite-{}.ics", std::process::id());
    let path = dir.join(name);
    std::fs::write(&path, ics)
        .map_err(|error| MailError::io(format!("cannot write {}: {error}", path.display())))?;
    Ok(path)
}

/// The CLI's `ical import` envelope: `{status, data: {...}}`.
#[derive(Deserialize)]
struct ImportEnvelope {
    status: String,
    data: Option<ImportData>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct ImportData {
    #[serde(default)]
    calendar_name: String,
    #[serde(default)]
    imported: usize,
    #[serde(default)]
    skipped: usize,
    #[serde(default)]
    warnings: Vec<String>,
}

fn parse_import(stdout: &str) -> MailResult<ImportReport> {
    let envelope: ImportEnvelope = serde_json::from_str(stdout)
        .map_err(|error| MailError::other(format!("Planner123 sent no JSON report: {error}")))?;
    if envelope.status != "ok" {
        return Err(MailError::other(format!(
            "Planner123 reported {}: {}",
            envelope.status,
            envelope.error.unwrap_or_default()
        )));
    }
    let data = envelope
        .data
        .ok_or_else(|| MailError::other("Planner123 reported no import data"))?;
    Ok(ImportReport {
        calendar_name: data.calendar_name,
        imported: data.imported,
        skipped: data.skipped,
        warnings: data.warnings,
    })
}

/// The CLI's `calendars list` envelope; ids are UUIDs.
#[derive(Deserialize)]
struct CalendarEnvelope {
    data: Vec<CalendarEntry>,
}

#[derive(Deserialize)]
struct CalendarEntry {
    id: String,
    #[serde(default)]
    is_default: bool,
}

fn parse_default_calendar(stdout: &str) -> MailResult<String> {
    let envelope: CalendarEnvelope = serde_json::from_str(stdout)
        .map_err(|error| MailError::other(format!("Planner123 sent no calendar list: {error}")))?;
    envelope
        .data
        .iter()
        .find(|calendar| calendar.is_default)
        .or_else(|| envelope.data.first())
        .map(|calendar| calendar.id.clone())
        .ok_or_else(|| MailError::other("Planner123 has no calendar to import into"))
}

/// The first non-empty line of a command's output.
fn first_line(text: &str) -> String {
    text.lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(text)
        .trim()
        .to_string()
}

/// Find a binary on `PATH` without pulling in a dependency for it.
fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
#[cfg(test)]
mod tests;

#[cfg(test)]
mod cli_tests;
