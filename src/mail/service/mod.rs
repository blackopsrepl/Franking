/*! Mail service module wiring. */

mod cache;
mod read;
mod router;
mod router_ops;
mod service_trait;

#[cfg(test)]
mod tests;

pub use router::default_mail_service;
pub use service_trait::{MailService, SendOptions};
