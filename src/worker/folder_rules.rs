/*! Sender-rule routing: a separate step after a listing, driven by a snapshot
the App takes from its own database. Listings never read rules or the DB. */

use std::collections::HashMap;

use super::{Worker, WorkerResult};
use crate::db::folder_rules::FolderRule;
use crate::mail::errors::MailResult;
use crate::mail::types::Envelope;
use crate::mail::MailService;

/// Active rules for the listed accounts plus each account's stored INBOX
/// UIDVALIDITY, captured by the App when it requests the listing.
#[derive(Debug, Clone)]
pub struct RoutingSnapshot {
    pub rules: Vec<FolderRule>,
    pub inbox_validity: HashMap<String, Option<u32>>,
}

/// What one routing pass did, per rule.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RoutingReport {
    pub moved: Vec<(FolderRule, usize)>,
    pub failed: Vec<(FolderRule, String)>,
    /// Rules whose INBOX UIDVALIDITY no longer matches: their UIDs are void.
    pub reset: Vec<FolderRule>,
}

impl RoutingReport {
    pub fn is_empty(&self) -> bool {
        self.moved.is_empty() && self.failed.is_empty() && self.reset.is_empty()
    }
}

enum Outcome {
    Active(usize),
    Reset,
    Failed(String),
}

/// Rule matching this envelope by its own account, INBOX source and UID epoch.
fn rule_for(rules: &[FolderRule], envelope: &Envelope) -> Option<usize> {
    let account = envelope.account.as_deref()?;
    if !envelope.folder.as_deref()?.eq_ignore_ascii_case("INBOX") {
        return None;
    }
    let uid = envelope.id.parse::<u32>().ok()?;
    let sender = envelope.sender.mailbox()?;
    rules
        .iter()
        .position(|r| r.account == account && r.sender == sender && uid >= r.first_uid)
}

/// Move matching mail; the returned listing keeps whatever is still in INBOX.
/// `server_validity` re-checks the live UIDVALIDITY before the first move per
/// account, so a reset since the last stored sync never moves the wrong UIDs.
pub(super) fn route(
    snapshot: &RoutingSnapshot,
    envelopes: Vec<Envelope>,
    mut server_validity: impl FnMut(&str) -> MailResult<Option<u32>>,
    mut move_message: impl FnMut(&str, &str, &str, &str) -> MailResult<()>,
) -> (Vec<Envelope>, RoutingReport) {
    let rules = &snapshot.rules;
    let mut outcomes: Vec<Outcome> = rules
        .iter()
        .map(|r| match snapshot.inbox_validity.get(&r.account) {
            Some(Some(v)) if *v == r.uid_validity => Outcome::Active(0),
            _ => Outcome::Reset,
        })
        .collect();
    let mut live: HashMap<String, Result<Option<u32>, String>> = HashMap::new();
    let mut visible = Vec::with_capacity(envelopes.len());
    for envelope in envelopes {
        let Some(i) = rule_for(rules, &envelope) else {
            visible.push(envelope);
            continue;
        };
        let rule = &rules[i];
        if matches!(outcomes[i], Outcome::Active(_)) {
            let validity = live
                .entry(rule.account.clone())
                .or_insert_with(|| server_validity(&rule.account).map_err(|e| e.to_string()));
            match validity {
                Ok(Some(v)) if *v == rule.uid_validity => {}
                Ok(_) => outcomes[i] = Outcome::Reset,
                Err(error) => outcomes[i] = Outcome::Failed(error.clone()),
            }
        }
        let Outcome::Active(count) = &mut outcomes[i] else {
            visible.push(envelope);
            continue;
        };
        // rule_for established provenance; never use the UI's account/folder.
        let folder = envelope.folder.as_deref().unwrap_or("INBOX");
        match move_message(&rule.account, folder, &rule.target, &envelope.id) {
            Ok(()) => *count += 1,
            Err(error) => {
                outcomes[i] = Outcome::Failed(error.to_string());
                visible.push(envelope);
            }
        }
    }
    let mut report = RoutingReport::default();
    for (rule, outcome) in rules.iter().zip(outcomes) {
        match outcome {
            Outcome::Active(0) => {}
            Outcome::Active(n) => report.moved.push((rule.clone(), n)),
            Outcome::Reset => report.reset.push(rule.clone()),
            Outcome::Failed(error) => report.failed.push((rule.clone(), error)),
        }
    }
    (visible, report)
}

/// Route with the real service; `None` means no active rules, so no work.
pub(super) fn route_listing(
    service: &dyn MailService,
    snapshot: Option<&RoutingSnapshot>,
    envelopes: Vec<Envelope>,
) -> (Vec<Envelope>, Option<RoutingReport>) {
    let Some(snapshot) = snapshot.filter(|s| !s.rules.is_empty()) else {
        return (envelopes, None);
    };
    let (visible, report) = route(
        snapshot,
        envelopes,
        |account| Ok(service.folder_sync_cursor(Some(account), "INBOX")?.0),
        |account, folder, target, id| service.move_message(Some(account), folder, target, id),
    );
    (visible, Some(report).filter(|r| !r.is_empty()))
}

/// Send a listing, then its routing report when routing did anything.
pub(super) fn deliver_listing(
    tx: &std::sync::mpsc::Sender<WorkerResult>,
    service: &dyn MailService,
    snapshot: Option<&RoutingSnapshot>,
    result: MailResult<Vec<Envelope>>,
) {
    match result {
        Ok(rows) => {
            let (visible, report) = route_listing(service, snapshot, rows);
            let _ = tx.send(WorkerResult::Envelopes(Ok(visible)));
            if let Some(report) = report {
                let _ = tx.send(WorkerResult::FolderRulesApplied(report));
            }
        }
        Err(error) => {
            let _ = tx.send(WorkerResult::Envelopes(Err(error)));
        }
    }
}

impl Worker {
    /// Move one message; the rule draft is only stored if the move succeeds.
    pub fn move_message_with_rule(
        &self,
        account: Option<String>,
        folder: String,
        id: String,
        rule: FolderRule,
    ) {
        let target = rule.target.clone();
        self.spawn(
            move |service| {
                service
                    .move_message(account.as_deref(), &folder, &target, &id)
                    .map(|()| format!("Moved to {target}."))
            },
            move |result| WorkerResult::RuleMoveDone(rule, result),
        );
    }
}

#[cfg(test)]
#[path = "folder_rules_live.rs"]
mod live_tests;

#[cfg(test)]
#[path = "folder_rule_tests.rs"]
mod tests;
