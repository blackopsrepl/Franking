/*! Generic IMAP/SMTP and iCloud account setup. */

use anyhow::Result;

use crate::mail::account_store::AccountConfig;
use crate::mail::types::Account;

use super::config::{
    authinfo_gpg_path, load_account_record, rewrite_authinfo_gpg, save_account_config,
    secret_service_id, store_secret,
};
use super::wizard::{
    choose_or_create_remote_account, print_probe_result, prompt_nonempty, prompt_password,
    prompt_port_with_default, prompt_required_with_default, selectable_remote_accounts,
};

pub(super) fn configure_password_account(accounts: &[Account]) -> Result<()> {
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
        sieve_host: None,
        sieve_port: None,
        sieve_security: None,
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

pub(super) fn configure_icloud_account(accounts: &[Account]) -> Result<()> {
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
        sieve_host: None,
        sieve_port: None,
        sieve_security: None,
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
