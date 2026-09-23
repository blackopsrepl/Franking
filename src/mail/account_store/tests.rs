use rusqlite::Connection;

use super::{
    get_account, get_oauth_state, upsert_account, upsert_oauth_state, AccountConfig,
    OauthStateConfig,
};

#[test]
fn upsert_account_persists_endpoint_and_auth_details() {
    let conn = Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();

    let config = AccountConfig {
        name: "work".to_string(),
        backend_kind: "imap".to_string(),
        provider_kind: "generic".to_string(),
        enabled: true,
        is_default: true,
        maildir_path: None,
        imap_host: Some("imap.example.com".to_string()),
        imap_port: Some(993),
        imap_security: Some("tls".to_string()),
        smtp_host: Some("smtp.example.com".to_string()),
        smtp_port: Some(465),
        smtp_security: Some("tls".to_string()),
        sieve_host: None,
        sieve_port: None,
        sieve_security: None,
        auth_mode: Some("password".to_string()),
        username: Some("alice@example.com".to_string()),
        keyring_imap_secret_id: Some("franking/work/imap".to_string()),
        keyring_smtp_secret_id: Some("franking/work/smtp".to_string()),
    };

    upsert_account(&conn, &config).unwrap();

    let stored = get_account(&conn, "work").unwrap().unwrap();
    assert_eq!(stored.imap_host.as_deref(), Some("imap.example.com"));
    assert_eq!(stored.smtp_port, Some(465));
    assert_eq!(stored.auth_mode.as_deref(), Some("password"));
    assert_eq!(
        stored.keyring_imap_secret_id.as_deref(),
        Some("franking/work/imap")
    );
    assert_eq!(stored.provider_kind, "generic");
    assert!(stored.is_default);
}

#[test]
fn upsert_oauth_state_persists_refresh_metadata() {
    let conn = Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();

    let config = AccountConfig {
        name: "gmail".to_string(),
        backend_kind: "imap".to_string(),
        provider_kind: "gmail".to_string(),
        enabled: true,
        is_default: false,
        maildir_path: None,
        imap_host: Some("imap.gmail.com".to_string()),
        imap_port: Some(993),
        imap_security: Some("tls".to_string()),
        smtp_host: Some("smtp.gmail.com".to_string()),
        smtp_port: Some(587),
        smtp_security: Some("starttls".to_string()),
        sieve_host: None,
        sieve_port: None,
        sieve_security: None,
        auth_mode: Some("oauth2".to_string()),
        username: Some("alice@gmail.com".to_string()),
        keyring_imap_secret_id: None,
        keyring_smtp_secret_id: None,
    };
    upsert_account(&conn, &config).unwrap();

    let oauth = OauthStateConfig {
        provider_kind: "gmail".to_string(),
        client_id: "client".to_string(),
        client_secret_ref: Some("franking/gmail/client-secret".to_string()),
        refresh_token_ref: "franking/gmail/refresh-token".to_string(),
        access_token_cached: Some("access".to_string()),
        access_token_expires_at: Some("2026-04-13T10:00:00+00:00".to_string()),
        scopes: "scope1 scope2".to_string(),
        token_endpoint: "https://token.example.com".to_string(),
        auth_endpoint: "https://auth.example.com".to_string(),
    };
    upsert_oauth_state(&conn, "gmail", &oauth).unwrap();

    let stored = get_oauth_state(&conn, "gmail").unwrap().unwrap();
    assert_eq!(stored.client_id, "client");
    assert_eq!(
        stored.client_secret_ref.as_deref(),
        Some("franking/gmail/client-secret")
    );
    assert_eq!(stored.refresh_token_ref, "franking/gmail/refresh-token");
    assert_eq!(stored.access_token_cached.as_deref(), Some("access"));
}

#[cfg(test)]
mod account_lifecycle {
    use crate::mail::account_store::{
        delete_account, list_accounts, set_default_account, upsert_account, AccountConfig,
    };

    fn second_account() -> AccountConfig {
        AccountConfig {
            name: "work".to_string(),
            backend_kind: "imap".to_string(),
            provider_kind: "generic".to_string(),
            enabled: true,
            is_default: false,
            maildir_path: None,
            imap_host: Some("imap.example.com".to_string()),
            imap_port: Some(993),
            imap_security: Some("tls".to_string()),
            smtp_host: Some("smtp.example.com".to_string()),
            smtp_port: Some(465),
            smtp_security: Some("tls".to_string()),
            sieve_host: None,
            sieve_port: None,
            sieve_security: None,
            auth_mode: Some("password".to_string()),
            username: Some("alice@example.com".to_string()),
            keyring_imap_secret_id: Some("work-imap".to_string()),
            keyring_smtp_secret_id: Some("work-smtp".to_string()),
        }
    }

    #[test]
    fn deletes_an_account_and_its_children() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        upsert_account(&conn, &second_account()).unwrap();
        crate::db::sender_routes::set(
            &conn,
            "work",
            "alice@example.org",
            crate::db::sender_routes::Route::Blocked,
        )
        .unwrap();
        assert!(list_accounts(&conn)
            .unwrap()
            .iter()
            .any(|record| record.name == "work"));

        delete_account(&conn, "work").unwrap();
        assert!(!list_accounts(&conn)
            .unwrap()
            .iter()
            .any(|record| record.name == "work"));

        let endpoints: i64 = conn
            .query_row("SELECT COUNT(*) FROM account_endpoints", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(endpoints, 1, "only the seeded account endpoint remains");
        assert_eq!(
            crate::db::sender_routes::get(&conn, "work", "alice@example.org").unwrap(),
            crate::db::sender_routes::Route::Screening
        );
    }

    #[test]
    fn setting_the_default_clears_the_previous_one() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();
        upsert_account(&conn, &second_account()).unwrap();

        set_default_account(&conn, "work").unwrap();
        let accounts = list_accounts(&conn).unwrap();
        let defaults: Vec<&str> = accounts
            .iter()
            .filter(|record| record.is_default)
            .map(|record| record.name.as_str())
            .collect();
        assert_eq!(defaults, vec!["work"]);
    }
}

#[cfg(test)]
mod sieve_endpoints {
    use crate::mail::account_store::{get_account, upsert_account, AccountConfig};

    #[test]
    fn persists_explicit_sieve_settings() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::init_for_test(&conn).unwrap();

        let config = AccountConfig {
            name: "work".to_string(),
            backend_kind: "imap".to_string(),
            provider_kind: "generic".to_string(),
            enabled: true,
            is_default: false,
            maildir_path: None,
            imap_host: Some("imap.example.com".to_string()),
            imap_port: Some(993),
            imap_security: Some("tls".to_string()),
            smtp_host: Some("smtp.example.com".to_string()),
            smtp_port: Some(465),
            smtp_security: Some("tls".to_string()),
            sieve_host: Some("sieve.example.com".to_string()),
            sieve_port: Some(14190),
            sieve_security: Some("tls".to_string()),
            auth_mode: Some("password".to_string()),
            username: Some("alice@example.com".to_string()),
            keyring_imap_secret_id: Some("work-imap".to_string()),
            keyring_smtp_secret_id: Some("work-smtp".to_string()),
        };
        upsert_account(&conn, &config).unwrap();

        let loaded = get_account(&conn, "work").unwrap().expect("account");
        assert_eq!(loaded.sieve_host.as_deref(), Some("sieve.example.com"));
        assert_eq!(loaded.sieve_port, Some(14190));
        assert_eq!(loaded.sieve_security.as_deref(), Some("tls"));
    }
}
