/*! Envelope metadata fetching. */

use std::io::{Read, Write};

use crate::mail::errors::MailResult;
use crate::mail::session::map_imap_error;
use crate::mail::types::{Envelope, Sender};

use super::search::{imap_flag_name, uid_set};

pub(super) fn fetch_envelope_metadata<S: Read + Write>(
    session: &mut imap::Session<S>,
    uids: &[u32],
) -> MailResult<Vec<Envelope>> {
    if uids.is_empty() {
        return Ok(Vec::new());
    }

    let query = uid_set(uids);
    let mut envelopes = session
        .uid_fetch(query, "(UID FLAGS INTERNALDATE ENVELOPE)")
        .map_err(map_imap_error)?
        .iter()
        .map(fetch_to_envelope)
        .collect::<Vec<_>>();
    envelopes.sort_by(|left, right| {
        right
            .id
            .parse::<u32>()
            .unwrap_or_default()
            .cmp(&left.id.parse::<u32>().unwrap_or_default())
    });
    Ok(envelopes)
}

fn fetch_to_envelope(fetch: &imap::types::Fetch) -> Envelope {
    let subject = fetch
        .envelope()
        .and_then(|envelope| envelope.subject)
        .map(|bytes| String::from_utf8_lossy(bytes).trim().to_string())
        .unwrap_or_default();
    let sender = fetch
        .envelope()
        .and_then(|envelope| envelope.from.as_ref())
        .map(|addresses| sender_from_addresses(addresses))
        .unwrap_or(Sender::Unknown);
    let date = fetch
        .internal_date()
        .map(|date| date.format("%Y-%m-%d %H:%M:%S%:z").to_string())
        .or_else(|| {
            fetch
                .envelope()
                .and_then(|envelope| envelope.date)
                .map(|bytes| String::from_utf8_lossy(bytes).trim().to_string())
        })
        .unwrap_or_default();

    Envelope {
        id: fetch.uid.unwrap_or(fetch.message).to_string(),
        flags: fetch.flags().iter().map(imap_flag_name).collect(),
        subject,
        sender,
        date,
    }
}

fn sender_from_addresses(addresses: &[imap_proto::types::Address<'_>]) -> Sender {
    let Some(address) = addresses.first() else {
        return Sender::Unknown;
    };

    let name = address
        .name
        .map(|value| String::from_utf8_lossy(value).trim().to_string())
        .filter(|value: &String| !value.is_empty());
    let mailbox = address
        .mailbox
        .map(|value| String::from_utf8_lossy(value).trim().to_string())
        .filter(|value: &String| !value.is_empty());
    let host = address
        .host
        .map(|value| String::from_utf8_lossy(value).trim().to_string())
        .filter(|value: &String| !value.is_empty());
    let addr = match (mailbox, host) {
        (Some(local), Some(domain)) => Some(format!("{local}@{domain}")),
        (Some(local), None) => Some(local),
        _ => None,
    };

    if name.is_some() || addr.is_some() {
        Sender::Structured { name, addr }
    } else {
        Sender::Unknown
    }
}
