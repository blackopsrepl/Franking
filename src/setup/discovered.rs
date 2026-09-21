/*! Add an account from an email address using auto-discovery. */

use anyhow::{bail, Result};

use crate::mail::account_store::AccountConfig;
use crate::mail::autoconfig::{discover, DiscoveredConfig};
use crate::mail::oauth;
use crate::mail::types::Account;

use super::config::{save_account_config, secret_service_id, store_secret};
use super::oauth_setup::configure_oauth_account;
use super::wizard::{
    choose_or_create_remote_account, print_probe_result, prompt_nonempty, prompt_password,
    selectable_remote_accounts,
};

pub(super) fn configure_discovered_account(accounts: &[Account]) -> Result<()> {
    let email = prompt_nonempty("Email address: ")?;
    let Some(discovered) = discover(&email) else {
        bail!("could not discover settings for {email}; add the account manually");
    };

    println!(
        "Discovered {} for {email}: IMAP {}:{} / SMTP {}:{}",
        discovered.provider_kind,
        discovered.imap_host,
        discovered.imap_port,
        discovered.smtp_host,
        discovered.smtp_port
    );

    if discovered.auth_mode == "oauth2" {
        let Some(provider) = oauth::provider_by_kind(&discovered.provider_kind) else {
            bail!("OAuth2 discovery is only supported for Gmail and Outlook");
        };
        return configure_oauth_account(accounts, provider);
    }

    configure_password_from_discovery(accounts, &email, &discovered)
}

fn configure_password_from_discovery(
    accounts: &[Account],
    email: &str,
    discovered: &DiscoveredConfig,
) -> Result<()> {
    let account_name = choose_or_create_remote_account(
        accounts,
        &format!("{} account", discovered.provider_kind),
    )?;
    let password_label = if discovered.auth_mode == "app_password" {
        "App-specific password: "
    } else {
        "Password: "
    };
    let password = prompt_password(password_label)?;

    let imap_secret_id = secret_service_id(&account_name, "imap");
    let smtp_secret_id = secret_service_id(&account_name, "smtp");
    store_secret(
        &format!("{account_name} IMAP password"),
        &imap_secret_id,
        email,
        &password,
    )?;
    store_secret(
        &format!("{account_name} SMTP password"),
        &smtp_secret_id,
        email,
        &password,
    )?;

    let config = AccountConfig {
        name: account_name.clone(),
        backend_kind: "imap".to_string(),
        provider_kind: discovered.provider_kind.clone(),
        enabled: true,
        is_default: selectable_remote_accounts(accounts).is_empty(),
        maildir_path: None,
        imap_host: Some(discovered.imap_host.clone()),
        imap_port: Some(discovered.imap_port),
        imap_security: Some(discovered.imap_security.clone()),
        smtp_host: Some(discovered.smtp_host.clone()),
        smtp_port: Some(discovered.smtp_port),
        smtp_security: Some(discovered.smtp_security.clone()),
        sieve_host: None,
        sieve_port: None,
        sieve_security: None,
        auth_mode: Some(discovered.auth_mode.clone()),
        username: Some(email.to_string()),
        keyring_imap_secret_id: Some(imap_secret_id),
        keyring_smtp_secret_id: Some(smtp_secret_id),
    };

    save_account_config(&config)?;
    println!("Stored discovered account definition for {account_name}.");
    print_probe_result(&account_name, Some("imap"));
    Ok(())
}
