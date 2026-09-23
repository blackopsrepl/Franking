/*! Which new mail is worth a desktop notification. */

use crate::db::preferences;

use super::model::App;

/// Preference key for the desktop-notification rule.
const RULE: &str = "notification_rule";

/// When to raise a desktop notification for arriving mail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NotificationRule {
    /// Never.
    Off,
    /// Only explicitly accepted correspondence in an inbox.
    #[default]
    Focused,
    /// Every arriving message.
    All,
    /// Only messages from an address in the address book.
    Contacts,
}

impl NotificationRule {
    /// The next rule in the cycle.
    pub fn next(self) -> Self {
        match self {
            NotificationRule::Off => NotificationRule::Focused,
            NotificationRule::Focused => NotificationRule::All,
            NotificationRule::All => NotificationRule::Contacts,
            NotificationRule::Contacts => NotificationRule::Off,
        }
    }

    /// A short label for the preferences overlay and status bar.
    pub fn label(self) -> &'static str {
        match self {
            NotificationRule::Off => "off",
            NotificationRule::Focused => "focused inbox",
            NotificationRule::All => "every message",
            NotificationRule::Contacts => "contacts only",
        }
    }

    /// A stable name for storage.
    pub fn as_str(self) -> &'static str {
        match self {
            NotificationRule::Off => "off",
            NotificationRule::Focused => "focused",
            NotificationRule::All => "all",
            NotificationRule::Contacts => "contacts",
        }
    }

    /// Parse a stored name.
    pub fn parse(value: &str) -> Self {
        match value {
            "off" => NotificationRule::Off,
            "focused" => NotificationRule::Focused,
            "all" => NotificationRule::All,
            "contacts" => NotificationRule::Contacts,
            _ => NotificationRule::Focused,
        }
    }

    /// Whether a message from `sender` should raise a notification.
    ///
    /// `known_contact` reports whether the address is in the address book; it
    /// is only consulted for the contacts rule, so callers that cannot look
    /// contacts up can pass `false` without changing the other rules.
    pub fn allows(self, known_contact: bool) -> bool {
        match self {
            NotificationRule::Off => false,
            NotificationRule::Focused => false,
            NotificationRule::All => true,
            NotificationRule::Contacts => known_contact,
        }
    }
}

impl App {
    /// Whether arriving mail in `folder` should raise a notification.
    ///
    /// The watcher fetches the newest envelope off the UI thread for rules that
    /// depend on its sender. Failure to fetch does not create an alert.
    pub(crate) fn notify_for_new_mail(
        &self,
        account: Option<&str>,
        folder: &str,
        newest: Option<&crate::mail::types::Envelope>,
    ) -> bool {
        if let (Some(envelope), Some(conn)) = (newest, self.db.as_ref()) {
            let owner = envelope.account.as_deref().or(account).unwrap_or_default();
            if !owner.is_empty() {
                let anchors = crate::db::conversations::anchors(envelope);
                if crate::db::conversations::is_loud(conn, owner, &anchors).unwrap_or(false) {
                    return true;
                }
                if crate::db::conversations::is_muted(conn, owner, &anchors).unwrap_or(false) {
                    return false;
                }
            }
        }
        match self.notification_rule {
            NotificationRule::Off => false,
            NotificationRule::All => true,
            NotificationRule::Contacts | NotificationRule::Focused => {
                if self.notification_rule == NotificationRule::Focused
                    && !folder.eq_ignore_ascii_case("INBOX")
                {
                    return false;
                }
                let Some(envelope) = newest else {
                    return false;
                };
                if self.notification_rule == NotificationRule::Focused {
                    let Some(sender) = crate::db::sender_routes::sender_address(&envelope.sender)
                    else {
                        return false;
                    };
                    let Some(owner) = envelope.account.as_deref().or(account) else {
                        return false;
                    };
                    let bypassed = self
                        .db
                        .as_ref()
                        .and_then(|conn| {
                            crate::db::account_policy::bypass_token(conn, owner)
                                .ok()
                                .flatten()
                        })
                        .is_some_and(|token| {
                            crate::db::account_policy::subject_has_token(&envelope.subject, &token)
                        });
                    bypassed
                        || self.db.as_ref().and_then(|conn| {
                            crate::db::sender_routes::get(conn, owner, &sender).ok()
                        }) == Some(crate::db::sender_routes::Route::Inbox)
                } else {
                    let sender = crate::db::sender_routes::sender_address(&envelope.sender)
                        .unwrap_or_default();
                    !sender.is_empty() && self.should_notify(&sender)
                }
            }
        }
    }

    /// Remember the rule after cycling it.
    pub(crate) fn cycle_notification_rule(&mut self) {
        self.notification_rule = self.notification_rule.next();
        if let Some(conn) = self.db.as_ref() {
            if let Err(error) = preferences::set_text(conn, RULE, self.notification_rule.as_str()) {
                self.set_error(&format!("Could not save the preference: {error}"));
                return;
            }
            // Keep the older boolean key in step, so a downgrade still reads a
            // sensible value.
            let _ = preferences::set(
                conn,
                "notifications",
                self.notification_rule != NotificationRule::Off,
            );
        }
        self.set_status(&format!(
            "Notify for {}.{}",
            self.notification_rule.label(),
            if self.notification_rule == NotificationRule::Contacts {
                " (address-book senders)"
            } else {
                ""
            }
        ));
    }

    /// Whether a notification for `sender` is wanted, looking the sender up in
    /// the address book when the contacts rule is active.
    pub(crate) fn should_notify(&self, sender: &str) -> bool {
        if self.notification_rule != NotificationRule::Contacts {
            return self.notification_rule.allows(false);
        }
        let known = self
            .db
            .as_ref()
            .map(|conn| {
                crate::contacts::find_by_email(conn, sender)
                    .ok()
                    .flatten()
                    .is_some()
            })
            .unwrap_or(false);
        self.notification_rule.allows(known)
    }
}

/// Best-effort desktop notification for new mail (no-op when unavailable).
pub(crate) fn notify_new_mail(folder: &str) {
    use std::process::{Command, Stdio};

    let _ = Command::new("notify-send")
        .args([crate::brand::NAME, &format!("New mail in {folder}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

#[cfg(test)]
mod tests {
    use super::NotificationRule;

    #[test]
    fn cycling_reaches_every_rule_and_returns() {
        let mut rule = NotificationRule::default();
        assert_eq!(rule, NotificationRule::Focused);
        let mut seen = vec![rule];
        for _ in 0..4 {
            rule = rule.next();
            seen.push(rule);
        }
        assert_eq!(rule, NotificationRule::default(), "returns to the start");
        assert!(seen.contains(&NotificationRule::Off));
        assert!(seen.contains(&NotificationRule::Focused));
        assert!(seen.contains(&NotificationRule::Contacts));
    }

    #[test]
    fn stored_names_round_trip() {
        for rule in [
            NotificationRule::Off,
            NotificationRule::Focused,
            NotificationRule::All,
            NotificationRule::Contacts,
        ] {
            assert_eq!(NotificationRule::parse(rule.as_str()), rule);
        }
        assert_eq!(
            NotificationRule::parse("nonsense"),
            NotificationRule::Focused
        );
    }

    #[test]
    fn the_contacts_rule_needs_a_known_sender() {
        assert!(!NotificationRule::Off.allows(true));
        assert!(!NotificationRule::Focused.allows(true));
        assert!(
            NotificationRule::All.allows(false),
            "all mail, known or not"
        );
        assert!(NotificationRule::Contacts.allows(true));
        assert!(!NotificationRule::Contacts.allows(false));
    }
}
