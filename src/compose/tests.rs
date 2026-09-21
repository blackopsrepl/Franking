/*! Compose template unit tests. */

use super::state::{ComposeMode, ComposeState};
use super::template::{populate_from_template, reassemble_template};

#[test]
fn template_round_trip_preserves_threading_headers() {
    let raw = "To: bob@example.com\nSubject: Re: Project\nIn-Reply-To: <child@example.com>\nReferences: <root@example.com> <child@example.com>\n\nquoted body";
    let mut state = ComposeState::new(ComposeMode::Reply, Some("work".to_string()));

    populate_from_template(&mut state, raw);

    assert_eq!(state.in_reply_to.as_deref(), Some("<child@example.com>"));
    assert_eq!(
        state.references.as_deref(),
        Some("<root@example.com> <child@example.com>")
    );

    let rebuilt = reassemble_template(&state);
    assert!(rebuilt.contains("In-Reply-To: <child@example.com>"));
    assert!(rebuilt.contains("References: <root@example.com> <child@example.com>"));
}
