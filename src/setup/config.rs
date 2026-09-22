/*! Setup persistence and secret access helpers. */

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;

use anyhow::{bail, Context, Result};

use crate::db;
use crate::mail::account_store::{self, AccountConfig, AccountRecord, OauthStateConfig};
use crate::mail::{default_mail_service, MailService};

pub(super) fn mail_service() -> Arc<dyn MailService> {
    default_mail_service()
}

pub fn load_account_record(name: &str) -> Result<Option<AccountRecord>> {
    let conn = db::open()?;
    account_store::get_account(&conn, name)
}

pub fn save_account_config(config: &AccountConfig) -> Result<()> {
    let conn = db::open()?;
    account_store::upsert_account(&conn, config)
}

pub fn load_oauth_state(name: &str) -> Result<Option<account_store::OauthState>> {
    let conn = db::open()?;
    account_store::get_oauth_state(&conn, name)
}

pub fn save_oauth_state(account_name: &str, config: &OauthStateConfig) -> Result<()> {
    let conn = db::open()?;
    account_store::upsert_oauth_state(&conn, account_name, config)
}

pub fn secret_service_id(account_name: &str, protocol: &str) -> String {
    format!(
        "{}/{account_name}/{protocol}",
        crate::brand::KEYRING_NAMESPACE
    )
}

pub fn store_secret(label: &str, service: &str, username: &str, password: &str) -> Result<()> {
    let mut child = Command::new("secret-tool")
        .args([
            "store",
            "--label",
            label,
            "service",
            service,
            "username",
            username,
            "application",
            crate::brand::KEYRING_APPLICATION,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to execute secret-tool for service {}", service))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(password.as_bytes())
            .with_context(|| format!("failed to write secret for service {}", service))?;
    }

    let output = child
        .wait_with_output()
        .context("failed to wait for secret-tool")?;

    if !output.status.success() {
        bail!(
            "secret-tool failed for service {}: {}",
            service,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(())
}

pub fn lookup_secret(service: &str, username: &str) -> Result<String> {
    let output = Command::new("secret-tool")
        .args([
            "lookup",
            "service",
            service,
            "username",
            username,
            "application",
            crate::brand::KEYRING_APPLICATION,
        ])
        .output()
        .with_context(|| format!("failed to execute secret-tool for service {}", service))?;

    if !output.status.success() {
        bail!(
            "secret-tool lookup failed for service {}: {}",
            service,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let secret = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if secret.is_empty() {
        bail!("no secret found for service {}", service);
    }

    Ok(secret)
}

pub(super) fn authinfo_gpg_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".authinfo.gpg")
}

pub(super) fn rewrite_authinfo_gpg(email: &str, password: &str, recipient: &str) -> Result<()> {
    let authinfo = authinfo_gpg_path();
    let decrypted = Command::new("gpg")
        .args(["-q", "--for-your-eyes-only", "-d"])
        .arg(&authinfo)
        .output()
        .with_context(|| format!("failed to read {}", authinfo.display()))?;

    let mut lines = if decrypted.status.success() {
        String::from_utf8_lossy(&decrypted.stdout)
            .lines()
            .filter(|line| !line.contains("imap.mail.me.com") && !line.contains("smtp.mail.me.com"))
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    lines.push(format!(
        "machine imap.mail.me.com login {} password {}",
        email, password
    ));
    lines.push(format!(
        "machine smtp.mail.me.com login {} password {}",
        email, password
    ));

    let tmp = std::env::temp_dir().join(format!("franking-authinfo-{}", std::process::id()));
    fs::write(&tmp, lines.join("\n") + "\n")
        .with_context(|| format!("failed to write {}", tmp.display()))?;

    let status = Command::new("gpg")
        .args(["--batch", "--yes", "-e", "-r", recipient])
        .arg(&tmp)
        .status()
        .context("failed to execute gpg")?;
    if !status.success() {
        let _ = fs::remove_file(&tmp);
        bail!("gpg encryption failed");
    }

    let encrypted = tmp.with_extension("gpg");
    fs::rename(&encrypted, &authinfo).with_context(|| {
        format!(
            "failed to move encrypted authinfo from {} to {}",
            encrypted.display(),
            authinfo.display()
        )
    })?;
    let _ = fs::remove_file(&tmp);
    Ok(())
}
