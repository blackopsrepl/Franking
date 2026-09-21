use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Local;

use super::errors::{MailError, MailResult};
use super::mime;
use super::model::MessageDocument;
use super::service::MailService;
use super::types::{Account, Envelope, Folder, Sender};

const SAMPLE_MESSAGE: &str = include_str!("../../tests/fixtures/message.txt");

#[derive(Debug, Clone)]
pub struct MaildirService {
    account_name: String,
    root: PathBuf,
    is_default: bool,
}

impl MaildirService {
    pub fn new(account_name: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self {
            account_name: account_name.into(),
            root: root.into(),
            is_default: false,
        }
    }

    pub fn default_test() -> Self {
        Self::new("test", default_test_maildir_path()).with_default(true)
    }

    pub fn with_default(mut self, value: bool) -> Self {
        self.is_default = value;
        self
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn ensure_ready(&self) -> MailResult<()> {
        ensure_maildir_structure(&self.root)?;
        seed_demo_message(&self.root)?;
        Ok(())
    }

    fn folder_path(&self, folder: &str) -> MailResult<PathBuf> {
        folder_path(&self.root, folder)
    }
}

impl MailService for MaildirService {
    fn list_accounts(&self) -> MailResult<Vec<Account>> {
        self.ensure_ready()?;
        Ok(vec![Account {
            name: self.account_name.clone(),
            backend: "maildir".to_string(),
            default: self.is_default,
        }])
    }

    fn probe_account(&self, account: &str) -> MailResult<()> {
        if account != self.account_name {
            return Err(MailError::account_not_found(account.to_string()));
        }
        self.ensure_ready()
    }

    fn list_folders(&self, _account: Option<&str>) -> MailResult<Vec<Folder>> {
        self.ensure_ready()?;
        Ok(vec![
            Folder {
                name: "INBOX".to_string(),
                desc: Some("Incoming messages".to_string()),
            },
            Folder {
                name: "Sent".to_string(),
                desc: Some("Sent messages".to_string()),
            },
            Folder {
                name: "Drafts".to_string(),
                desc: Some("Draft messages".to_string()),
            },
            Folder {
                name: "Trash".to_string(),
                desc: Some("Deleted messages".to_string()),
            },
        ])
    }

    fn list_envelopes(
        &self,
        _account: Option<&str>,
        folder: &str,
        page: usize,
        page_size: usize,
        query: Option<&str>,
    ) -> MailResult<Vec<Envelope>> {
        self.ensure_ready()?;
        let dir = self.folder_path(folder)?;
        let mut entries = list_message_entries(&dir)?;
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.sort_key));

        let filtered: Vec<Envelope> = entries
            .into_iter()
            .filter(|entry| matches_query(entry, query))
            .map(|entry| entry.envelope)
            .collect();

        let start = page.saturating_sub(1) * page_size;
        Ok(filtered
            .into_iter()
            .skip(start)
            .take(page_size)
            .collect::<Vec<_>>())
    }

    fn list_envelopes_threaded(
        &self,
        account: Option<&str>,
        folder: &str,
        query: Option<&str>,
    ) -> MailResult<Vec<Envelope>> {
        self.list_envelopes(account, folder, 1, usize::MAX, query)
    }

    fn read_message_raw(
        &self,
        _account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<Vec<u8>> {
        self.ensure_ready()?;
        let dir = self.folder_path(folder)?;
        let path = find_message_path(&dir, id)?;
        let raw =
            fs::read(&path).map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
        mark_seen(&path)?;
        Ok(raw)
    }

    fn delete_message(&self, _account: Option<&str>, folder: &str, id: &str) -> MailResult<()> {
        self.ensure_ready()?;
        if folder.eq_ignore_ascii_case("Trash") {
            let dir = self.folder_path(folder)?;
            let path = find_message_path(&dir, id)?;
            fs::remove_file(path)
                .map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
            return Ok(());
        }

        self.move_message(None, folder, "Trash", id)
    }

    fn move_message(
        &self,
        _account: Option<&str>,
        folder: &str,
        target: &str,
        id: &str,
    ) -> MailResult<()> {
        self.ensure_ready()?;
        let source_dir = self.folder_path(folder)?;
        let source = find_message_path(&source_dir, id)?;
        let target_dir = self.folder_path(target)?;
        let flags = parse_flag_codes(&source);
        let destination = next_message_path(&target_dir, &flags);
        fs::rename(&source, &destination)
            .map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
        Ok(())
    }

    fn copy_message(
        &self,
        _account: Option<&str>,
        folder: &str,
        target: &str,
        id: &str,
    ) -> MailResult<()> {
        self.ensure_ready()?;
        let source_dir = self.folder_path(folder)?;
        let source = find_message_path(&source_dir, id)?;
        let target_dir = self.folder_path(target)?;
        let flags = parse_flag_codes(&source);
        let destination = next_message_path(&target_dir, &flags);
        fs::copy(&source, &destination)
            .map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
        Ok(())
    }

    fn flag_add(
        &self,
        _account: Option<&str>,
        folder: &str,
        id: &str,
        flag: &str,
    ) -> MailResult<()> {
        self.ensure_ready()?;
        let dir = self.folder_path(folder)?;
        let path = find_message_path(&dir, id)?;
        update_flag(&path, flag, true)
    }

    fn flag_remove(
        &self,
        _account: Option<&str>,
        folder: &str,
        id: &str,
        flag: &str,
    ) -> MailResult<()> {
        self.ensure_ready()?;
        let dir = self.folder_path(folder)?;
        let path = find_message_path(&dir, id)?;
        update_flag(&path, flag, false)
    }

    fn download_attachments(
        &self,
        _account: Option<&str>,
        _folder: &str,
        _id: &str,
    ) -> MailResult<String> {
        Err(MailError::unsupported_feature(
            "attachment extraction is not implemented for the local test backend",
        ))
    }

    fn template_write(&self, _account: Option<&str>) -> MailResult<String> {
        self.ensure_ready()?;
        Ok("\n".to_string())
    }

    fn template_reply(
        &self,
        _account: Option<&str>,
        folder: &str,
        id: &str,
        all: bool,
    ) -> MailResult<String> {
        self.ensure_ready()?;
        let original = read_parsed_message(&find_message_path(&self.folder_path(folder)?, id)?)?;
        let to = original
            .header_value("Reply-To")
            .map(str::to_string)
            .or_else(|| original.header_value("From").map(str::to_string))
            .unwrap_or_default();
        let cc = if all {
            original.header_value("Cc").unwrap_or_default().to_string()
        } else {
            String::new()
        };
        let subject = reply_subject(original.header_value("Subject").map(str::to_string));
        let body = quoted_reply_body(&original);

        let mut headers: Vec<(&str, String)> = vec![("To", to), ("Cc", cc), ("Subject", subject)];
        headers.extend(original.thread.reply_headers());
        Ok(render_template(&headers, &body))
    }

    fn template_forward(
        &self,
        _account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<String> {
        self.ensure_ready()?;
        let original = read_parsed_message(&find_message_path(&self.folder_path(folder)?, id)?)?;
        let subject = forward_subject(original.header_value("Subject").map(str::to_string));
        let body = forwarded_body(&original);

        Ok(render_template(&[("Subject", subject)], &body))
    }

    fn template_send(&self, _account: Option<&str>, template: &str) -> MailResult<String> {
        self.ensure_ready()?;
        let raw = render_outgoing(&parse_template_message(template));
        let sent_dir = self.folder_path("Sent")?;
        let destination = next_message_path(&sent_dir, &['S']);
        fs::write(&destination, raw)
            .map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
        Ok("Message sent.".to_string())
    }

    fn save_draft(&self, _account: Option<&str>, template: &str) -> MailResult<String> {
        self.ensure_ready()?;
        let raw = render_outgoing(&parse_template_message(template));
        let drafts_dir = self.folder_path("Drafts")?;
        let destination = next_message_path(&drafts_dir, &['D']);
        fs::write(&destination, raw)
            .map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
        Ok("Draft saved.".to_string())
    }

    fn folder_unread(&self, account: Option<&str>, folder: &str) -> MailResult<usize> {
        self.list_envelopes(account, folder, 1, usize::MAX, Some("not flag seen"))
            .map(|envelopes| envelopes.len())
    }
}

pub fn default_test_maildir_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("solverforge")
        .join("test-maildir")
}

fn ensure_maildir_structure(root: &Path) -> MailResult<()> {
    ensure_maildir_dir(root)?;
    for folder in [".Sent", ".Drafts", ".Trash"] {
        ensure_maildir_dir(&root.join(folder))?;
    }
    Ok(())
}

fn ensure_maildir_dir(path: &Path) -> MailResult<()> {
    for child in ["cur", "new", "tmp"] {
        fs::create_dir_all(path.join(child))
            .map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
    }
    Ok(())
}

fn seed_demo_message(root: &Path) -> MailResult<()> {
    let inbox_new = root.join("new");
    let has_messages = fs::read_dir(&inbox_new)
        .map_err(|err| MailError::local_maildir_failure(err.to_string()))?
        .next()
        .transpose()
        .map_err(|err| MailError::local_maildir_failure(err.to_string()))?
        .is_some();

    let inbox_cur = root.join("cur");
    let has_cur_messages = fs::read_dir(&inbox_cur)
        .map_err(|err| MailError::local_maildir_failure(err.to_string()))?
        .next()
        .transpose()
        .map_err(|err| MailError::local_maildir_failure(err.to_string()))?
        .is_some();

    if has_messages || has_cur_messages {
        return Ok(());
    }

    let destination = next_message_path(root, &[]);
    fs::write(destination, SAMPLE_MESSAGE)
        .map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
    Ok(())
}

fn folder_path(root: &Path, folder: &str) -> MailResult<PathBuf> {
    match folder {
        "INBOX" => Ok(root.to_path_buf()),
        "Sent" => Ok(root.join(".Sent")),
        "Drafts" => Ok(root.join(".Drafts")),
        "Trash" => Ok(root.join(".Trash")),
        other => Err(MailError::account_not_found(format!(
            "unknown folder {other}"
        ))),
    }
}

fn list_message_entries(dir: &Path) -> MailResult<Vec<MessageEntry>> {
    let mut entries = Vec::new();
    for child in ["cur", "new"] {
        let bucket = dir.join(child);
        let read_dir = fs::read_dir(&bucket)
            .map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
        for item in read_dir {
            let item = item.map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
            if !item
                .file_type()
                .map_err(|err| MailError::local_maildir_failure(err.to_string()))?
                .is_file()
            {
                continue;
            }
            let path = item.path();
            let raw =
                fs::read(&path).map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
            let parsed = mime::parse_message(&raw)?;
            let sort_key = item
                .metadata()
                .ok()
                .and_then(|meta| meta.modified().ok())
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or(0);

            entries.push(MessageEntry {
                sort_key,
                searchable: format!(
                    "{}\n{}\n{}\n{}",
                    parsed.header_value("From").unwrap_or_default(),
                    parsed.header_value("To").unwrap_or_default(),
                    parsed.subject(),
                    parsed.render(78)
                )
                .to_ascii_lowercase(),
                envelope: Envelope {
                    id: file_name(&path)?,
                    flags: flags_to_names(&parse_flag_codes(&path)),
                    subject: parsed.subject().to_string(),
                    sender: Sender::Plain(
                        parsed.header_value("From").unwrap_or_default().to_string(),
                    ),
                    date: parsed.header_value("Date").unwrap_or_default().to_string(),
                },
            });
        }
    }
    Ok(entries)
}

fn matches_query(entry: &MessageEntry, query: Option<&str>) -> bool {
    let Some(query) = query else {
        return true;
    };
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return true;
    }
    if query == "not flag seen" {
        return !entry.envelope.is_seen();
    }
    if query == "flag seen" {
        return entry.envelope.is_seen();
    }

    for part in query.split(" and ") {
        let part = part.trim();
        if let Some(term) = part.strip_prefix("subject ") {
            if !entry
                .envelope
                .subject
                .to_ascii_lowercase()
                .contains(term.trim())
            {
                return false;
            }
        } else if let Some(term) = part.strip_prefix("from ") {
            if !entry
                .envelope
                .sender_display()
                .to_ascii_lowercase()
                .contains(term.trim())
            {
                return false;
            }
        } else if !entry.searchable.contains(part) {
            return false;
        }
    }

    true
}

fn find_message_path(dir: &Path, id: &str) -> MailResult<PathBuf> {
    let base_id = id.split_once(":2,").map(|(base, _)| base).unwrap_or(id);
    for child in ["cur", "new"] {
        let candidate = dir.join(child).join(id);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    for child in ["cur", "new"] {
        let bucket = dir.join(child);
        let read_dir = fs::read_dir(&bucket)
            .map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
        for item in read_dir {
            let item = item.map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
            let path = item.path();
            if !path.is_file() {
                continue;
            }
            if base_message_name(&path)? == base_id {
                return Ok(path);
            }
        }
    }
    Err(MailError::account_not_found(format!(
        "message {id} was not found"
    )))
}

fn mark_seen(path: &Path) -> MailResult<()> {
    update_flag(path, "seen", true)
}

fn update_flag(path: &Path, flag: &str, present: bool) -> MailResult<()> {
    let mut flags = parse_flag_codes(path).into_iter().collect::<BTreeSet<_>>();
    let code = flag_code(flag)?;
    if present {
        flags.insert(code);
    } else {
        flags.remove(&code);
    }

    let parent = path
        .parent()
        .ok_or_else(|| MailError::local_maildir_failure("maildir path is missing a parent"))?;
    let mailbox = parent
        .parent()
        .ok_or_else(|| MailError::local_maildir_failure("maildir path is missing a mailbox"))?;
    let destination = if flags.is_empty() {
        mailbox.join("new").join(base_message_name(path)?)
    } else {
        let rendered = flags.iter().collect::<String>();
        mailbox
            .join("cur")
            .join(format!("{}:2,{}", base_message_name(path)?, rendered))
    };

    if destination != path {
        fs::rename(path, destination)
            .map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
    }
    Ok(())
}

fn flag_code(flag: &str) -> MailResult<char> {
    match flag.to_ascii_lowercase().as_str() {
        "seen" => Ok('S'),
        "flagged" => Ok('F'),
        "answered" => Ok('R'),
        "deleted" => Ok('T'),
        other => Err(MailError::unsupported_feature(format!(
            "maildir flag {other} is not supported"
        ))),
    }
}

fn parse_flag_codes(path: &Path) -> Vec<char> {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return Vec::new();
    };
    let Some((_, suffix)) = name.rsplit_once(":2,") else {
        return Vec::new();
    };
    suffix.chars().collect()
}

fn flags_to_names(flags: &[char]) -> Vec<String> {
    flags
        .iter()
        .filter_map(|flag| match flag {
            'S' => Some("Seen"),
            'F' => Some("Flagged"),
            'R' => Some("Answered"),
            'T' => Some("Deleted"),
            _ => None,
        })
        .map(str::to_string)
        .collect()
}

fn local_message_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("<{nanos}.{}@solverforge.local>", std::process::id())
}

fn next_message_path(mailbox_dir: &Path, flags: &[char]) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let base = format!("{nanos}.{}.solverforge", std::process::id());
    if flags.is_empty() {
        mailbox_dir.join("new").join(base)
    } else {
        let rendered = flags.iter().collect::<String>();
        mailbox_dir.join("cur").join(format!("{base}:2,{rendered}"))
    }
}

fn file_name(path: &Path) -> MailResult<String> {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .ok_or_else(|| MailError::local_maildir_failure("message path is missing a file name"))
}

fn base_message_name(path: &Path) -> MailResult<String> {
    let name = file_name(path)?;
    Ok(name
        .split_once(":2,")
        .map(|(base, _)| base.to_string())
        .unwrap_or(name))
}

fn read_parsed_message(path: &Path) -> MailResult<MessageDocument> {
    let raw = fs::read(path).map_err(|err| MailError::local_maildir_failure(err.to_string()))?;
    mime::parse_message(&raw)
}

fn parse_template_message(raw: &str) -> TemplateMessage {
    let mut headers: Vec<(String, String)> = Vec::new();
    let mut current_key: Option<String> = None;
    let mut body_lines = Vec::new();
    let mut in_body = false;

    for line in raw.lines() {
        if in_body {
            body_lines.push(line);
            continue;
        }

        if line.is_empty() {
            in_body = true;
            continue;
        }

        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(key) = current_key.as_ref() {
                if let Some((_, value)) = headers
                    .iter_mut()
                    .find(|(name, _)| name.eq_ignore_ascii_case(key))
                {
                    if !value.is_empty() {
                        value.push(' ');
                    }
                    value.push_str(line.trim());
                }
            }
            continue;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_ascii_lowercase();
            headers.push((key.clone(), value.trim().to_string()));
            current_key = Some(key);
        }
    }

    TemplateMessage {
        headers,
        body: body_lines.join("\n"),
    }
}

fn render_outgoing(parsed: &TemplateMessage) -> String {
    let header = |name: &str| {
        parsed
            .headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.clone())
            .unwrap_or_default()
    };
    let from = {
        let value = header("from");
        if value.is_empty() {
            "SolverForge Mail <test@solverforge.local>".to_string()
        } else {
            value
        }
    };
    let to = header("to");
    let cc = header("cc");
    let bcc = header("bcc");
    let subject = header("subject");
    let in_reply_to = header("in-reply-to");
    let references = header("references");
    let date = Local::now().format("%Y-%m-%d %H:%M:%S%:z").to_string();

    let mut raw = String::new();
    raw.push_str(&format!("From: {from}\n"));
    if !to.is_empty() {
        raw.push_str(&format!("To: {to}\n"));
    }
    if !cc.is_empty() {
        raw.push_str(&format!("Cc: {cc}\n"));
    }
    if !bcc.is_empty() {
        raw.push_str(&format!("Bcc: {bcc}\n"));
    }
    if !subject.is_empty() {
        raw.push_str(&format!("Subject: {subject}\n"));
    }
    if !in_reply_to.is_empty() {
        raw.push_str(&format!("In-Reply-To: {in_reply_to}\n"));
    }
    if !references.is_empty() {
        raw.push_str(&format!("References: {references}\n"));
    }
    raw.push_str(&format!("Message-ID: {}\n", local_message_id()));
    raw.push_str(&format!("Date: {date}\n\n"));
    raw.push_str(&parsed.body);
    raw
}

fn render_template(headers: &[(&str, String)], body: &str) -> String {
    let mut out = String::new();
    for (name, value) in headers {
        if !value.trim().is_empty() {
            out.push_str(&format!("{name}: {}\n", value.trim()));
        }
    }
    out.push('\n');
    out.push_str(body);
    out
}

fn reply_subject(subject: Option<String>) -> String {
    let subject = subject.unwrap_or_default();
    if subject.to_ascii_lowercase().starts_with("re:") {
        subject
    } else if subject.is_empty() {
        "Re:".to_string()
    } else {
        format!("Re: {subject}")
    }
}

fn forward_subject(subject: Option<String>) -> String {
    let subject = subject.unwrap_or_default();
    if subject.to_ascii_lowercase().starts_with("fwd:") {
        subject
    } else if subject.is_empty() {
        "Fwd:".to_string()
    } else {
        format!("Fwd: {subject}")
    }
}

fn quoted_reply_body(message: &MessageDocument) -> String {
    let from = message.header_value("From").unwrap_or_default();
    let date = message.header_value("Date").unwrap_or_default();
    let intro = match (!date.is_empty(), !from.is_empty()) {
        (true, true) => format!("On {date}, {from} wrote:\n"),
        (false, true) => format!("{from} wrote:\n"),
        _ => "Previous message:\n".to_string(),
    };
    let rendered = message.render(78);
    let quoted = rendered
        .lines()
        .map(|line| format!("> {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let quoted = if rendered.is_empty() {
        String::new()
    } else {
        quoted
    };
    format!("\n{intro}{quoted}")
}

fn forwarded_body(message: &MessageDocument) -> String {
    let mut lines = vec!["---------- Forwarded message ----------".to_string()];
    for header in ["From", "Date", "Subject", "To", "Cc"] {
        if let Some(value) = message.header_value(header) {
            let label = match header {
                "From" => "From",
                "Date" => "Date",
                "Subject" => "Subject",
                "To" => "To",
                "Cc" => "Cc",
                _ => continue,
            };
            lines.push(format!("{label}: {value}"));
        }
    }
    lines.push(String::new());
    lines.push(message.render(78));
    lines.join("\n")
}

#[derive(Debug, Clone)]
struct TemplateMessage {
    headers: Vec<(String, String)>,
    body: String,
}

#[derive(Debug, Clone)]
struct MessageEntry {
    sort_key: u64,
    searchable: String,
    envelope: Envelope,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_maildir() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("solverforge-maildir-test-{unique}"))
    }

    #[test]
    fn maildir_round_trip_supports_read_flag_move_and_send() {
        let root = temp_maildir();
        let service = MaildirService::new("test", &root).with_default(true);
        service.ensure_ready().unwrap();

        let folders = service.list_folders(Some("test")).unwrap();
        assert_eq!(folders[0].name, "INBOX");

        let inbox = service
            .list_envelopes(Some("test"), "INBOX", 1, 50, None)
            .unwrap();
        assert_eq!(inbox.len(), 1);
        assert!(!inbox[0].is_seen());

        let id = inbox[0].id.clone();
        let body = service.read_message(Some("test"), "INBOX", &id).unwrap();
        assert!(body.contains("Project update"));

        let inbox_after_read = service
            .list_envelopes(Some("test"), "INBOX", 1, 50, Some("flag seen"))
            .unwrap();
        assert_eq!(inbox_after_read.len(), 1);

        service
            .flag_add(Some("test"), "INBOX", &id, "flagged")
            .unwrap();
        let flagged = service
            .list_envelopes(Some("test"), "INBOX", 1, 50, None)
            .unwrap();
        assert!(flagged[0].is_flagged());

        service
            .move_message(Some("test"), "INBOX", "Trash", &id)
            .unwrap();
        let trash = service
            .list_envelopes(Some("test"), "Trash", 1, 50, None)
            .unwrap();
        assert_eq!(trash.len(), 1);

        let template = service.template_write(Some("test")).unwrap();
        assert_eq!(template, "\n");
        service
            .template_send(
                Some("test"),
                "To: bob@example.com\nSubject: Test send\n\nHello from SolverForge Mail",
            )
            .unwrap();
        let sent = service
            .list_envelopes(Some("test"), "Sent", 1, 50, None)
            .unwrap();
        assert_eq!(sent.len(), 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reply_template_carries_threading_headers() {
        let root = temp_maildir();
        let service = MaildirService::new("test", &root).with_default(true);
        service.ensure_ready().unwrap();

        let raw = "From: alice@example.com\nTo: bob@example.com\nSubject: Project\nMessage-ID: <child@example.com>\nReferences: <root@example.com>\nDate: 2026-04-13 09:00:00+00:00\n\nbody";
        fs::write(root.join("new").join("threading-message"), raw).unwrap();

        let inbox = service
            .list_envelopes(Some("test"), "INBOX", 1, 50, None)
            .unwrap();
        let id = inbox
            .iter()
            .find(|envelope| envelope.subject == "Project")
            .expect("seeded message should be listed")
            .id
            .clone();

        let template = service
            .template_reply(Some("test"), "INBOX", &id, false)
            .unwrap();

        assert!(
            template.contains("In-Reply-To: <child@example.com>"),
            "{template}"
        );
        assert!(
            template.contains("References: <root@example.com> <child@example.com>"),
            "{template}"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sent_message_receives_a_message_id() {
        let root = temp_maildir();
        let service = MaildirService::new("test", &root).with_default(true);
        service.ensure_ready().unwrap();

        service
            .template_send(Some("test"), "To: bob@example.com\nSubject: Hi\n\nhello")
            .unwrap();

        let sent = service
            .list_envelopes(Some("test"), "Sent", 1, 50, None)
            .unwrap();
        let raw = service
            .read_message_raw(Some("test"), "Sent", &sent[0].id)
            .unwrap();
        let raw = String::from_utf8_lossy(&raw);

        assert!(raw.contains("Message-ID: <"), "{raw}");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn save_draft_writes_to_the_drafts_folder() {
        let root = temp_maildir();
        let service = MaildirService::new("test", &root).with_default(true);
        service.ensure_ready().unwrap();

        service
            .save_draft(
                Some("test"),
                "To: bob@example.com\nSubject: Draft subject\n\nwork in progress",
            )
            .unwrap();

        let drafts = service
            .list_envelopes(Some("test"), "Drafts", 1, 50, None)
            .unwrap();
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].subject, "Draft subject");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn folder_unread_counts_unseen_messages() {
        let root = temp_maildir();
        let service = MaildirService::new("test", &root).with_default(true);
        service.ensure_ready().unwrap();

        assert_eq!(service.folder_unread(Some("test"), "INBOX").unwrap(), 1);

        let inbox = service
            .list_envelopes(Some("test"), "INBOX", 1, 50, None)
            .unwrap();
        service
            .flag_add(Some("test"), "INBOX", &inbox[0].id, "seen")
            .unwrap();
        assert_eq!(service.folder_unread(Some("test"), "INBOX").unwrap(), 0);

        let _ = fs::remove_dir_all(root);
    }
}
