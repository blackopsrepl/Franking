/*! Classification of server refusals and transport failures. */

use crate::mail::errors::MailError;

/// Whether a server refusal reads like a rejected credential rather than a
/// protocol error, so the app can ask for a new password instead of retrying.
pub fn looks_like_auth_failure(detail: &str) -> bool {
    let lowered = detail.to_ascii_lowercase();
    lowered.contains("auth")
        || lowered.contains("login failed")
        || lowered.contains("invalid credentials")
        || lowered.contains("username and password")
        || lowered.contains("535")
}

/// Classify a failure from the app-owned client into the app's error taxonomy.
///
/// The client reports transport faults as plain messages or wrapped IO errors;
/// losing that classification would stop the pool from reconnecting.
pub fn map_codec_error(error: anyhow::Error) -> MailError {
    for cause in error.chain() {
        let Some(io) = cause.downcast_ref::<std::io::Error>() else {
            continue;
        };
        return match io.kind() {
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => {
                MailError::transport_timeout(io.to_string())
            }
            _ => MailError::connection_dropped(io.to_string()),
        };
    }

    let detail = error.to_string();
    let lowered = detail.to_ascii_lowercase();
    if lowered.contains("closed the connection") || lowered.contains("mid-response") {
        MailError::connection_dropped(detail)
    } else {
        MailError::other(detail)
    }
}
