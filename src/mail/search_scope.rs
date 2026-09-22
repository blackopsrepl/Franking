/*! How far a search reaches. */

/// The scope a search covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchScope {
    /// The folder being viewed.
    #[default]
    Folder,
    /// Every folder of the current account.
    Folders,
    /// Every folder of every account.
    Accounts,
}

impl SearchScope {
    /// The next scope in the cycle.
    pub fn next(self) -> Self {
        match self {
            SearchScope::Folder => SearchScope::Folders,
            SearchScope::Folders => SearchScope::Accounts,
            SearchScope::Accounts => SearchScope::Folder,
        }
    }

    /// A short label for the status bar.
    pub fn label(self) -> &'static str {
        match self {
            SearchScope::Folder => "this folder",
            SearchScope::Folders => "all folders",
            SearchScope::Accounts => "all accounts",
        }
    }

    /// A stable name for storage.
    pub fn as_str(self) -> &'static str {
        match self {
            SearchScope::Folder => "folder",
            SearchScope::Folders => "folders",
            SearchScope::Accounts => "accounts",
        }
    }

    /// Parse a stored name, defaulting to the narrowest scope.
    pub fn parse(value: &str) -> Self {
        match value {
            "folders" => SearchScope::Folders,
            "accounts" => SearchScope::Accounts,
            _ => SearchScope::Folder,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SearchScope;

    #[test]
    fn cycling_reaches_every_scope_and_returns() {
        let mut scope = SearchScope::default();
        let mut seen = vec![scope];
        for _ in 0..3 {
            scope = scope.next();
            seen.push(scope);
        }
        assert_eq!(scope, SearchScope::default(), "returns to the start");
        assert!(seen.contains(&SearchScope::Folders));
        assert!(seen.contains(&SearchScope::Accounts));
    }

    #[test]
    fn stored_names_round_trip() {
        for scope in [
            SearchScope::Folder,
            SearchScope::Folders,
            SearchScope::Accounts,
        ] {
            assert_eq!(SearchScope::parse(scope.as_str()), scope);
        }
        assert_eq!(SearchScope::parse("nonsense"), SearchScope::Folder);
    }
}
