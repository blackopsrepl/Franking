use super::errors::{MailError, MailResult};
use super::message::MessageContent;
use super::mime;
use super::types::{Account, Envelope, Folder};
use crate::himalaya::{client, diagnostics};

#[derive(Debug, Default, Clone, Copy)]
pub struct HimalayaService;

impl HimalayaService {
    pub fn list_accounts(&self) -> MailResult<Vec<Account>> {
        client::list_accounts()
            .map(|accounts| accounts.into_iter().map(Into::into).collect())
            .map_err(|err| map_error(None, Operation::Generic, &err.to_string()))
    }

    pub fn probe_account(&self, account: &str) -> MailResult<()> {
        client::probe_account(account)
            .map_err(|err| map_error(None, Operation::Imap, &err.to_string()))
    }

    pub fn list_folders(&self, account: Option<&str>) -> MailResult<Vec<Folder>> {
        client::list_folders(account)
            .map(|folders| folders.into_iter().map(Into::into).collect())
            .map_err(|err| map_error(None, Operation::Imap, &err.to_string()))
    }

    pub fn list_envelopes(
        &self,
        account: Option<&str>,
        folder: &str,
        page: usize,
        page_size: usize,
        query: Option<&str>,
    ) -> MailResult<Vec<Envelope>> {
        client::list_envelopes(account, folder, page, page_size, query)
            .map(|items| items.into_iter().map(Into::into).collect())
            .map_err(|err| map_error(None, Operation::Imap, &err.to_string()))
    }

    pub fn list_envelopes_threaded(
        &self,
        account: Option<&str>,
        folder: &str,
        query: Option<&str>,
    ) -> MailResult<Vec<Envelope>> {
        client::list_envelopes_threaded(account, folder, query)
            .map(|items| items.into_iter().map(Into::into).collect())
            .map_err(|err| map_error(None, Operation::Imap, &err.to_string()))
    }

    pub fn read_message_content(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<MessageContent> {
        client::read_message(account, folder, id)
            .and_then(|raw| mime::parse_message(raw.as_bytes()).map_err(anyhow::Error::from))
            .map_err(|err| map_error(None, Operation::Imap, &err.to_string()))
    }

    pub fn delete_message(&self, account: Option<&str>, folder: &str, id: &str) -> MailResult<()> {
        client::delete_message(account, folder, id)
            .map_err(|err| map_error(None, Operation::Imap, &err.to_string()))
    }

    pub fn move_message(
        &self,
        account: Option<&str>,
        folder: &str,
        target: &str,
        id: &str,
    ) -> MailResult<()> {
        client::move_message(account, folder, target, id)
            .map_err(|err| map_error(None, Operation::Imap, &err.to_string()))
    }

    pub fn copy_message(
        &self,
        account: Option<&str>,
        folder: &str,
        target: &str,
        id: &str,
    ) -> MailResult<()> {
        client::copy_message(account, folder, target, id)
            .map_err(|err| map_error(None, Operation::Imap, &err.to_string()))
    }

    pub fn flag_add(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
        flag: &str,
    ) -> MailResult<()> {
        client::flag_add(account, folder, id, flag)
            .map_err(|err| map_error(None, Operation::Imap, &err.to_string()))
    }

    pub fn flag_remove(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
        flag: &str,
    ) -> MailResult<()> {
        client::flag_remove(account, folder, id, flag)
            .map_err(|err| map_error(None, Operation::Imap, &err.to_string()))
    }

    pub fn download_attachments(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<String> {
        client::download_attachments(account, folder, id)
            .map_err(|err| map_error(None, Operation::Imap, &err.to_string()))
    }

    pub fn template_write(&self, account: Option<&str>) -> MailResult<String> {
        client::template_write(account)
            .map_err(|err| map_error(None, Operation::Imap, &err.to_string()))
    }

    pub fn template_reply(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
        all: bool,
    ) -> MailResult<String> {
        client::template_reply(account, folder, id, all)
            .map_err(|err| map_error(None, Operation::Imap, &err.to_string()))
    }

    pub fn template_forward(
        &self,
        account: Option<&str>,
        folder: &str,
        id: &str,
    ) -> MailResult<String> {
        client::template_forward(account, folder, id)
            .map_err(|err| map_error(None, Operation::Imap, &err.to_string()))
    }

    pub fn template_send(&self, account: Option<&str>, template: &str) -> MailResult<String> {
        client::template_send(account, template)
            .map_err(|err| map_error(None, Operation::Smtp, &err.to_string()))
    }
}

#[derive(Debug, Clone, Copy)]
enum Operation {
    Generic,
    Imap,
    Smtp,
}

fn map_error(account_backend: Option<&str>, operation: Operation, raw: &str) -> MailError {
    let failure = diagnostics::classify(account_backend, raw);
    match failure.kind {
        diagnostics::FailureKind::BackendMissing => MailError::backend_unavailable(failure.detail),
        diagnostics::FailureKind::ConfigMissing => MailError::config_invalid(failure.detail),
        diagnostics::FailureKind::KeyringUnavailable => {
            MailError::keyring_unavailable(failure.detail)
        }
        diagnostics::FailureKind::GpgFailure => MailError::secret_missing(failure.detail),
        diagnostics::FailureKind::OauthReconfigure => {
            MailError::oauth_reconfigure_required(failure.detail)
        }
        diagnostics::FailureKind::AuthRejected => match operation {
            Operation::Smtp => MailError::smtp_auth_rejected(failure.detail),
            _ => MailError::imap_auth_rejected(failure.detail),
        },
        diagnostics::FailureKind::LocalBackendFailure => {
            MailError::local_maildir_failure(failure.detail)
        }
        diagnostics::FailureKind::Other => MailError::other(failure.detail),
    }
}
