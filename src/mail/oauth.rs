use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use ureq::Error as HttpError;
use url::Url;

use super::account_store::{self, OauthState, OauthStateConfig};
use super::errors::{MailError, MailResult};
use crate::db;

const CALLBACK_PATH: &str = "/oauth/callback";
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);
const REFRESH_SKEW_SECS: i64 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuthProviderKind {
    Gmail,
    Outlook,
}

#[derive(Debug, Clone, Copy)]
pub struct OAuthProvider {
    pub kind: OAuthProviderKind,
    pub provider_kind: &'static str,
    pub display_name: &'static str,
    pub imap_host: &'static str,
    pub imap_port: u16,
    pub imap_security: &'static str,
    pub smtp_host: &'static str,
    pub smtp_port: u16,
    pub smtp_security: &'static str,
    pub auth_endpoint: &'static str,
    pub token_endpoint: &'static str,
    pub scopes: &'static [&'static str],
}

#[derive(Debug, Clone)]
pub struct OAuthAuthorization {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: Option<String>,
    pub scopes: String,
}

pub const GMAIL_PROVIDER: OAuthProvider = OAuthProvider {
    kind: OAuthProviderKind::Gmail,
    provider_kind: "gmail",
    display_name: "Gmail",
    imap_host: "imap.gmail.com",
    imap_port: 993,
    imap_security: "tls",
    smtp_host: "smtp.gmail.com",
    smtp_port: 587,
    smtp_security: "starttls",
    auth_endpoint: "https://accounts.google.com/o/oauth2/v2/auth",
    token_endpoint: "https://oauth2.googleapis.com/token",
    scopes: &["https://mail.google.com/"],
};

pub const OUTLOOK_PROVIDER: OAuthProvider = OAuthProvider {
    kind: OAuthProviderKind::Outlook,
    provider_kind: "outlook",
    display_name: "Outlook",
    imap_host: "outlook.office365.com",
    imap_port: 993,
    imap_security: "tls",
    smtp_host: "smtp.office365.com",
    smtp_port: 587,
    smtp_security: "starttls",
    auth_endpoint: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize",
    token_endpoint: "https://login.microsoftonline.com/common/oauth2/v2.0/token",
    scopes: &[
        "offline_access",
        "https://outlook.office.com/IMAP.AccessAsUser.All",
        "https://outlook.office.com/SMTP.Send",
    ],
};

pub fn provider_by_kind(kind: &str) -> Option<&'static OAuthProvider> {
    match kind.trim().to_ascii_lowercase().as_str() {
        "gmail" => Some(&GMAIL_PROVIDER),
        "outlook" => Some(&OUTLOOK_PROVIDER),
        _ => None,
    }
}

pub fn authorize_account(
    provider: &OAuthProvider,
    client_id: &str,
    client_secret: Option<&str>,
    login_hint: Option<&str>,
) -> Result<OAuthAuthorization> {
    let listener =
        TcpListener::bind(("127.0.0.1", 0)).context("failed to bind OAuth callback listener")?;
    listener
        .set_nonblocking(true)
        .context("failed to configure OAuth callback listener")?;
    let redirect_uri = format!(
        "http://127.0.0.1:{}{}",
        listener.local_addr()?.port(),
        CALLBACK_PATH
    );

    let verifier = random_urlsafe(48);
    let challenge = code_challenge(&verifier);
    let state = random_urlsafe(24);

    let mut auth_url = Url::parse(provider.auth_endpoint)?;
    {
        let mut query = auth_url.query_pairs_mut();
        query.append_pair("response_type", "code");
        query.append_pair("client_id", client_id);
        query.append_pair("redirect_uri", &redirect_uri);
        query.append_pair("scope", &provider.scopes.join(" "));
        query.append_pair("code_challenge", &challenge);
        query.append_pair("code_challenge_method", "S256");
        query.append_pair("state", &state);
        match provider.kind {
            OAuthProviderKind::Gmail => {
                query.append_pair("access_type", "offline");
                query.append_pair("prompt", "consent");
                query.append_pair("include_granted_scopes", "true");
            }
            OAuthProviderKind::Outlook => {
                query.append_pair("prompt", "select_account");
            }
        }
        if let Some(login_hint) = login_hint.filter(|value| !value.trim().is_empty()) {
            query.append_pair("login_hint", login_hint.trim());
        }
    }

    println!();
    println!(
        "Opening the browser for {} OAuth authorization.",
        provider.display_name
    );
    println!(
        "If the browser does not open, visit:\n{}\n",
        auth_url.as_str()
    );
    let _ = open_url(auth_url.as_str());

    let callback = wait_for_callback(&listener, &state)?;
    let token = exchange_authorization_code(
        provider.token_endpoint,
        client_id,
        client_secret,
        &callback.code,
        &redirect_uri,
        &verifier,
    )?;
    let refresh_token = token
        .refresh_token
        .ok_or_else(|| anyhow!("provider did not return a refresh token"))?;

    Ok(OAuthAuthorization {
        access_token: token.access_token,
        refresh_token,
        expires_at: expires_at_rfc3339(token.expires_in),
        scopes: token.scope.unwrap_or_else(|| provider.scopes.join(" ")),
    })
}

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

fn wait_for_callback(listener: &TcpListener, expected_state: &str) -> Result<OAuthCallback> {
    let started = Instant::now();
    loop {
        match listener.accept() {
            Ok((mut stream, _addr)) => {
                let mut buffer = [0u8; 4096];
                let read = stream
                    .read(&mut buffer)
                    .context("failed reading OAuth callback")?;
                let request = String::from_utf8_lossy(&buffer[..read]);
                let first_line = request
                    .lines()
                    .next()
                    .ok_or_else(|| anyhow!("OAuth callback request was empty"))?;
                let path = first_line
                    .split_whitespace()
                    .nth(1)
                    .ok_or_else(|| anyhow!("OAuth callback request was malformed"))?;
                let url = Url::parse(&format!("http://127.0.0.1{path}"))?;
                let params = url.query_pairs().collect::<Vec<_>>();
                let code = params
                    .iter()
                    .find(|(key, _)| key == "code")
                    .map(|(_, value)| value.to_string());
                let state = params
                    .iter()
                    .find(|(key, _)| key == "state")
                    .map(|(_, value)| value.to_string());
                let error = params
                    .iter()
                    .find(|(key, _)| key == "error")
                    .map(|(_, value)| value.to_string());

                let response = if let Some(ref error) = error {
                    format!(
                        "HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain\r\n\r\nOAuth authorization failed: {error}\n"
                    )
                } else {
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nSolverForge Mail received the OAuth callback. You can close this tab.\n".to_string()
                };
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();

                if let Some(error) = error {
                    bail!("OAuth provider returned an error: {error}");
                }

                let code = code.ok_or_else(|| anyhow!("OAuth callback did not include a code"))?;
                let state = state.ok_or_else(|| anyhow!("OAuth callback did not include state"))?;
                if state != expected_state {
                    bail!("OAuth callback state did not match");
                }

                return Ok(OAuthCallback { code });
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if started.elapsed() >= CALLBACK_TIMEOUT {
                    bail!("timed out waiting for the OAuth browser callback");
                }
                thread::sleep(Duration::from_millis(200));
            }
            Err(error) => return Err(error).context("failed waiting for OAuth callback"),
        }
    }
}

fn exchange_authorization_code(
    token_endpoint: &str,
    client_id: &str,
    client_secret: Option<&str>,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<TokenResponse> {
    let mut form = vec![
        ("grant_type", "authorization_code".to_string()),
        ("code", code.to_string()),
        ("client_id", client_id.to_string()),
        ("redirect_uri", redirect_uri.to_string()),
        ("code_verifier", code_verifier.to_string()),
    ];
    if let Some(secret) = client_secret.filter(|value| !value.is_empty()) {
        form.push(("client_secret", secret.to_string()));
    }
    send_token_request(token_endpoint, &form)
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

fn send_token_request(token_endpoint: &str, form: &[(&str, String)]) -> Result<TokenResponse> {
    let body = form_urlencoded(form);
    let response = ureq::post(token_endpoint)
        .set("Content-Type", "application/x-www-form-urlencoded")
        .send_string(&body)
        .map_err(map_http_error)?;

    let parsed: TokenResponse =
        serde_json::from_reader(response.into_reader()).context("invalid OAuth token response")?;
    if parsed.access_token.is_empty() {
        bail!("OAuth token response did not include an access token");
    }
    Ok(parsed)
}

fn form_urlencoded(form: &[(&str, String)]) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in form {
        serializer.append_pair(key, value);
    }
    serializer.finish()
}

fn open_url(url: &str) -> std::io::Result<()> {
    if cfg!(target_os = "macos") {
        Command::new("open").arg(url).spawn()?;
    } else if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", "start", "", url]).spawn()?;
    } else {
        Command::new("xdg-open").arg(url).spawn()?;
    }
    Ok(())
}

fn random_urlsafe(len_bytes: usize) -> String {
    let mut bytes = vec![0u8; len_bytes];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn code_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

fn expires_at_rfc3339(expires_in: Option<u64>) -> Option<String> {
    expires_in.map(|seconds| (Utc::now() + ChronoDuration::seconds(seconds as i64)).to_rfc3339())
}

fn map_http_error(error: HttpError) -> anyhow::Error {
    match error {
        HttpError::Status(_status, response) => {
            let body = response.into_string().unwrap_or_default();
            anyhow!(body)
        }
        HttpError::Transport(error) => anyhow!(error.to_string()),
    }
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
            "solverforge-mail",
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
            "solverforge-mail",
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

#[derive(Debug)]
struct OAuthCallback {
    code: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    scope: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{code_challenge, expires_at_rfc3339, provider_by_kind, GMAIL_PROVIDER};

    #[test]
    fn providers_are_registered_by_kind() {
        assert_eq!(
            provider_by_kind("gmail").map(|provider| provider.display_name),
            Some("Gmail")
        );
        assert_eq!(
            provider_by_kind("outlook").map(|provider| provider.display_name),
            Some("Outlook")
        );
        assert!(provider_by_kind("unknown").is_none());
    }

    #[test]
    fn code_challenge_is_urlsafe() {
        let challenge = code_challenge("test-verifier");
        assert!(!challenge.contains('='));
        assert!(!challenge.contains('+'));
        assert!(!challenge.contains('/'));
    }

    #[test]
    fn expires_at_is_generated_for_token_lifetime() {
        let expires_at = expires_at_rfc3339(Some(3600)).unwrap();
        assert!(expires_at.contains('T'));
        assert!(expires_at.contains('+') || expires_at.ends_with('Z'));
    }

    #[test]
    fn gmail_provider_uses_mail_scope() {
        assert_eq!(GMAIL_PROVIDER.scopes, &["https://mail.google.com/"]);
    }
}
