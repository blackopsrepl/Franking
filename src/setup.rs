use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};

use crate::db;
use crate::himalaya::client;
use crate::mail::account_store::{self, AccountConfig, AccountRecord};
use crate::mail::types::Account;
use crate::mail::{app_owned_remote_transport_available, default_mail_service, MailService};

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
        if app_owned_remote_transport_available() {
            println!("1) Add or update a generic IMAP/SMTP account");
            println!("2) Add or update an iCloud account");
        } else {
            println!("1) Generic IMAP/SMTP setup (pending native transport)");
            println!("2) iCloud setup (pending native transport)");
        }
        println!("3) Run the temporary OAuth browser flow");
        println!("4) Launch SolverForge Mail with the first working account");
        println!("5) Exit");
        println!();

        match prompt("Choice [1-5]: ")? {
            choice if choice == "1" => {
                let result = if app_owned_remote_transport_available() {
                    configure_password_account(&inventory.accounts)
                } else {
                    bail_app_owned_remote_setup()
                };
                if let Err(error) = result {
                    println!("{error}");
                }
            }
            choice if choice == "2" => {
                let result = if app_owned_remote_transport_available() {
                    configure_icloud_account(&inventory.accounts)
                } else {
                    bail_app_owned_remote_setup()
                };
                if let Err(error) = result {
                    println!("{error}");
                }
            }
            choice if choice == "3" => configure_oauth(Some(&inventory.accounts))?,
            choice if choice == "4" => {
                return Ok(Some(first_working_account(&inventory.accounts)?))
            }
            choice if choice == "5" || choice.is_empty() => return Ok(None),
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
    let banner = if should_show_bootstrap_only(&accounts) {
        Some(
            "No configured remote accounts found yet. Run the temporary OAuth bootstrap flow or add a legacy Himalaya-backed account until native IMAP/SMTP transport lands."
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

fn configure_password_account(accounts: &[Account]) -> Result<()> {
    let account_name = choose_or_create_remote_account(accounts, "Generic IMAP/SMTP account")?;
    let existing = load_account_record(&account_name)?;
    let username = prompt_required_with_default(
        &format!("Username/login for {}: ", account_name),
        existing
            .as_ref()
            .and_then(|record| record.username.as_deref()),
    )?;
    let imap_host = prompt_required_with_default(
        "IMAP host: ",
        existing
            .as_ref()
            .and_then(|record| record.imap_host.as_deref()),
    )?;
    let imap_port = prompt_port_with_default(
        "IMAP port: ",
        existing.as_ref().and_then(|record| record.imap_port),
        993,
    )?;
    let smtp_host = prompt_required_with_default(
        "SMTP host: ",
        existing
            .as_ref()
            .and_then(|record| record.smtp_host.as_deref()),
    )?;
    let smtp_port = prompt_port_with_default(
        "SMTP port: ",
        existing.as_ref().and_then(|record| record.smtp_port),
        465,
    )?;
    let password = prompt_password(&format!("Password for {}: ", account_name))?;

    let imap_secret_id = existing
        .as_ref()
        .and_then(|record| record.keyring_imap_secret_id.clone())
        .unwrap_or_else(|| secret_service_id(&account_name, "imap"));
    let smtp_secret_id = existing
        .as_ref()
        .and_then(|record| record.keyring_smtp_secret_id.clone())
        .unwrap_or_else(|| secret_service_id(&account_name, "smtp"));

    store_secret(
        &format!("{} IMAP password", account_name),
        &imap_secret_id,
        &username,
        &password,
    )?;
    store_secret(
        &format!("{} SMTP password", account_name),
        &smtp_secret_id,
        &username,
        &password,
    )?;

    let config = AccountConfig {
        name: account_name.clone(),
        backend_kind: "imap".to_string(),
        provider_kind: "generic".to_string(),
        enabled: true,
        is_default: existing
            .as_ref()
            .map(|record| record.is_default)
            .unwrap_or_else(|| selectable_remote_accounts(accounts).is_empty()),
        maildir_path: None,
        imap_host: Some(imap_host),
        imap_port: Some(imap_port),
        imap_security: Some("tls".to_string()),
        smtp_host: Some(smtp_host),
        smtp_port: Some(smtp_port),
        smtp_security: Some("tls".to_string()),
        auth_mode: Some("password".to_string()),
        username: Some(username),
        keyring_imap_secret_id: Some(imap_secret_id),
        keyring_smtp_secret_id: Some(smtp_secret_id),
    };

    save_account_config(&config)?;
    println!(
        "Stored app-owned IMAP/SMTP definition for {}.",
        account_name
    );
    print_probe_result(&account_name, Some("imap"));
    Ok(())
}

fn configure_icloud_account(accounts: &[Account]) -> Result<()> {
    let account_name = choose_or_create_remote_account(accounts, "iCloud account")?;
    let existing = load_account_record(&account_name)?;
    let email = prompt_nonempty("iCloud email address: ")?;
    let password = prompt_password("iCloud app-specific password: ")?;

    let imap_secret_id = existing
        .as_ref()
        .and_then(|record| record.keyring_imap_secret_id.clone())
        .unwrap_or_else(|| secret_service_id(&account_name, "imap"));
    let smtp_secret_id = existing
        .as_ref()
        .and_then(|record| record.keyring_smtp_secret_id.clone())
        .unwrap_or_else(|| secret_service_id(&account_name, "smtp"));

    store_secret(
        &format!("{} IMAP password", account_name),
        &imap_secret_id,
        &email,
        &password,
    )?;
    store_secret(
        &format!("{} SMTP password", account_name),
        &smtp_secret_id,
        &email,
        &password,
    )?;

    if authinfo_gpg_path().is_file() {
        let recipient = prompt_nonempty("GPG recipient for ~/.authinfo.gpg: ")?;
        rewrite_authinfo_gpg(&email, &password, &recipient)?;
        println!("Updated ~/.authinfo.gpg for iCloud compatibility.");
    }

    let config = AccountConfig {
        name: account_name.clone(),
        backend_kind: "imap".to_string(),
        provider_kind: "icloud".to_string(),
        enabled: true,
        is_default: existing
            .as_ref()
            .map(|record| record.is_default)
            .unwrap_or_else(|| selectable_remote_accounts(accounts).is_empty()),
        maildir_path: None,
        imap_host: Some("imap.mail.me.com".to_string()),
        imap_port: Some(993),
        imap_security: Some("tls".to_string()),
        smtp_host: Some("smtp.mail.me.com".to_string()),
        smtp_port: Some(587),
        smtp_security: Some("starttls".to_string()),
        auth_mode: Some("app_password".to_string()),
        username: Some(email),
        keyring_imap_secret_id: Some(imap_secret_id),
        keyring_smtp_secret_id: Some(smtp_secret_id),
    };

    save_account_config(&config)?;
    println!("Stored app-owned iCloud definition for {}.", account_name);
    print_probe_result(&account_name, Some("imap"));
    Ok(())
}

fn configure_oauth(accounts: Option<&[Account]>) -> Result<()> {
    let account_name = choose_oauth_account(accounts)?;
    println!("Starting OAuth setup for {}...", account_name);
    client::configure_account(&account_name)?;
    print_probe_result(
        &account_name,
        accounts.and_then(|items| find_backend(items, &account_name)),
    );
    Ok(())
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

fn choose_or_create_remote_account(accounts: &[Account], prompt_text: &str) -> Result<String> {
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

fn choose_oauth_account(accounts: Option<&[Account]>) -> Result<String> {
    if let Some(accounts) = accounts {
        let selectable = selectable_remote_accounts(accounts);
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

        let raw = prompt(&format!("OAuth account [0-{}]: ", selectable.len()))?;
        if raw == "0" {
            return prompt_nonempty("New account name: ");
        }

        let choice: usize = raw.parse().context("invalid account selection")?;
        let account = selectable
            .get(choice.saturating_sub(1))
            .copied()
            .ok_or_else(|| anyhow!("account selection out of range"))?;
        return Ok(account.name.clone());
    }

    prompt_nonempty("Account name to configure: ")
}

fn prompt(label: &str) -> Result<String> {
    print!("{label}");
    io::stdout().flush().context("failed to flush stdout")?;
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .context("failed to read from stdin")?;
    Ok(line.trim().to_string())
}

fn prompt_nonempty(label: &str) -> Result<String> {
    let value = prompt(label)?;
    if value.is_empty() {
        bail!("input is required");
    }
    Ok(value)
}

fn prompt_required_with_default(label: &str, default: Option<&str>) -> Result<String> {
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

fn prompt_port_with_default(label: &str, default: Option<u16>, fallback: u16) -> Result<u16> {
    let default = default.unwrap_or(fallback);
    let value = prompt(&format!("{label}[{default}] "))?;
    if value.is_empty() {
        return Ok(default);
    }
    value.parse().context("invalid port")
}

fn prompt_password(label: &str) -> Result<String> {
    let value = rpassword::prompt_password(label).context("failed to read password")?;
    if value.is_empty() {
        bail!("password is required");
    }
    Ok(value)
}

fn print_probe_result(account: &str, _backend: Option<&str>) {
    match mail_service().probe_account(account) {
        Ok(()) => println!("✓ {} is working.", account),
        Err(error) => println!("Stored definition for {account}. Current runtime status: {error}"),
    }
}

fn find_backend<'a>(accounts: &'a [Account], account_name: &str) -> Option<&'a str> {
    accounts
        .iter()
        .find(|account| account.name == account_name)
        .map(|account| account.backend.as_str())
}

fn selectable_remote_accounts(accounts: &[Account]) -> Vec<&Account> {
    accounts
        .iter()
        .filter(|account| !account.backend.eq_ignore_ascii_case("maildir"))
        .collect()
}

fn should_show_bootstrap_only(accounts: &[Account]) -> bool {
    selectable_remote_accounts(accounts).is_empty()
}

fn mail_service() -> Arc<dyn MailService> {
    default_mail_service()
}

fn load_account_record(name: &str) -> Result<Option<AccountRecord>> {
    let conn = db::open()?;
    account_store::get_account(&conn, name)
}

fn save_account_config(config: &AccountConfig) -> Result<()> {
    let conn = db::open()?;
    account_store::upsert_account(&conn, config)
}

fn bail_app_owned_remote_setup() -> Result<()> {
    bail!(
        "App-owned IMAP/SMTP setup is temporarily disabled until native remote transport lands. Use the temporary OAuth flow or a legacy Himalaya-backed account for now."
    )
}

fn secret_service_id(account_name: &str, protocol: &str) -> String {
    format!("solverforge-mail/{account_name}/{protocol}")
}

fn store_secret(label: &str, service: &str, username: &str, password: &str) -> Result<()> {
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
            "solverforge-mail",
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

fn authinfo_gpg_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".authinfo.gpg")
}

fn rewrite_authinfo_gpg(email: &str, password: &str, recipient: &str) -> Result<()> {
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

    let tmp =
        std::env::temp_dir().join(format!("solverforge-mail-authinfo-{}", std::process::id()));
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

#[cfg(test)]
mod tests {
    use super::{selectable_remote_accounts, should_show_bootstrap_only};
    use crate::mail::types::Account;

    #[test]
    fn only_test_account_uses_bootstrap_menu() {
        let accounts = vec![Account {
            name: "test".to_string(),
            backend: "maildir".to_string(),
            default: false,
        }];

        assert!(should_show_bootstrap_only(&accounts));
        assert!(selectable_remote_accounts(&accounts).is_empty());
    }

    #[test]
    fn remote_accounts_enable_full_setup_menu() {
        let accounts = vec![
            Account {
                name: "test".to_string(),
                backend: "maildir".to_string(),
                default: false,
            },
            Account {
                name: "work".to_string(),
                backend: "imap".to_string(),
                default: false,
            },
        ];

        assert!(!should_show_bootstrap_only(&accounts));
        assert_eq!(selectable_remote_accounts(&accounts).len(), 1);
    }
}
