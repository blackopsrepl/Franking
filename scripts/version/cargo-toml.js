// commit-and-tag-version updater: the `version` line of Cargo.toml's [package]
// table. The match is anchored to the start of a line so it cannot collide with
// dependency versions, which are written inline (`imap-codec = { version = ... }`).
const VERSION = /^version = "([^"]+)"/m;

module.exports = {
  readVersion(contents) {
    const match = contents.match(VERSION);
    return match ? match[1] : null;
  },
  writeVersion(contents, version) {
    return contents.replace(VERSION, () => `version = "${version}"`);
  },
};
