//! Archive-writing tests.

use super::save_attachments_as_zip;

/// A private directory for one test.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("sfm-zip-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn archives_every_payload_and_reads_back() {
    use std::io::Read;

    let dir = scratch("all");
    let payloads = vec![
        ("report.pdf".to_string(), b"pdf bytes".to_vec()),
        ("data.csv".to_string(), b"a,b\n1,2\n".to_vec()),
    ];
    let path = save_attachments_as_zip(payloads, &dir, "Quarterly report").unwrap();
    assert!(
        path.ends_with("Quarterly report.zip"),
        "named after the subject: {path}"
    );

    let file = std::fs::File::open(&path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    assert_eq!(archive.len(), 2);

    {
        let mut report = archive.by_name("report.pdf").unwrap();
        let mut content = Vec::new();
        report.read_to_end(&mut content).unwrap();
        assert_eq!(content, b"pdf bytes");
    }
    assert!(archive.by_name("data.csv").is_ok());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn archives_without_attachments_are_refused() {
    let dir = scratch("empty");
    let error = save_attachments_as_zip(Vec::new(), &dir, "Nothing").expect_err("refused");
    assert!(error.to_string().contains("does not include"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_archive_name_is_sanitized_and_never_overwrites() {
    let dir = scratch("unique");
    let payloads = vec![("a.txt".to_string(), b"one".to_vec())];
    let first = save_attachments_as_zip(payloads.clone(), &dir, "a/b:c").unwrap();
    assert!(
        first.ends_with("a_b_c.zip"),
        "path separators are replaced: {first}"
    );
    let second = save_attachments_as_zip(payloads, &dir, "a/b:c").unwrap();
    assert_ne!(first, second, "the second archive gets its own name");
    assert!(second.ends_with("a_b_c-2.zip"), "{second}");

    let _ = std::fs::remove_dir_all(&dir);
}
