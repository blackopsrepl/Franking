/*! Shared attachment extraction helpers.
Backends decode raw bytes into named payloads; this module owns naming and
writing them to the user's download directory so every backend behaves the
same. */

use std::path::{Path, PathBuf};

use super::errors::{MailError, MailResult};

/// Write attachment payloads to the default downloads directory.
pub fn save_to_downloads(items: Vec<(String, Vec<u8>)>) -> MailResult<String> {
    save_attachments(items, &default_download_dir())
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

fn default_download_dir() -> PathBuf {
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
    fn saving_nothing_is_an_error() {
        let base = temp_dir();
        assert!(save_attachments(Vec::new(), &base).is_err());
    }
}
