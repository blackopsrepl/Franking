/*! App unit tests: saving an attachment to a chosen directory. */

#[test]
fn saves_an_attachment_into_a_chosen_directory() {
    use crate::file_picker::{FilePickerState, PickerPurpose};
    use crate::keys::View;

    use super::super::App;

    let raw = b"MIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=\"m\"\r\n\r\n--m\r\nContent-Type: text/plain\r\n\r\nhello\r\n--m\r\nContent-Type: text/plain; name=\"notes.txt\"\r\nContent-Disposition: attachment; filename=\"notes.txt\"\r\n\r\nline one\r\n--m--\r\n";
    let mut document = crate::mail::mime::parse_message(raw).unwrap();
    document.raw = Some(raw.to_vec());

    let target = std::env::temp_dir().join(format!(
        "sfmail-save-as-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    std::fs::create_dir_all(&target).unwrap();

    let mut app = App::new(None);
    app.view = View::MessageView;
    app.message_content = Some(document);
    app.attachment_index = 0;

    app.open_save_attachment_picker();
    assert_eq!(app.view, View::FilePicker);
    app.file_picker = Some(FilePickerState::open_with(
        PickerPurpose::SaveAttachment,
        Some(target.clone()),
    ));

    app.file_picker_enter();
    assert_eq!(app.view, View::AttachmentList);
    assert!(app.status_message.contains("Saved"));

    let saved = std::fs::read_to_string(target.join("notes.txt")).expect("saved payload");
    assert_eq!(saved, "line one");
    let _ = std::fs::remove_dir_all(&target);
}
