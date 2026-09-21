/*! Account auto-discovery module wiring. */

mod autodiscover;
mod discover;
mod model;
mod mozilla;
mod presets;

#[cfg(test)]
mod tests;

pub use discover::discover;
pub use model::{DiscoveredConfig, DiscoverySource};
