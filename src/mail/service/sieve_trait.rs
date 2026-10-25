/*! ManageSieve operations, split from the mail boundary: only some backends
host server-side filters, and every default reports that plainly. */

use crate::mail::errors::{MailError, MailResult};
use crate::mail::sieve::SieveScript;

pub trait SieveService {
    fn sieve_scripts(&self, account: Option<&str>) -> MailResult<Vec<SieveScript>> {
        let _ = account;
        Err(MailError::unsupported_feature(
            "Sieve scripts are not supported by this backend",
        ))
    }

    fn sieve_script(&self, account: Option<&str>, name: &str) -> MailResult<String> {
        let _ = (account, name);
        Err(MailError::unsupported_feature(
            "Sieve scripts are not supported by this backend",
        ))
    }

    fn sieve_save_script(&self, account: Option<&str>, name: &str, body: &str) -> MailResult<()> {
        let _ = (account, name, body);
        Err(MailError::unsupported_feature(
            "Sieve scripts are not supported by this backend",
        ))
    }

    fn sieve_set_active(&self, account: Option<&str>, name: Option<&str>) -> MailResult<()> {
        let _ = (account, name);
        Err(MailError::unsupported_feature(
            "Sieve scripts are not supported by this backend",
        ))
    }

    /// Rename a Sieve script.
    fn sieve_rename_script(&self, account: Option<&str>, from: &str, to: &str) -> MailResult<()> {
        let _ = (account, from, to);
        Err(MailError::unsupported_feature(
            "Sieve scripts are not supported by this backend",
        ))
    }

    /// Delete a Sieve script.
    fn sieve_delete_script(&self, account: Option<&str>, name: &str) -> MailResult<()> {
        let _ = (account, name);
        Err(MailError::unsupported_feature(
            "Sieve scripts are not supported by this backend",
        ))
    }
}
