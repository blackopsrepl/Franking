/*! Process execution helpers. */

use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

use crate::himalaya::config::himalaya_bin;

// Run a himalaya command and return its stdout.
pub(super) fn run(args: &[String]) -> Result<String> {
    let bin = himalaya_bin()?;
    let output = Command::new(&bin)
        .args(args)
        .output()
        .with_context(|| format!("failed to execute: {} {}", bin.display(), args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let msg = if stderr.is_empty() {
            stdout.to_string()
        } else {
            stderr.to_string()
        };
        anyhow::bail!("himalaya error: {}", msg.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/* Run a himalaya command, writing `input` to the child's stdin.
himalaya's `template send` checks `io::stdin().is_terminal()` and reads
from stdin when it is not a terminal (i.e. when spawned as a subprocess). */
pub(super) fn run_with_stdin(args: &[String], input: &str) -> Result<String> {
    let bin = himalaya_bin()?;
    let mut child = Command::new(&bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn: {} {}", bin.display(), args.join(" ")))?;

    // Write the template to stdin, then close it so the child sees EOF.
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input.as_bytes())
            .context("failed to write template to himalaya stdin")?;
        // stdin is dropped here, closing the pipe
    }

    let output = child
        .wait_with_output()
        .context("failed to wait for himalaya")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let msg = if stderr.is_empty() {
            stdout.to_string()
        } else {
            stderr.to_string()
        };
        anyhow::bail!("himalaya error: {}", msg.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// Build the shell command string for compose/reply/forward operations
// that need to shell out to $EDITOR.
pub(super) fn editor_command(args: &[String]) -> Result<String> {
    let bin = himalaya_bin()?;
    let mut parts = vec![bin.display().to_string()];
    parts.extend(args.iter().cloned());
    Ok(parts.join(" "))
}
