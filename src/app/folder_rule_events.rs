/*! Sender-rule state owned by the App: routing snapshots out, worker reports in. */

use std::collections::HashMap;

use super::model::App;
use crate::db::folder_rules::{self as store, FolderRule, MAILBOX_RESET};
use crate::mail::MailError;
use crate::worker::{FolderChange, RoutingReport, RoutingSnapshot};

const OPEN_RULES: &str = "Open Actions → Sender folder rules.";

impl App {
    /// Active rules for `account` (every account when `None`) plus each
    /// account's stored INBOX UIDVALIDITY. `None` means no routing work.
    pub(crate) fn rule_snapshot(&mut self, account: Option<&str>) -> Option<RoutingSnapshot> {
        let conn = self.db.as_ref()?;
        let snapshot = (|| -> anyhow::Result<RoutingSnapshot> {
            let rules: Vec<FolderRule> = store::list(conn)?
                .into_iter()
                .filter(|r| r.suspended.is_none() && account.is_none_or(|a| r.account == a))
                .collect();
            let mut inbox_validity = HashMap::new();
            for rule in &rules {
                if !inbox_validity.contains_key(&rule.account) {
                    let state = crate::mail::store::get_sync_state(conn, &rule.account, "INBOX")?;
                    inbox_validity.insert(rule.account.clone(), state.and_then(|s| s.uid_validity));
                }
            }
            Ok(RoutingSnapshot {
                rules,
                inbox_validity,
            })
        })();
        match snapshot {
            Ok(snapshot) => Some(snapshot).filter(|s| !s.rules.is_empty()),
            Err(error) => {
                self.set_error(&format!("Sender folder rules were not applied: {error}"));
                None
            }
        }
    }

    /// Rules route INBOX only; any other folder is listed without them.
    pub(crate) fn inbox_rule_snapshot(&mut self) -> Option<RoutingSnapshot> {
        let account = self.acct_owned()?;
        if !self.current_folder.eq_ignore_ascii_case("INBOX") {
            return None;
        }
        self.rule_snapshot(Some(&account))
    }

    /// Suspend every rule that failed or whose mailbox was reset.
    pub(crate) fn handle_routing_report(&mut self, report: RoutingReport) {
        let problems: Vec<(&FolderRule, &str)> = report
            .failed
            .iter()
            .map(|(rule, error)| (rule, error.as_str()))
            .chain(report.reset.iter().map(|rule| (rule, MAILBOX_RESET)))
            .collect();
        let mut stored = Ok(());
        if let Some(conn) = self.db.as_ref() {
            for (rule, reason) in &problems {
                if let Err(error) = store::suspend(conn, &rule.account, &rule.sender, reason) {
                    stored = Err(error);
                }
            }
        } else if !problems.is_empty() {
            stored = Err(anyhow::anyhow!("Local database is unavailable"));
        }
        if self.folder_rules.open {
            self.reload_folder_rules();
        }
        let moved: usize = report.moved.iter().map(|(_, n)| n).sum();
        if let Err(error) = stored {
            self.set_error(&format!("Could not suspend a failing sender rule: {error}"));
        } else if let Some((rule, reason)) = problems.first() {
            let more = match problems.len() - 1 {
                0 => String::new(),
                n => format!(" (+{n} more)"),
            };
            self.set_error(&format!(
                "Sender rule {} → {} suspended: {reason}{more}. {OPEN_RULES}",
                rule.sender, rule.target
            ));
        } else if moved > 0 {
            let s = if moved == 1 { "" } else { "s" };
            self.set_status(&format!("Moved {moved} message{s} by sender rules."));
        }
    }

    /// Store the opted-in rule only once its move has succeeded on the server.
    pub(crate) fn handle_rule_move_done(
        &mut self,
        rule: FolderRule,
        result: Result<String, MailError>,
    ) {
        match result {
            Ok(message) => {
                let saved = self
                    .db
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Local database is unavailable"))
                    .and_then(|conn| store::save(conn, &rule));
                self.handle_action_done(Ok(message.clone()));
                match saved {
                    Ok(()) => self.set_status(&format!(
                        "{message} Future mail from {} will move there too.",
                        rule.sender
                    )),
                    Err(error) => self.set_error(&format!("{message} Rule not saved: {error}")),
                }
            }
            Err(error) => {
                let text = format!("Rule not saved because the move failed: {error}");
                self.handle_action_done(Err(error));
                self.set_error(&text);
            }
        }
    }

    /// Keep rules pointing at real folders after a successful rename/delete.
    pub(crate) fn handle_folder_changed(
        &mut self,
        change: FolderChange,
        result: Result<String, MailError>,
    ) {
        let message = match result {
            Ok(message) => message,
            Err(error) => return self.handle_action_done(Err(error)),
        };
        let updated = self
            .db
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Local database is unavailable"))
            .and_then(|conn| match &change {
                FolderChange::Renamed {
                    account: Some(account),
                    from,
                    to,
                } => store::retarget(conn, account, from, to).map(|n| (n, "updated")),
                FolderChange::Deleted {
                    account: Some(account),
                    name,
                } => store::delete_targeting(conn, account, name).map(|n| (n, "removed")),
                _ => Ok((0, "updated")),
            });
        self.handle_action_done(Ok(message.clone()));
        match updated {
            Ok((0, _)) => {}
            Ok((n, verb)) => {
                let s = if n == 1 { "" } else { "s" };
                self.set_status(&format!("{message} {n} sender rule{s} {verb}."));
            }
            Err(error) => self.set_error(&format!(
                "{message} Sender rules for it were not updated: {error}. {OPEN_RULES}"
            )),
        }
        if self.folder_rules.open {
            self.reload_folder_rules();
        }
    }
}
