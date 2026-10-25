use super::*;
use crate::mail::types::Sender;
use crate::mail::MailError;

fn rule(account: &str, sender: &str, target: &str) -> FolderRule {
    FolderRule {
        account: account.into(),
        sender: sender.into(),
        target: target.into(),
        uid_validity: 7,
        first_uid: 100,
        suspended: None,
    }
}

fn snapshot(rules: Vec<FolderRule>, validity: Option<u32>) -> RoutingSnapshot {
    let inbox_validity = rules
        .iter()
        .map(|r| (r.account.clone(), validity))
        .collect();
    RoutingSnapshot {
        rules,
        inbox_validity,
    }
}

fn make(id: &str, account: &str, folder: &str, sender: &str) -> Envelope {
    Envelope {
        id: id.into(),
        account: Some(account.into()),
        folder: Some(folder.into()),
        sender: Sender::Plain(format!("Someone <{sender}>")),
        flags: vec![],
        subject: "Mail".into(),
        date: "1990-01-01".into(),
        message_id: None,
        in_reply_to: None,
    }
}

#[test]
fn routing_moves_only_matching_future_mail_and_reports_server_failure_per_rule() {
    let snap = snapshot(
        vec![
            rule("work", "alice@example.org", "Reading"),
            rule("work", "bob@example.org", "Receipts"),
        ],
        Some(7),
    );
    let mut no_account = make("150", "work", "INBOX", "alice@example.org");
    no_account.account = None;
    let rows = vec![
        make("99", "work", "INBOX", "alice@example.org"),
        make("100", "home", "INBOX", "alice@example.org"),
        make("100", "work", "Sent", "alice@example.org"),
        make("nonnumeric", "work", "INBOX", "alice@example.org"),
        no_account,
        make("100", "work", "INBOX", "ALICE@example.org"),
        make("101", "work", "INBOX", "bob@example.org"),
        make("102", "work", "INBOX", "bob@example.org"),
    ];
    let mut moved = Vec::new();
    let (visible, report) = route(
        &snap,
        rows,
        |_| Ok(Some(7)),
        |a, f, t, id| {
            if t == "Receipts" {
                return Err(MailError::other("server refused move"));
            }
            moved.push((a.to_string(), f.to_string(), t.to_string(), id.to_string()));
            Ok(())
        },
    );
    assert_eq!(
        moved,
        [(
            "work".into(),
            "INBOX".into(),
            "Reading".into(),
            "100".into()
        )]
    );
    // Failed mail is still in INBOX, so it stays listed; one failure stops the rule.
    assert_eq!(visible.len(), 7);
    assert_eq!(report.moved, [(snap.rules[0].clone(), 1)]);
    assert_eq!(report.failed.len(), 1);
    assert_eq!(report.failed[0].0, snap.rules[1]);
    assert!(report.failed[0].1.contains("server refused move"));
    assert!(report.reset.is_empty());
}

#[test]
fn a_changed_uidvalidity_reports_reset_and_moves_nothing() {
    let rows = || vec![make("100", "work", "INBOX", "alice@example.org")];
    let rules = vec![rule("work", "alice@example.org", "Reading")];
    let no_moves = |_: &str, _: &str, _: &str, _: &str| -> MailResult<()> {
        panic!("a reset mailbox must not move mail")
    };
    for (stored, live) in [(Some(8), Some(7)), (None, Some(7)), (Some(7), Some(8))] {
        let (visible, report) = route(
            &snapshot(rules.clone(), stored),
            rows(),
            |_| Ok(live),
            no_moves,
        );
        assert_eq!(visible.len(), 1);
        assert_eq!(report.reset, rules);
    }
    let (visible, report) = route(
        &snapshot(rules.clone(), Some(7)),
        rows(),
        |_| Err(MailError::other("offline")),
        no_moves,
    );
    assert_eq!(visible.len(), 1);
    assert!(report.failed[0].1.contains("offline"));
}
