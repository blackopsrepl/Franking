/*! Token endpoint requests and the raw token response. */

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use ureq::Error as HttpError;

pub(super) fn send_token_request(
    token_endpoint: &str,
    form: &[(&str, String)],
) -> Result<TokenResponse> {
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

pub(super) fn form_urlencoded(form: &[(&str, String)]) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in form {
        serializer.append_pair(key, value);
    }
    serializer.finish()
}

pub(super) fn map_http_error(error: HttpError) -> anyhow::Error {
    match error {
        HttpError::Status(_status, response) => {
            let body = response.into_string().unwrap_or_default();
            anyhow!(body)
        }
        HttpError::Transport(error) => anyhow!(error.to_string()),
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct TokenResponse {
    pub(super) access_token: String,
    pub(super) refresh_token: Option<String>,
    pub(super) expires_in: Option<u64>,
    pub(super) scope: Option<String>,
}
