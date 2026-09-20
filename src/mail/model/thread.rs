/*! Threading metadata.
Message identity and relations come from `Message-ID`, `In-Reply-To`, and
`References` (RFC 5322), with an RFC 5256 base subject for grouping fallback. */

use super::headers::DecodedHeaders;

const REPLY_PREFIXES: [&str; 4] = ["re", "fwd", "fw", "aw"];

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ThreadRefs {
    pub message_id: Option<String>,
    pub in_reply_to: Vec<String>,
    pub references: Vec<String>,
    pub base_subject: String,
    pub is_reply: bool,
    pub is_forward: bool,
}

impl ThreadRefs {
    pub fn from_headers(headers: &DecodedHeaders) -> Self {
        let subject = headers.subject.as_deref().unwrap_or_default();
        let is_reply = has_reply_prefix(subject) || !headers.in_reply_to.is_empty();
        Self {
            message_id: headers.message_id.clone(),
            in_reply_to: headers.in_reply_to.clone(),
            references: headers.references.clone(),
            base_subject: base_subject(subject),
            is_reply,
            is_forward: has_forward_prefix(subject),
        }
    }

    /// Best-effort parent id: the last `References` entry, else `In-Reply-To`.
    pub fn parent_id(&self) -> Option<&str> {
        self.references
            .last()
            .or_else(|| self.in_reply_to.last())
            .map(String::as_str)
    }

    /// Best-effort thread root: the first `References` entry, else the parent.
    pub fn root_id(&self) -> Option<&str> {
        self.references
            .first()
            .or_else(|| self.in_reply_to.first())
            .map(String::as_str)
    }
}

/// RFC 5256 §2.1 base subject: strip reply/forward prefixes and list tags.
pub fn base_subject(subject: &str) -> String {
    let mut result = subject.trim().to_string();

    loop {
        let stripped = strip_one_list_tag(&strip_reply_prefix(&result));
        if stripped == result {
            break;
        }
        result = stripped;
    }

    let lowered = result.to_ascii_lowercase();
    if let Some(trimmed) = lowered.strip_suffix("(fwd)") {
        result = result[..trimmed.len()].trim_end().to_string();
    }

    result
}

fn strip_reply_prefix(value: &str) -> String {
    let lowered = value.to_ascii_lowercase();
    for prefix in REPLY_PREFIXES {
        let marker = format!("{prefix}:");
        if lowered.starts_with(&marker) {
            return value[marker.len()..].trim_start().to_string();
        }
    }
    value.to_string()
}

fn strip_one_list_tag(value: &str) -> String {
    let Some(inner) = value.strip_prefix('[') else {
        return value.to_string();
    };
    let Some(close) = inner.find(']') else {
        return value.to_string();
    };
    if close > 64 {
        return value.to_string();
    }
    inner[close + 1..].trim_start().to_string()
}

fn has_reply_prefix(subject: &str) -> bool {
    let lowered = subject.trim_start().to_ascii_lowercase();
    lowered.starts_with("re:") || lowered.starts_with("aw:")
}

fn has_forward_prefix(subject: &str) -> bool {
    let lowered = subject.trim_start().to_ascii_lowercase();
    lowered.starts_with("fwd:") || lowered.starts_with("fw:")
}

#[cfg(test)]
mod tests {
    use super::base_subject;

    #[test]
    fn base_subject_strips_reply_and_forward_prefixes() {
        assert_eq!(
            base_subject("Re: Fwd: Re: Project update"),
            "Project update"
        );
        assert_eq!(base_subject("[list] Re: Project update"), "Project update");
        assert_eq!(base_subject("Project update (fwd)"), "Project update");
    }
}
