// commit-and-tag-version updater: the `franking` package block in Cargo.lock.
//
// Cargo.lock records the crate's own version and CI builds with `--locked`, so a
// Cargo.toml bump that does not reach the lock file makes the release fail to
// build. The name/version pair is unique to this package.
const BLOCK = /(\[\[package\]\]\nname = "franking"\nversion = ")([^"]+)(")/;

module.exports = {
  readVersion(contents) {
    const match = contents.match(BLOCK);
    return match ? match[2] : null;
  },
  writeVersion(contents, version) {
    return contents.replace(BLOCK, (_, before, _old, after) => `${before}${version}${after}`);
  },
};
