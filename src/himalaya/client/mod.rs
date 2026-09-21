/*! Himalaya client module wiring (legacy migration helper). */

mod accounts;
mod args;
mod exec;
mod messages;
mod templates;

pub use accounts::{configure_account, list_account_names, list_accounts, probe_account};
pub use args::{build_envelope_args, build_read_args};
pub use messages::{
    copy_message, delete_message, download_attachments, flag_add, flag_remove, list_envelopes,
    list_envelopes_threaded, list_folders, move_message, preview_message, read_message,
};
pub use templates::{
    compose_command, forward_command, reply_command, template_forward, template_reply,
    template_send, template_write,
};
