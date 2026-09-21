/*! SMTP error classification. */

use lettre::transport::smtp::Error as SmtpError;

use crate::mail::errors::MailError;
use crate::mail::session::looks_like_auth_failure;

pub(super) fn map_smtp_error(error: SmtpError) -> MailError {
    if error.is_tls() {
        MailError::tls_failure(error.to_string())
    } else if error.is_timeout() {
        MailError::transport_timeout(error.to_string())
    } else if looks_like_auth_failure(&error.to_string()) {
        MailError::smtp_auth_rejected(error.to_string())
    } else if error.is_transport_shutdown() {
        MailError::connection_dropped(error.to_string())
    } else {
        MailError::other(error.to_string())
    }
}
