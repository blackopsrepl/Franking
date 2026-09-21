/*! Interactive setup wizard and inventory display. */

use std::io::{self, Write};

use anyhow::{anyhow, bail, Context, Result};

use crate::mail::oauth;
use crate::mail::types::Account;

use super::accounts::{configure_icloud_account, configure_password_account};
use super::config::mail_service;
use super::oauth_setup::configure_oauth_account;

pub fn run_wizard() -> Result<Option<String>> {
    println!("╔════════════════════════════════════════════╗");
    println!("║     SolverForge Mail - Account Setup      ║");
    println!("╚════════════════════════════════════════════╝");
    println!();

    loop {
        let inventory = load_inventory()?;
        print_inventory(&inventory);

        println!();
        println!("Select an action:");
        println!("1) Add or update a generic IMAP/SMTP account");
        println!("2) Add or update an iCloud account");
        println!("3) Add or update a Gmail OAuth account");
        println!("4) Add or update an Outlook OAuth account");
        println!("5) Launch SolverForge Mail with the first working account");
        println!("6) Exit");
        println!();

        match prompt("Choice [1-6]: ")? {
            choice if choice == "1" => {
                if let Err(error) = configure_password_account(&inventory.accounts) {
                    println!("{error}");
                }
            }
            choice if choice == "2" => {
                if let Err(error) = configure_icloud_account(&inventory.accounts) {
                    println!("{error}");
                }
            }
            choice if choice == "3" => {
                if let Err(error) =
                    configure_oauth_account(&inventory.accounts, &oauth::GMAIL_PROVIDER)
                {
                    println!("{error}");
                }
            }
            choice if choice == "4" => {
                if let Err(error) =
                    configure_oauth_account(&inventory.accounts, &oauth::OUTLOOK_PROVIDER)
                {
                    println!("{error}");
                }
            }
            choice if choice == "5" => {
                return Ok(Some(first_working_account(&inventory.accounts)?))
            }
            choice if choice == "6" || choice.is_empty() => return Ok(None),
            _ => println!("Invalid choice."),
        }

        println!();
    }
}

pub fn print_account_status() -> Result<()> {
    let inventory = load_inventory()?;
    print_inventory(&inventory);

    if inventory.accounts.is_empty() {
        if let Some(message) = inventory.banner.as_ref() {
            bail!("{message}");
        }
        bail!("No configured accounts found.");
    }

    Ok(())
}

struct Inventory {
    accounts: Vec<Account>,
    banner: Option<String>,
}

fn load_inventory() -> Result<Inventory> {
    let accounts = mail_service()
        .list_accounts()
        .map_err(|error| anyhow!(error.to_string()))?;
    let banner = if selectable_remote_accounts(&accounts).is_empty() {
        Some(
            "No configured remote accounts found yet. Add a generic IMAP/SMTP, iCloud, Gmail, or Outlook account to use the native app-owned mail engine."
                .to_string(),
        )
    } else {
        None
    };
    Ok(Inventory { accounts, banner })
}

fn print_inventory(inventory: &Inventory) {
    println!("Current account status:");
    println!("----------------------");

    if let Some(message) = inventory.banner.as_ref() {
        println!("{message}");
        return;
    }

    for account in &inventory.accounts {
        println!(
            "{} {}",
            pad_account(&account.name),
            describe_account_status(account)
        );
    }
}

fn pad_account(name: &str) -> String {
    format!("{name:20}:")
}

fn describe_account_status(account: &Account) -> String {
    match mail_service().probe_account(&account.name) {
        Ok(()) => "✓ Working".to_string(),
        Err(error) => format!("✗ {error}"),
    }
}

fn first_working_account(accounts: &[Account]) -> Result<String> {
    let mut candidates = accounts.to_vec();
    crate::mail::types::sort_accounts(&mut candidates);

    for account in &candidates {
        if mail_service().probe_account(&account.name).is_ok() {
            println!("Launching SolverForge Mail with account {}.", account.name);
            return Ok(account.name.clone());
        }
    }

    bail!("No working account found. Fix the reported backend/auth issues first.")
}

pub(super) fn choose_or_create_remote_account(
    accounts: &[Account],
    prompt_text: &str,
) -> Result<String> {
    let selectable = selectable_remote_accounts(accounts);
    if selectable.is_empty() {
        return prompt_nonempty(&format!("{prompt_text} name: "));
    }

    for (index, account) in selectable.iter().enumerate() {
        let default_marker = if account.default { " (default)" } else { "" };
        println!(
            "  {}) {} [{}]{}",
            index + 1,
            account.name,
            account.backend,
            default_marker
        );
    }
    println!("  0) Enter a new account name");
    println!();

    let raw = prompt(&format!("{prompt_text} [0-{}]: ", selectable.len()))?;
    if raw == "0" {
        return prompt_nonempty("New account name: ");
    }

    let choice: usize = raw.parse().context("invalid account selection")?;
    let account = selectable
        .get(choice.saturating_sub(1))
        .copied()
        .ok_or_else(|| anyhow!("account selection out of range"))?;
    Ok(account.name.clone())
}

pub(super) fn prompt(label: &str) -> Result<String> {
    print!("{label}");
    io::stdout().flush().context("failed to flush stdout")?;
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .context("failed to read from stdin")?;
    Ok(line.trim().to_string())
}

pub(super) fn prompt_nonempty(label: &str) -> Result<String> {
    let value = prompt(label)?;
    if value.is_empty() {
        bail!("input is required");
    }
    Ok(value)
}

pub(super) fn prompt_required_with_default(label: &str, default: Option<&str>) -> Result<String> {
    let label = match default {
        Some(value) if !value.is_empty() => format!("{label}[{value}] "),
        _ => label.to_string(),
    };
    let value = prompt(&label)?;
    if value.is_empty() {
        default
            .map(str::to_string)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("input is required"))
    } else {
        Ok(value)
    }
}

pub(super) fn prompt_port_with_default(
    label: &str,
    default: Option<u16>,
    fallback: u16,
) -> Result<u16> {
    let default = default.unwrap_or(fallback);
    let value = prompt(&format!("{label}[{default}] "))?;
    if value.is_empty() {
        return Ok(default);
    }
    value.parse().context("invalid port")
}

pub(super) fn prompt_password(label: &str) -> Result<String> {
    let value = rpassword::prompt_password(label).context("failed to read password")?;
    if value.is_empty() {
        bail!("password is required");
    }
    Ok(value)
}

pub(super) fn prompt_optional_password(label: &str) -> Result<Option<String>> {
    let value = rpassword::prompt_password(label).context("failed to read password")?;
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

pub(super) fn print_probe_result(account: &str, _backend: Option<&str>) {
    match mail_service().probe_account(account) {
        Ok(()) => println!("✓ {} is working.", account),
        Err(error) => println!("Stored definition for {account}. Current runtime status: {error}"),
    }
}

pub(super) fn selectable_remote_accounts(accounts: &[Account]) -> Vec<&Account> {
    accounts
        .iter()
        .filter(|account| !account.backend.eq_ignore_ascii_case("maildir"))
        .collect()
}
