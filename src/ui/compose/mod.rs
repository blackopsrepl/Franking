/*! Compose view module wiring. */

mod body;
mod overlays;
mod render;

#[cfg(test)]
mod overlay_tests;

pub use render::render;
