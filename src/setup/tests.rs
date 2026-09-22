use super::wizard::selectable_remote_accounts;
use crate::mail::types::Account;

#[test]
fn only_test_account_has_no_remote_options() {
    let accounts = vec![Account {
        name: "test".to_string(),
        backend: "maildir".to_string(),
        default: false,
    }];

    assert!(selectable_remote_accounts(&accounts).is_empty());
}

#[test]
fn remote_accounts_are_selectable_in_setup() {
    let accounts = vec![
        Account {
            name: "test".to_string(),
            backend: "maildir".to_string(),
            default: false,
        },
        Account {
            name: "work".to_string(),
            backend: "imap".to_string(),
            default: false,
        },
    ];

    assert_eq!(selectable_remote_accounts(&accounts).len(), 1);
}
