/*! Folder synchronization: full listings and server-side deltas. */

use crate::mail::errors::{MailError, MailResult};
use crate::mail::types::Envelope;

use super::cache::{
    apply_folder_delta, cached_envelopes, is_offline, record_listing, stored_anchor,
};
use super::router::{route, RouterMailService};
use super::service_trait::MailService;
impl RouterMailService {
    /// Synchronize a folder, preferring a server-side delta when the account
    /// has a stored anchor, and falling back to a full listing.
    pub(super) fn sync_folder_cached(
        &self,
        account: Option<&str>,
        folder: &str,
    ) -> MailResult<Vec<Envelope>> {
        let record = self.choose_account(account)?;
        let name = record.name.clone();

        // A stored anchor lets the server report only what changed. The delta
        // updates the cache; the caller then reads the full listing from it.
        let anchor = self.with_db(|conn| Ok(stored_anchor(conn, &name, folder)))?;
        if let Some(anchor) = anchor {
            match route!(self, Some(&name), service => {
                service.sync_folder_delta(Some(&name), folder, Some(anchor))
            }) {
                Ok(delta) if !delta.full_resync => {
                    self.with_db(|conn| {
                        apply_folder_delta(conn, &name, folder, &delta)
                            .map_err(|err| MailError::config_invalid(err.to_string()))
                    })?;
                    return self.with_db(|conn| {
                        cached_envelopes(conn, &name, folder, 1, usize::MAX, None)
                    });
                }
                Ok(_) => {}
                Err(error) if is_offline(&error) => {
                    return self.with_db(|conn| {
                        cached_envelopes(conn, &name, folder, 1, usize::MAX, None)
                    });
                }
                Err(error) => return Err(error),
            }
        }

        // Full listing: fetch the state first so the next sync can be a delta.
        let state = route!(self, Some(&name), service => {
            service.sync_folder_delta(Some(&name), folder, None)
        })
        .unwrap_or_default();
        let envelopes = route!(self, Some(&name), service => service.sync_folder(account, folder))?;

        let cursor = self
            .folder_sync_cursor(Some(&name), folder)
            .unwrap_or((None, None));
        let _ = self.with_db(|conn| {
            record_listing(conn, &name, folder, &envelopes, cursor)
                .map_err(|err| MailError::config_invalid(err.to_string()))?;
            if let Some(modseq) = state.highest_modseq {
                apply_folder_delta(
                    conn,
                    &name,
                    folder,
                    &crate::mail::types::FolderDelta {
                        highest_modseq: Some(modseq),
                        uid_validity: state.uid_validity,
                        uid_next: state.uid_next,
                        ..crate::mail::types::FolderDelta::default()
                    },
                )
                .map_err(|err| MailError::config_invalid(err.to_string()))?;
            }
            Ok(())
        });
        Ok(envelopes)
    }
}
