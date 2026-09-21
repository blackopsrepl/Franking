/*! Raw message reads with caching and offline fallback. */

use crate::mail::errors::{MailError, MailResult};
use crate::mail::store;

use super::cache::{cache_message, is_offline};
use super::router::{Route, RouterMailService};
use super::service_trait::MailService;

pub(super) fn read_message_raw(
    router: &RouterMailService,
    account: Option<&str>,
    folder: &str,
    id: &str,
) -> MailResult<Vec<u8>> {
    let record = router.choose_account(account)?;
    let result = match router.route_account(Some(&record.name))? {
        Route::Maildir(service) => service.read_message_raw(account, folder, id),
        Route::Remote(service) => service.read_message_raw(account, folder, id),
    };

    match result {
        Ok(raw) => {
            if let Ok(document) = crate::mail::mime::parse_message(&raw) {
                let _ = router.with_db(|conn| {
                    cache_message(conn, &record.name, folder, id, &raw, &document)
                        .map_err(|err| MailError::config_invalid(err.to_string()))
                });
            }
            Ok(raw)
        }
        Err(error) if is_offline(&error) => router.with_db(|conn| {
            store::get_message(conn, &record.name, folder, id)
                .map_err(|err| MailError::config_invalid(err.to_string()))?
                .and_then(|message| message.raw)
                .ok_or(error)
        }),
        Err(error) => Err(error),
    }
}
