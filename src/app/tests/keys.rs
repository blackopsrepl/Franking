/*! Key material overlay tests. */

use crate::app::pgp_keys::KeyPrompt;
use crate::keys::View;

use super::super::App;

/// A private keys directory for the duration of `body`.
///
/// The variable is process-wide, so these tests take a lock: two of them
/// running at once would otherwise share whichever directory was set last.
fn with_keys_dir(name: &str, body: impl FnOnce(&std::path::Path)) {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    let _guard = LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let dir = std::env::temp_dir().join(format!("sfm-appkeys-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let previous = std::env::var_os("SOLVERFORGE_MAIL_KEYS_DIR");
    std::env::set_var("SOLVERFORGE_MAIL_KEYS_DIR", &dir);
    body(&dir);
    match previous {
        Some(value) => std::env::set_var("SOLVERFORGE_MAIL_KEYS_DIR", value),
        None => std::env::remove_var("SOLVERFORGE_MAIL_KEYS_DIR"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn generating_a_key_lists_it_and_export_writes_the_public_half() {
    with_keys_dir("generate", |dir| {
        let mut app = App::new(None);
        app.open_keys();
        assert_eq!(app.view, View::Keys);
        assert!(app.keys.keys.is_empty(), "nothing stored yet");

        app.keys_generate();
        assert_eq!(app.view, View::KeysPrompt);
        for c in "Alice <alice@example.com>".chars() {
            app.keys_input(c);
        }
        app.keys_submit();

        assert_eq!(app.view, View::Keys, "the overlay comes back");
        assert_eq!(app.keys.keys.len(), 2, "a public and a secret file");
        assert!(
            app.status_message.contains("Generated a key pair"),
            "status: {}",
            app.status_message
        );

        // Select the public half and export it.
        app.keys.index = app
            .keys
            .keys
            .iter()
            .position(|key| !key.secret)
            .expect("public half");
        app.keys_export();
        let exported = std::fs::read_dir(dir)
            .unwrap()
            .flatten()
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".export.asc"))
            .count();
        assert_eq!(
            exported, 1,
            "the public key was written out (status: {})",
            app.status_message
        );
    });
}

#[test]
fn importing_a_file_reports_the_key_and_rejects_junk() {
    with_keys_dir("import", |dir| {
        let mut app = App::new(None);

        // A file that is not a key.
        let junk = dir.join("junk.asc");
        std::fs::write(&junk, "not a key").unwrap();
        app.open_keys();
        app.keys_import();
        for c in junk.to_string_lossy().chars() {
            app.keys_input(c);
        }
        app.keys_submit();
        assert!(
            app.status_message.contains("Could not import"),
            "status: {}",
            app.status_message
        );
        assert!(app.keys.keys.is_empty());
    });
}

#[test]
fn deleting_a_key_requires_confirmation() {
    with_keys_dir("delete", |_dir| {
        let mut app = App::new(None);
        app.open_keys();
        app.keys_generate();
        for c in "Bob <bob@example.com>".chars() {
            app.keys_input(c);
        }
        app.keys_submit();
        assert_eq!(app.keys.keys.len(), 2);

        app.keys_delete();
        assert!(
            app.status_message.contains("Press d again"),
            "the first press only asks: {}",
            app.status_message
        );
        assert_eq!(app.keys.keys.len(), 2, "nothing removed yet");

        app.keys_delete();
        assert!(
            app.keys.keys.is_empty(),
            "the pair is gone: {:?}",
            app.keys.keys
        );
        assert!(!app.status_is_error);
    });
}

#[test]
fn cancelling_a_prompt_leaves_the_listing_alone() {
    with_keys_dir("cancel", |_dir| {
        let mut app = App::new(None);
        app.open_keys();
        app.keys_import();
        assert_eq!(app.view, View::KeysPrompt);
        app.keys_input('x');
        app.keys_cancel();
        assert_eq!(app.view, View::Keys);
        assert_eq!(app.keys.prompt, None);
        assert!(app.keys.input.is_empty());
        assert_eq!(app.keys.prompt, None::<KeyPrompt>);
    });
}
