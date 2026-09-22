// Release configuration for commit-and-tag-version.
//
// The product version is the [package] version in Cargo.toml. Cargo.lock
// records the same number for this crate and CI builds with `--locked`, so the
// lock file is a version surface too and is declared here instead of being left
// to drift. The README version badge follows the repository tag, so it needs no
// entry.
//
// Links point at the GitHub remote, which is where the releases are published;
// the Forgejo remote carries the same commits and the same tag.
module.exports = {
  packageFiles: [
    { filename: "Cargo.toml", updater: "scripts/version/cargo-toml.js" },
  ],
  bumpFiles: [
    { filename: "Cargo.toml", updater: "scripts/version/cargo-toml.js" },
    { filename: "Cargo.lock", updater: "scripts/version/cargo-lock.js" },
  ],
  tagPrefix: "v",
  releaseCommitMessageFormat: "chore(release): {{currentTag}}",
  commitUrlFormat: "https://github.com/blackopsrepl/Franking/commit/{{hash}}",
  compareUrlFormat:
    "https://github.com/blackopsrepl/Franking/compare/{{previousTag}}...{{currentTag}}",
  issueUrlFormat: "https://github.com/blackopsrepl/Franking/issues/{{id}}",
};
