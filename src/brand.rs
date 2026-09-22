/*! Product identity in one place.
A rebrand changes these constants; everything else reads them. */

use std::path::PathBuf;

/// Display name, used in the interface and in desktop notifications.
pub const NAME: &str = "Franking";

/// Directory under the user's data directory that holds this app's state.
pub const DATA_DIR: &str = "franking";

/// Namespace of the secret-service ids this app writes.
pub const KEYRING_NAMESPACE: &str = "franking";

/// Value of the `application` attribute on stored secrets.
pub const KEYRING_APPLICATION: &str = "franking";

/// The data directory for this app.
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(DATA_DIR)
}

#[cfg(test)]
mod tests {
    use super::{data_dir, DATA_DIR};

    #[test]
    fn the_data_directory_is_named_after_the_product() {
        assert!(data_dir().ends_with(DATA_DIR));
    }
}
