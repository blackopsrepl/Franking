/*! The attachment library indexes cached mail across accounts. */

use crate::app::App;
use crate::keys::View;

#[test]
fn opening_the_library_lists_cached_attachments() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    crate::db::init_for_test(&conn).unwrap();
    let raw = b"MIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=\"m\"\r\n\r\n--m\r\nContent-Type: text/plain\r\n\r\nhello\r\n--m\r\nContent-Type: application/pdf; name=\"report.pdf\"\r\nContent-Disposition: attachment; filename=\"report.pdf\"\r\nContent-Transfer-Encoding: base64\r\n\r\nAAECAwQ=\r\n--m--\r\n";
    conn.execute(
        "INSERT INTO messages (account, folder, uid, message_id, subject, from_display, has_attachments, raw)
         VALUES ('work', 'INBOX', '5', 'five@x', 'Report', 'Alice', 1, ?1)",
        rusqlite::params![raw.to_vec()],
    )
    .unwrap();
    let mut app = App::new(Some("work".into()));
    app.db = Some(conn);
    app.open_attachment_library();
    assert_eq!(app.view, View::AttachmentLibrary);
    assert_eq!(app.attachment_library.items.len(), 1);
    assert_eq!(app.attachment_library.items[0].file_name, "report.pdf");
}
