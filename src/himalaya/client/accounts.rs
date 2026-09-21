/*! Account listing, probing, and configuration. */

use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::himalaya::config::{global_args, himalaya_bin};
use crate::himalaya::types::*;

use super::exec::run;
use super::messages::list_folders;

pub fn list_accounts() -> Result<Vec<Account>> {
    let mut args = global_args();
    args.extend(["account".to_string(), "list".to_string()]);
    let output = run(&args)?;
    let accounts: Vec<Account> =
        serde_json::from_str(&output).context("failed to parse account list")?;
    Ok(accounts)
}

/// List configured account names only.
pub fn list_account_names() -> Result<Vec<String>> {
    Ok(list_accounts()?
        .into_iter()
        .map(|account| account.name)
        .collect())
}

/// Probe whether an account can list folders successfully.
pub fn probe_account(account: &str) -> Result<()> {
    list_folders(Some(account)).map(|_| ())
}

/// Run Himalaya's interactive account configuration flow.
pub fn configure_account(account: &str) -> Result<()> {
    let bin = himalaya_bin()?;
    let status = Command::new(&bin)
        .args(["account", "configure", account])
        .status()
        .with_context(|| {
            format!(
                "failed to execute: {} account configure {}",
                bin.display(),
                account
            )
        })?;

    if !status.success() {
        bail!("himalaya error: account configure exited with {}", status);
    }

    Ok(())
}
