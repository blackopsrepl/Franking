/*! Local maildir backend module wiring. */

mod flags;
mod fs_ops;
mod model;
mod reply_templates;
mod service;
mod template;

#[cfg(test)]
mod tests;

pub use fs_ops::default_test_maildir_path;
pub use model::MaildirService;
