/*! Command builders exposed for tests. */

use crate::himalaya::config::{account_args, global_args};

pub fn build_envelope_args(
    account: Option<&str>,
    folder: &str,
    page: usize,
    page_size: usize,
    query: Option<&str>,
) -> Vec<String> {
    let mut args = global_args();
    args.extend(["envelope".to_string(), "list".to_string()]);
    args.extend(account_args(account));
    args.extend([
        "-f".to_string(),
        folder.to_string(),
        "-p".to_string(),
        page.to_string(),
        "-s".to_string(),
        page_size.to_string(),
    ]);
    if let Some(q) = query {
        for word in q.split_whitespace() {
            args.push(word.to_string());
        }
    }
    args
}

/// Build command args for read_message (exposed for testing).
pub fn build_read_args(account: Option<&str>, folder: &str, id: &str) -> Vec<String> {
    let mut args = global_args();
    args.extend(["message".to_string(), "read".to_string()]);
    args.extend(account_args(account));
    args.extend(["-f".to_string(), folder.to_string(), id.to_string()]);
    args
}
