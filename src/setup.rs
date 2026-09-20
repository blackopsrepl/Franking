use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};

use crate::db;
use crate::mail::account_store::{self, AccountConfig, AccountRecord, OauthStateConfig};
use crate::mail::oauth::{self, OAuthProvider};
use crate::mail::types::Account;
use crate::mail::{default_mail_service, MailService};

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

fn configure_oauth_account(accounts: &[Account], provider: &OAuthProvider) -> Result<()> {
    let account_name =
        choose_or_create_remote_account(accounts, &format!("{} account", provider.display_name))?;
    let existing = load_account_record(&account_name)?;
    let existing_oauth = load_oauth_state(&account_name)?;
    let username = prompt_required_with_default(
        &format!("{} email/login: ", provider.display_name),
        existing
            .as_ref()
            .and_then(|record| record.username.as_deref()),
    )?;
    let existing_client_id = existing_oauth
        .as_ref()
        .map(|state| state.client_id.as_str());
    let client_id = prompt_required_with_default("OAuth client ID: ", existing_client_id)?;
    let client_secret = prompt_optional_password(
        match existing_oauth
            .as_ref()
            .and_then(|state| state.client_secret_ref.as_deref())
        {
            Some(_) => "OAuth client secret (leave blank to keep the stored value): ",
            None => "OAuth client secret (optional, press Enter to skip): ",
        },
    )?;
    let existing_client_secret = if client_secret.is_none() {
        existing_oauth
            .as_ref()
            .and_then(|state| state.client_secret_ref.as_deref())
            .map(|service| {
                lookup_secret(
                    service,
                    existing
                        .as_ref()
                        .and_then(|record| record.username.as_deref())
                        .unwrap_or(username.as_str()),
                )
            })
            .transpose()?
    } else {
        None
    };

    let authorization = oauth::authorize_account(
        provider,
        &client_id,
        client_secret
            .as_deref()
            .or(existing_client_secret.as_deref()),
        Some(&username),
    )?;

    let refresh_token_ref = existing_oauth
        .as_ref()
        .map(|state| state.refresh_token_ref.clone())
        .unwrap_or_else(|| secret_service_id(&account_name, "oauth-refresh"));
    store_secret(
        &format!("{} OAuth refresh token", account_name),
        &refresh_token_ref,
        &username,
        &authorization.refresh_token,
    )?;

    let client_secret_ref = match (
        client_secret
            .as_deref()
            .or(existing_client_secret.as_deref()),
        existing_oauth
            .as_ref()
            .and_then(|state| state.client_secret_ref.clone()),
    ) {
        (Some(secret), existing_ref) => {
            let service = existing_ref
                .unwrap_or_else(|| secret_service_id(&account_name, "oauth-client-secret"));
            if client_secret.is_some()
                || existing
                    .as_ref()
                    .and_then(|record| record.username.as_deref())
                    != Some(username.as_str())
            {
                store_secret(
                    &format!("{} OAuth client secret", account_name),
                    &service,
                    &username,
                    secret,
                )?;
            }
            Some(service)
        }
        (None, existing_ref) => existing_ref,
    };

    let config = AccountConfig {
        name: account_name.clone(),
        backend_kind: "imap".to_string(),
        provider_kind: provider.provider_kind.to_string(),
        enabled: true,
        is_default: existing
            .as_ref()
            .map(|record| record.is_default)
            .unwrap_or_else(|| selectable_remote_accounts(accounts).is_empty()),
        maildir_path: None,
        imap_host: Some(provider.imap_host.to_string()),
        imap_port: Some(provider.imap_port),
        imap_security: Some(provider.imap_security.to_string()),
        smtp_host: Some(provider.smtp_host.to_string()),
        smtp_port: Some(provider.smtp_port),
        smtp_security: Some(provider.smtp_security.to_string()),
        auth_mode: Some("oauth2".to_string()),
        username: Some(username.clone()),
        keyring_imap_secret_id: None,
        keyring_smtp_secret_id: None,
    };
    save_account_config(&config)?;

    save_oauth_state(
        &account_name,
        &OauthStateConfig {
            provider_kind: provider.provider_kind.to_string(),
            client_id,
            client_secret_ref,
            refresh_token_ref,
            access_token_cached: Some(authorization.access_token),
            access_token_expires_at: authorization.expires_at,
            scopes: authorization.scopes,
            token_endpoint: provider.token_endpoint.to_string(),
            auth_endpoint: provider.auth_endpoint.to_string(),
        },
    )?;

    println!(
        "Stored app-owned {} OAuth definition for {}.",
        provider.display_name, account_name
    );
    print_probe_result(&account_name, Some("imap"));
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

fn prompt_optional_password(label: &str) -> Result<Option<String>> {
    let value = rpassword::prompt_password(label).context("failed to read password")?;
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

fn print_probe_result(account: &str, _backend: Option<&str>) {
    match mail_service().probe_account(account) {
        Ok(()) => println!("✓ {} is working.", account),
        Err(error) => println!("Stored definition for {account}. Current runtime status: {error}"),
    }
}

fn selectable_remote_accounts(accounts: &[Account]) -> Vec<&Account> {
    accounts
        .iter()
        .filter(|account| !account.backend.eq_ignore_ascii_case("maildir"))
        .collect()
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

fn load_oauth_state(name: &str) -> Result<Option<account_store::OauthState>> {
    let conn = db::open()?;
    account_store::get_oauth_state(&conn, name)
}

fn save_oauth_state(account_name: &str, config: &OauthStateConfig) -> Result<()> {
    let conn = db::open()?;
    account_store::upsert_oauth_state(&conn, account_name, config)
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

fn lookup_secret(service: &str, username: &str) -> Result<String> {
    let output = Command::new("secret-tool")
        .args([
            "lookup",
            "service",
            service,
            "username",
            username,
            "application",
            "solverforge-mail",
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
    use super::selectable_remote_accounts;
    use crate::mail::types::Account;

    #[test]
    fn only_test_account_has_no_remote_options() {
        let accounts = vec![Account {
            name: "test".to_string(),
            backend: "maildir".to_string(),
            default: false,
        }];

        assert!(selectable_remote_accounts(&accounts).is_empty());
    }

    #[test]
    fn remote_accounts_are_selectable_in_setup() {
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

        assert_eq!(selectable_remote_accounts(&accounts).len(), 1);
    }
}
