/*! Token refresh and keyring secret access for OAuth accounts. */

use std::io::Write;
use std::process::{Command, Stdio};

use chrono::{DateTime, Duration as ChronoDuration, Utc};

use super::super::account_store::{self, OauthState, OauthStateConfig};
use super::super::errors::{MailError, MailResult};
use super::token::{send_token_request, TokenResponse};
use crate::db;

const REFRESH_SKEW_SECS: i64 = 60;

pub fn ensure_access_token(account_name: &str, username: &str) -> MailResult<String> {
    let conn = db::open().map_err(|err| MailError::config_invalid(err.to_string()))?;
    let mut state = account_store::get_oauth_state(&conn, account_name)
        .map_err(|err| MailError::config_invalid(err.to_string()))?
        .ok_or_else(|| {
            MailError::oauth_reconfigure_required(format!(
                "OAuth state is missing for account {account_name}"
            ))
        })?;

    if let Some(token) = cached_token_if_fresh(&state) {
        return Ok(token);
    }

    let refresh_token = lookup_secret(&state.refresh_token_ref, username)?;
    let client_secret = state
        .client_secret_ref
        .as_deref()
        .map(|service| lookup_secret(service, username))
        .transpose()?;

    let refreshed = refresh_access_token(
        &state.token_endpoint,
        &state.client_id,
        client_secret.as_deref(),
        &refresh_token,
    )?;
    if let Some(next_refresh_token) = refreshed.refresh_token.as_ref() {
        store_secret(
            &format!("{account_name} OAuth refresh token"),
            &state.refresh_token_ref,
            username,
            next_refresh_token,
        )?;
    }
    state.access_token_cached = Some(refreshed.access_token.clone());
    state.access_token_expires_at = expires_at_rfc3339(refreshed.expires_in);
    if let Some(scopes) = refreshed.scope.as_ref() {
        state.scopes = scopes.clone();
    }

    account_store::upsert_oauth_state(
        &conn,
        account_name,
        &OauthStateConfig {
            provider_kind: state.provider_kind.clone(),
            client_id: state.client_id.clone(),
            client_secret_ref: state.client_secret_ref.clone(),
            refresh_token_ref: state.refresh_token_ref.clone(),
            access_token_cached: state.access_token_cached.clone(),
            access_token_expires_at: state.access_token_expires_at.clone(),
            scopes: state.scopes.clone(),
            token_endpoint: state.token_endpoint.clone(),
            auth_endpoint: state.auth_endpoint.clone(),
        },
    )
    .map_err(|err| MailError::config_invalid(err.to_string()))?;

    Ok(refreshed.access_token)
}

fn cached_token_if_fresh(state: &OauthState) -> Option<String> {
    let token = state.access_token_cached.as_ref()?;
    let expires_at = state.access_token_expires_at.as_deref()?;
    let expires_at = DateTime::parse_from_rfc3339(expires_at).ok()?;
    if expires_at.with_timezone(&Utc) > Utc::now() + ChronoDuration::seconds(REFRESH_SKEW_SECS) {
        Some(token.clone())
    } else {
        None
    }
}

fn refresh_access_token(
    token_endpoint: &str,
    client_id: &str,
    client_secret: Option<&str>,
    refresh_token: &str,
) -> MailResult<TokenResponse> {
    let mut form = vec![
        ("grant_type", "refresh_token".to_string()),
        ("refresh_token", refresh_token.to_string()),
        ("client_id", client_id.to_string()),
    ];
    if let Some(secret) = client_secret.filter(|value| !value.is_empty()) {
        form.push(("client_secret", secret.to_string()));
    }
    send_token_request(token_endpoint, &form).map_err(map_refresh_error)
}

pub(super) fn expires_at_rfc3339(expires_in: Option<u64>) -> Option<String> {
    expires_in.map(|seconds| (Utc::now() + ChronoDuration::seconds(seconds as i64)).to_rfc3339())
}

fn map_refresh_error(error: anyhow::Error) -> MailError {
    let detail = error.to_string();
    let lowered = detail.to_ascii_lowercase();
    if lowered.contains("invalid_grant")
        || lowered.contains("invalid_client")
        || lowered.contains("invalid_request")
        || lowered.contains("interaction_required")
    {
        MailError::oauth_reconfigure_required(detail)
    } else if lowered.contains("timed out") {
        MailError::transport_timeout(detail)
    } else {
        MailError::oauth_refresh_failure(detail)
    }
}

fn lookup_secret(service: &str, username: &str) -> MailResult<String> {
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
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                MailError::keyring_unavailable(
                    "secret-tool is not installed or not available in PATH",
                )
            } else {
                MailError::keyring_unavailable(err.to_string())
            }
        })?;

    if !output.status.success() {
        return Err(MailError::keyring_unavailable(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    let secret = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if secret.is_empty() {
        Err(MailError::secret_missing(format!(
            "no secret found for service {service}"
        )))
    } else {
        Ok(secret)
    }
}

fn store_secret(label: &str, service: &str, username: &str, secret: &str) -> MailResult<()> {
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
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                MailError::keyring_unavailable(
                    "secret-tool is not installed or not available in PATH",
                )
            } else {
                MailError::keyring_unavailable(err.to_string())
            }
        })?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(secret.as_bytes())
            .map_err(|err| MailError::keyring_unavailable(err.to_string()))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|err| MailError::keyring_unavailable(err.to_string()))?;
    if !output.status.success() {
        return Err(MailError::keyring_unavailable(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    Ok(())
}
