/*! Browser authorization-code flow with PKCE and the loopback callback. */

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, bail, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};
use url::Url;

use super::providers::{OAuthAuthorization, OAuthProvider, OAuthProviderKind};
use super::refresh::expires_at_rfc3339;
use super::token::{send_token_request, TokenResponse};

const CALLBACK_PATH: &str = "/oauth/callback";
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);

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
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\n{} received the OAuth callback. You can close this tab.\n",
                        crate::brand::NAME
                    )
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

pub(super) fn code_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

struct OAuthCallback {
    code: String,
}
