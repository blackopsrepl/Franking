/*! Gmail and Outlook OAuth account setup. */

use anyhow::Result;

use crate::mail::account_store::{AccountConfig, OauthStateConfig};
use crate::mail::oauth::{self, OAuthProvider};
use crate::mail::types::Account;

use super::config::{
    load_account_record, load_oauth_state, lookup_secret, save_account_config, save_oauth_state,
    secret_service_id, store_secret,
};
use super::wizard::{
    choose_or_create_remote_account, print_probe_result, prompt_optional_password,
    prompt_required_with_default, selectable_remote_accounts,
};

pub(super) fn configure_oauth_account(
    accounts: &[Account],
    provider: &OAuthProvider,
) -> Result<()> {
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
