/*! Maildir service type and construction. */

use std::path::{Path, PathBuf};

use crate::mail::errors::MailResult;

use super::fs_ops::{
    default_test_maildir_path, ensure_maildir_structure, folder_path, seed_demo_message,
};

pub struct MaildirService {
    pub(super) account_name: String,
    pub(super) root: PathBuf,
    pub(super) is_default: bool,
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

    pub(super) fn ensure_ready(&self) -> MailResult<()> {
        ensure_maildir_structure(&self.root)?;
        seed_demo_message(&self.root)?;
        Ok(())
    }

    pub(super) fn folder_path(&self, folder: &str) -> MailResult<PathBuf> {
        folder_path(&self.root, folder)
    }
}
