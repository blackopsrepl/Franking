/*! Collapse a high-volume sender to one row, expandable on demand. */

use std::collections::{HashMap, HashSet};

use crate::db::{bundles, sender_routes};

use super::model::App;

impl App {
    /// Load the bundled senders for every account in the current list.
    pub(crate) fn load_bundled_senders(&mut self, accounts: &[String]) {
        self.bundled_senders.clear();
        let Some(conn) = self.db.as_ref() else {
            return;
        };
        for account in accounts {
            if let Ok(senders) = bundles::bundled_for_account(conn, account) {
                for sender in senders {
                    self.bundled_senders.insert((account.clone(), sender));
                }
            }
        }
    }

    /// Collapse bundled senders to their newest row, unless expanded.
    pub(crate) fn apply_bundles(&mut self) {
        self.bundled_reps.clear();
        if self.bundled_senders.is_empty() {
            return;
        }
        let bundled = std::mem::take(&mut self.envelopes);
        let mut kept: Vec<crate::mail::types::Envelope> = Vec::with_capacity(bundled.len());
        let mut index_of: HashMap<(String, String), usize> = HashMap::new();
        let mut reps: HashMap<String, usize> = HashMap::new();
        for envelope in bundled {
            let key = bundle_key(&envelope).filter(|key| {
                self.bundled_senders.contains(key) && !self.expanded_bundles.contains(key)
            });
            match key {
                Some(key) => {
                    if let Some(&index) = index_of.get(&key) {
                        if let Some(count) = reps.get_mut(&kept[index].id) {
                            *count += 1;
                        }
                    } else {
                        index_of.insert(key, kept.len());
                        reps.insert(envelope.id.clone(), 1);
                        kept.push(envelope);
                    }
                }
                None => kept.push(envelope),
            }
        }
        self.envelopes = kept;
        self.bundled_reps = reps;
    }

    /// Cycle the cursor's sender through not bundled, bundled, expanded.
    pub(crate) fn cycle_bundle(&mut self) {
        let Some(envelope) = self.selected_envelope().cloned() else {
            return;
        };
        let Some(key) = bundle_key(&envelope) else {
            self.set_status("This message has no sender mailbox to bundle.");
            return;
        };
        let Some(conn) = self.db.as_ref() else {
            self.set_error("Local database is unavailable.");
            return;
        };
        let bundled = self.bundled_senders.contains(&key);
        let expanded = self.expanded_bundles.contains(&key);
        let message = if !bundled {
            if let Err(error) = bundles::set_bundled(conn, &key.0, &key.1, true) {
                self.set_error(&format!("Could not bundle: {error}"));
                return;
            }
            "Sender bundled into one row."
        } else if !expanded {
            self.expanded_bundles.insert(key.clone());
            "Bundle expanded."
        } else {
            if let Err(error) = bundles::set_bundled(conn, &key.0, &key.1, false) {
                self.set_error(&format!("Could not unbundle: {error}"));
                return;
            }
            self.expanded_bundles.remove(&key);
            "Sender unbundled."
        };
        self.set_status(message);
        self.load_envelopes();
    }
}

/// Account-scoped sender key for an envelope, when the mailbox is trustworthy.
pub(crate) fn bundle_key(envelope: &crate::mail::types::Envelope) -> Option<(String, String)> {
    let account = envelope.account.clone()?;
    let sender = sender_routes::sender_address(&envelope.sender)?;
    Some((account, sender))
}

/// Re-export the set element type for the app state.
pub(crate) type BundleKey = (String, String);
pub(crate) type BundleKeys = HashSet<BundleKey>;

impl App {
    /// If the cursor row is a collapsed bundle, expand it instead of opening.
    pub(crate) fn expand_bundle_at_cursor(&mut self) -> bool {
        let Some(envelope) = self.selected_envelope().cloned() else {
            return false;
        };
        let Some(key) = bundle_key(&envelope) else {
            return false;
        };
        if !self.bundled_senders.contains(&key) || !self.bundled_reps.contains_key(&envelope.id) {
            return false;
        }
        self.expanded_bundles.insert(key);
        self.set_status("Bundle expanded.");
        self.load_envelopes();
        true
    }
}
