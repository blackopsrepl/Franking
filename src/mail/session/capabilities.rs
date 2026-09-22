/*! Server capability decoding, from the codec's typed capability list. */

use imap_types::response::Capability;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Capabilities {
    pub names: Vec<String>,
    pub imap4rev1: bool,
    pub idle: bool,
    pub move_: bool,
    pub uidplus: bool,
    pub qresync: bool,
    pub condstore: bool,
    pub special_use: bool,
    pub sort: bool,
    pub thread: bool,
    pub utf8_accept: bool,
    pub compress_deflate: bool,
    pub auth_plain: bool,
    pub auth_xoauth2: bool,
}

impl Capabilities {
    pub fn from_names<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let names = names
            .into_iter()
            .map(|name| name.as_ref().to_string())
            .collect::<Vec<_>>();
        let has = |capability: &str| {
            names
                .iter()
                .any(|name| name.eq_ignore_ascii_case(capability))
        };
        Self {
            imap4rev1: has("IMAP4rev1"),
            idle: has("IDLE"),
            move_: has("MOVE"),
            uidplus: has("UIDPLUS"),
            qresync: has("QRESYNC"),
            condstore: has("CONDSTORE"),
            special_use: has("SPECIAL-USE") || has("SPECIAL-USE=EXTENDED"),
            sort: has("SORT"),
            thread: has("THREAD"),
            utf8_accept: has("UTF8=ACCEPT"),
            compress_deflate: has("COMPRESS=DEFLATE"),
            auth_plain: has("AUTH=PLAIN"),
            auth_xoauth2: has("AUTH=XOAUTH2"),
            names,
        }
    }

    pub fn from_capabilities(capabilities: &[Capability<'_>]) -> Self {
        let names = capabilities
            .iter()
            .map(|capability| match capability {
                Capability::Imap4Rev1 => "IMAP4rev1".to_string(),
                Capability::Auth(mechanism) => format!("AUTH={mechanism}"),
                Capability::Sort(Some(algorithm)) => format!("SORT={algorithm}"),
                Capability::Thread(algorithm) => format!("THREAD={algorithm}"),
                Capability::Idle => "IDLE".to_string(),
                Capability::Move => "MOVE".to_string(),
                Capability::UidPlus => "UIDPLUS".to_string(),
                Capability::CondStore => "CONDSTORE".to_string(),
                Capability::QResync => "QRESYNC".to_string(),
                Capability::StartTls => "STARTTLS".to_string(),
                Capability::LoginDisabled => "LOGINDISABLED".to_string(),
                Capability::Other(other) => {
                    // The wrapped atom is private; its Debug form names it.
                    let debug = format!("{other:?}");
                    debug.split('"').nth(1).unwrap_or_default().to_string()
                }
                other => format!("{other:?}"),
            })
            .collect::<Vec<_>>();
        Self::from_names(names)
    }

    pub fn supports(&self, capability: &str) -> bool {
        self.names
            .iter()
            .any(|name| name.eq_ignore_ascii_case(capability))
    }
}
