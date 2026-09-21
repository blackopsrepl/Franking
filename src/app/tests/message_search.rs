/*! App unit tests: in-message search. */

#[test]
fn finds_text_inside_the_loaded_message() {
    use crate::keys::View;

    use super::super::App;

    let raw = b"From: alice@example.com\r\nSubject: Findings\r\n\r\nfirst line\r\nneedle here\r\nlast line\r\n";
    let mut document = crate::mail::mime::parse_message(raw).unwrap();
    document.raw = Some(raw.to_vec());

    let mut app = App::new(None);
    app.view = View::MessageView;
    app.last_terminal_width = 80;
    app.message_content = Some(document);

    app.enter_message_search();
    assert!(app.message_search_active);
    assert_eq!(app.view, View::MessageSearch);
    for c in "needle".chars() {
        app.message_search_input(c);
    }
    app.submit_message_search();

    assert!(!app.message_search_active);
    assert_eq!(app.view, View::MessageView);
    assert_eq!(app.message_matches.len(), 1);
    assert!(app.status_message.contains("Match 1/1"));
    assert!(app.message_scroll > 0);
}

#[test]
fn message_search_reports_when_nothing_matches() {
    use crate::keys::View;

    use super::super::App;

    let raw = b"From: alice@example.com\r\nSubject: Nothing\r\n\r\nbody\r\n";
    let mut document = crate::mail::mime::parse_message(raw).unwrap();
    document.raw = Some(raw.to_vec());

    let mut app = App::new(None);
    app.view = View::MessageView;
    app.last_terminal_width = 80;
    app.message_content = Some(document);

    app.enter_message_search();
    for c in "absent".chars() {
        app.message_search_input(c);
    }
    app.submit_message_search();
    assert!(app.message_matches.is_empty());
    assert!(app.status_message.contains("No matches"));
}

#[test]
fn next_and_previous_match_wrap_around() {
    use crate::keys::View;

    use super::super::App;

    let raw = b"From: a@example.com\r\nSubject: Many\r\n\r\nhit one\r\nfiller\r\nhit two\r\nhit three\r\n";
    let mut document = crate::mail::mime::parse_message(raw).unwrap();
    document.raw = Some(raw.to_vec());

    let mut app = App::new(None);
    app.view = View::MessageView;
    app.last_terminal_width = 80;
    app.message_content = Some(document);
    app.message_search = "hit".to_string();
    app.submit_message_search();

    // The renderer joins the body into one wrapped line, so the three
    // occurrences map to the same line offset.
    assert_eq!(app.message_matches.len(), 3);
    assert!(app
        .message_matches
        .windows(2)
        .all(|pair| pair[0] == pair[1]));
    assert_eq!(app.message_match_index, 0);
    app.next_match();
    assert_eq!(app.message_match_index, 1);
    app.next_match();
    app.next_match();
    assert_eq!(app.message_match_index, 0, "wraps forward");
    app.prev_match();
    assert_eq!(app.message_match_index, 2, "wraps backward");
}

#[test]
fn link_list_navigates_and_opens() {
    use crate::keys::View;

    use super::super::App;

    let raw = b"From: a@example.com\r\nSubject: Links\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<p>See <a href=\"https://one.example\">one</a> and <a href=\"https://two.example\">two</a>.</p>\r\n";
    let mut document = crate::mail::mime::parse_message(raw).unwrap();
    document.raw = Some(raw.to_vec());

    let mut app = App::new(None);
    app.view = View::MessageView;
    app.message_content = Some(document);
    assert_eq!(app.message_content.as_ref().unwrap().body.links.len(), 2);

    app.open_links();
    assert_eq!(app.view, View::LinkList);
    assert_eq!(app.link_index, 0);

    app.link_next();
    assert_eq!(app.link_index, 1);
    app.link_next();
    assert_eq!(app.link_index, 1, "clamps at the end");
    app.link_prev();
    assert_eq!(app.link_index, 0);

    app.open_selected_link();
    let command = app.pending_open_command.as_ref().expect("open command");
    assert_eq!(command.args, vec!["https://one.example".to_string()]);

    app.close_links();
    assert_eq!(app.view, View::MessageView);
}

#[test]
fn opening_links_without_any_reports_status() {
    use crate::keys::View;

    use super::super::App;

    let raw = b"From: a@example.com\r\nSubject: Plain\r\n\r\nno links here\r\n";
    let mut document = crate::mail::mime::parse_message(raw).unwrap();
    document.raw = Some(raw.to_vec());

    let mut app = App::new(None);
    app.view = View::MessageView;
    app.message_content = Some(document);
    app.open_links();
    assert_eq!(app.view, View::MessageView);
    assert!(app.status_message.contains("no links"));
}

#[test]
fn quoted_lines_collapse_on_request() {
    use crate::keys::View;

    use super::super::App;

    let raw = b"From: a@example.com\r\nSubject: Thread\r\n\r\nmy reply\r\n> quoted one\r\n> quoted two\r\nnew text\r\n";
    let mut document = crate::mail::mime::parse_message(raw).unwrap();
    document.raw = Some(raw.to_vec());

    let mut app = App::new(None);
    app.view = View::MessageView;
    app.message_content = Some(document);

    assert!(!app.collapse_quotes);
    app.toggle_quotes();
    assert!(app.collapse_quotes);
    app.toggle_quotes();
    assert!(!app.collapse_quotes);
}
