/*! Turning a raw message into one archive of its attachments.
Shared by the backends so the archive naming and layout are identical whether
the message came from IMAP or from a local maildir. */

use crate::mail::errors::MailResult;

/// Archive every attachment of a raw message, returning the archive path.
pub fn archive_from_raw(raw: Vec<u8>) -> MailResult<String> {
    let document = crate::mail::mime::parse_message(&raw)?;
    crate::mail::attachments::save_attachments_as_zip(
        crate::mail::attachments::payloads(&document),
        &crate::mail::attachments::downloads_dir(),
        document.subject(),
    )
}
