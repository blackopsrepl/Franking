/*! Delta synchronization using CONDSTORE and QRESYNC. */

use crate::mail::errors::MailResult;
use crate::mail::types::FolderDelta;

use super::model::ImapSmtpService;
use super::next::{self, SyncAnchor};

impl ImapSmtpService {
    /// Report what changed in a folder since `anchor`, or report that a full
    /// listing is required.
    ///
    /// QRESYNC transfers a delta: the server names the messages that vanished
    /// and the messages whose flags changed. Servers without CONDSTORE refuse
    /// the SELECT parameters, which is reported as a full resync rather than an
    /// error. A transport failure is always propagated, so a dropped
    /// connection is never mistaken for "nothing changed".
    pub fn sync_folder_delta(
        &self,
        account: Option<&str>,
        folder: &str,
        anchor: Option<SyncAnchor>,
    ) -> MailResult<FolderDelta> {
        self.ensure_requested_account(account)?;

        self.pool.with_client(&self.account, |client| {
            if anchor.is_some() {
                // QRESYNC parameters and VANISHED require ENABLE first.
                if next::enable_qresync(client).is_err() {
                    let status = next::status(client, folder)?;
                    return Ok(FolderDelta {
                        full_resync: true,
                        uid_validity: status.uid_validity,
                        uid_next: status.uid_next,
                        ..FolderDelta::default()
                    });
                }
            }
            let state = match next::select_condstore(client, folder, anchor) {
                Ok(state) => state,
                Err(error) if error.is_transport() => return Err(error),
                Err(_) => {
                    next::select(client, folder)?;
                    let status = next::status(client, folder)?;
                    return Ok(FolderDelta {
                        full_resync: true,
                        uid_validity: status.uid_validity,
                        uid_next: status.uid_next,
                        ..FolderDelta::default()
                    });
                }
            };

            let mut delta = FolderDelta {
                uid_validity: state.uid_validity,
                uid_next: state.uid_next,
                highest_modseq: state.highest_modseq,
                ..FolderDelta::default()
            };

            let Some(anchor) = anchor else {
                delta.full_resync = true;
                return Ok(delta);
            };
            if state.highest_modseq.is_none() {
                // The server ignored QRESYNC; only a full listing is safe.
                delta.full_resync = true;
                return Ok(delta);
            }

            delta.vanished = state.vanished;
            delta.changed_flags = next::fetch_changed_flags(client, anchor.highest_modseq)?;
            Ok(delta)
        })
    }
}
