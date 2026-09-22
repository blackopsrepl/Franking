/*! Non-interactive OAuth account authorization and persistence. */

use anyhow::{Context, Result};

use crate::mail::account_store::{AccountConfig, OauthStateConfig};
use crate::mail::oauth::{authorize_account, provider_by_kind, OAuthAuthorization, OAuthProvider};

/// Everything needed to authorize and store an OAuth account.
#[derive(Debug, Clone)]
pub struct OAuthAccountRequest {
    pub account: String,
    pub username: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub provider_kind: String,
    pub is_default: bool,
    /// Keep the previously stored client secret when none is supplied.
    pub existing_client_secret: Option<String>,
    pub existing_refresh_token_ref: Option<String>,
    pub existing_client_secret_ref: Option<String>,
}

/// Provider settings for a kind, if supported.
pub fn provider(kind: &str) -> Option<&'static OAuthProvider> {
    provider_by_kind(kind)
}

/// Run the browser flow for a request.
pub fn authorize(request: &OAuthAccountRequest) -> Result<OAuthAuthorization> {
    let provider = provider(&request.provider_kind)
        .with_context(|| format!("unsupported OAuth provider: {}", request.provider_kind))?;
    let secret = request
        .client_secret
        .as_deref()
        .or(request.existing_client_secret.as_deref());
    authorize_account(
        provider,
        &request.client_id,
        secret,
        Some(&request.username),
    )
}

/// Persist the account and its tokens through the app-owned stores.
pub fn store(request: &OAuthAccountRequest, authorization: &OAuthAuthorization) -> Result<()> {
    use crate::db;
    use crate::mail::account_store;
    use crate::setup::{save_account_config, save_oauth_state, secret_service_id, store_secret};

    let provider = provider(&request.provider_kind)
        .with_context(|| format!("unsupported OAuth provider: {}", request.provider_kind))?;

    let refresh_token_ref = request
        .existing_refresh_token_ref
        .clone()
        .unwrap_or_else(|| secret_service_id(&request.account, "oauth-refresh"));
    store_secret(
        &format!("{} OAuth refresh token", request.account),
        &refresh_token_ref,
        &request.username,
        &authorization.refresh_token,
    )?;

    let client_secret_ref = match (
        request.client_secret.as_deref(),
        request.existing_client_secret_ref.clone(),
    ) {
        (Some(secret), existing) => {
            let service = existing
                .unwrap_or_else(|| secret_service_id(&request.account, "oauth-client-secret"));
            store_secret(
                &format!("{} OAuth client secret", request.account),
                &service,
                &request.username,
                secret,
            )?;
            Some(service)
        }
        (None, existing) => existing,
    };

    let config = AccountConfig {
        name: request.account.clone(),
        backend_kind: "imap".to_string(),
        provider_kind: provider.provider_kind.to_string(),
        enabled: true,
        is_default: request.is_default,
        maildir_path: None,
        imap_host: Some(provider.imap_host.to_string()),
        imap_port: Some(provider.imap_port),
        imap_security: Some(provider.imap_security.to_string()),
        smtp_host: Some(provider.smtp_host.to_string()),
        smtp_port: Some(provider.smtp_port),
        smtp_security: Some(provider.smtp_security.to_string()),
        sieve_host: None,
        sieve_port: None,
        sieve_security: None,
        auth_mode: Some("oauth2".to_string()),
        username: Some(request.username.clone()),
        keyring_imap_secret_id: None,
        keyring_smtp_secret_id: None,
    };
    save_account_config(&config)?;

    save_oauth_state(
        &request.account,
        &OauthStateConfig {
            provider_kind: provider.provider_kind.to_string(),
            client_id: request.client_id.clone(),
            client_secret_ref,
            refresh_token_ref,
            access_token_cached: Some(authorization.access_token.clone()),
            access_token_expires_at: authorization.expires_at.clone(),
            scopes: authorization.scopes.clone(),
            token_endpoint: provider.token_endpoint.to_string(),
            auth_endpoint: provider.auth_endpoint.to_string(),
        },
    )?;

    // Touch the DB once so a missing schema fails here rather than later.
    let conn = db::open()?;
    let stored = account_store::get_account(&conn, &request.account)?;
    anyhow::ensure!(stored.is_some(), "account was not persisted");
    Ok(())
}
