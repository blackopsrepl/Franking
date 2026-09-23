/*! Harvest addresses from a message opened in the reader. */

use crate::mail::MessageDocument;

use super::model::App;

impl App {
    /// Parse From/To/Cc/Reply-To addresses into contacts. Best effort.
    pub(crate) fn harvest_contacts_from_message(&mut self, message: &MessageDocument) {
        if self.db.is_none() {
            return;
        }
        let mut addrs: Vec<(Option<String>, String)> = Vec::new();
        for header in message.header_fields() {
            if ["from", "to", "cc", "reply-to"]
                .iter()
                .any(|name| header.name.eq_ignore_ascii_case(name))
            {
                addrs.extend(crate::contacts::parse_address_list(&header.value));
            }
        }
        let sender = self
            .selected_envelope()
            .map(|envelope| envelope.sender.display())
            .unwrap_or_default();
        if !sender.is_empty() {
            addrs.extend(crate::contacts::parse_address_list(&sender));
        }
        if let Some(ref conn) = self.db {
            for (name, email) in addrs {
                let _ = crate::contacts::upsert_harvested(conn, name.as_deref(), &email);
            }
        }
    }
}
