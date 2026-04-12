pub mod account_store;
pub mod errors;
mod himalaya;
pub mod maildir;
pub mod message;
pub mod mime;
pub mod service;
pub mod types;

pub use errors::{MailError, MailErrorKind, MailResult};
pub use message::{MessageAttachment, MessageContent, MessageDisplayMode, MessageHeader};
pub use service::{default_mail_service, MailService};
pub use types::{Account, Envelope, Folder, Sender};
