/*! Shared attachment extraction helpers.
Backends decode raw bytes into named payloads; this module owns naming and
writing them to the user's download directory so every backend behaves the
same. */

use std::path::{Path, PathBuf};

use super::errors::{MailError, MailResult};
use super::model::{MessageDocument, PartBody};

/// Named payloads for every attachment in a parsed message, in the same order
/// as [`MessageDocument::attachments`].
///
/// Binary parts contribute their bytes; an attached message contributes its
/// serialized source.
pub fn payloads(document: &MessageDocument) -> Vec<(String, Vec<u8>)> {
    let mut payloads = Vec::new();
    let mut index = 0;
    for part in &document.parts {
        part.walk(&mut |part| {
            if !part.is_attachment() {
                return;
            }
            index += 1;
            let name = part
                .filename
                .clone()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| format!("attachment-{index}"));
            let bytes = match &part.body {
                PartBody::Binary(bytes) => bytes.clone(),
                PartBody::Nested(nested) => nested.raw.clone().unwrap_or_default(),
                _ => return,
            };
            payloads.push((name, bytes));
        });
    }
    payloads
}

/// Write attachment payloads to the default downloads directory.
pub fn save_to_downloads(items: Vec<(String, Vec<u8>)>) -> MailResult<String> {
    save_attachments(items, &downloads_dir())
}

/// Write attachment payloads into `base`, returning the saved paths.
pub fn save_attachments(items: Vec<(String, Vec<u8>)>, base: &Path) -> MailResult<String> {
    if items.is_empty() {
        return Err(MailError::unsupported_feature(
            "this message does not include any downloadable attachments",
        ));
    }

    std::fs::create_dir_all(base).map_err(|err| MailError::io(err.to_string()))?;

    let mut saved = Vec::new();
    for (index, (name, bytes)) in items.into_iter().enumerate() {
        let file_name = unique_file_name(base, index, &name);
        let path = base.join(&file_name);
        std::fs::write(&path, bytes).map_err(|err| MailError::io(err.to_string()))?;
        saved.push(path.display().to_string());
    }

    Ok(saved.join(", "))
}

/// Directory where attachments are written when no explicit base is given.
pub fn downloads_dir() -> PathBuf {
    dirs::download_dir()
        .or_else(dirs::data_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("solverforge-mail")
}

fn unique_file_name(base: &Path, index: usize, requested: &str) -> String {
    let sanitized = sanitize_file_name(requested);
    let candidate = if sanitized.is_empty() {
        format!("attachment-{}", index + 1)
    } else {
        sanitized
    };

    if !base.join(&candidate).exists() {
        return candidate;
    }

    let (stem, extension) = candidate
        .rsplit_once('.')
        .map(|(stem, extension)| (stem.to_string(), Some(extension.to_string())))
        .unwrap_or_else(|| (candidate.clone(), None));

    for suffix in 2..1000 {
        let attempt = match extension.as_deref() {
            Some(extension) => format!("{stem}-{suffix}.{extension}"),
            None => format!("{stem}-{suffix}"),
        };
        if !base.join(&attempt).exists() {
            return attempt;
        }
    }

    candidate
}

fn sanitize_file_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{sanitize_file_name, save_attachments, unique_file_name};
    use std::path::Path;

    fn temp_dir() -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("solverforge-attachments-{nanos}"))
    }

    #[test]
    fn sanitize_replaces_path_separators() {
        assert_eq!(
            sanitize_file_name("report:Q2/2026?.pdf"),
            "report_Q2_2026_.pdf"
        );
    }

    #[test]
    fn unique_name_avoids_collisions() {
        let base = temp_dir();
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("file.txt"), b"one").unwrap();

        assert_eq!(unique_file_name(&base, 0, "file.txt"), "file-2.txt");
        assert_eq!(unique_file_name(&base, 0, ""), "attachment-1");

        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn saving_writes_all_payloads() {
        let base = temp_dir();
        let saved = save_attachments(
            vec![
                ("report.pdf".to_string(), b"pdf".to_vec()),
                ("../evil/name.txt".to_string(), b"text".to_vec()),
            ],
            &base,
        )
        .unwrap();

        let paths: Vec<&Path> = saved.split(", ").map(Path::new).collect();
        assert_eq!(paths.len(), 2);
        for path in &paths {
            assert!(path.exists(), "missing {}", path.display());
            assert_eq!(path.parent().unwrap(), base);
        }
        assert!(saved.contains("report.pdf"));
        assert!(saved.contains(".._evil_name.txt"));

        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn payloads_follow_the_message_attachment_order() {
        let raw = b"MIME-Version: 1.0
Content-Type: multipart/mixed; boundary=\"m\"

--m
Content-Type: text/plain

hello
--m
Content-Type: application/pdf; name=\"report.pdf\"
Content-Disposition: attachment; filename=\"report.pdf\"
Content-Transfer-Encoding: base64

AAECAwQ=
--m--
";
        let document = crate::mail::mime::parse_message(raw).unwrap();
        assert_eq!(document.attachments.len(), 1);

        let payloads = super::payloads(&document);
        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].0, "report.pdf");
        assert_eq!(payloads[0].1, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn saving_nothing_is_an_error() {
        let base = temp_dir();
        assert!(save_attachments(Vec::new(), &base).is_err());
    }
}
