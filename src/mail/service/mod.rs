/*! Mail service module wiring. */

mod cache;
mod read;
mod router;
mod router_ops;
mod router_pages;
mod router_sieve;
mod router_sync;
mod service_options;
mod service_trait;

#[cfg(test)]
mod tests;

pub use router::default_mail_service;
pub use service_options::SendOptions;
pub use service_trait::MailService;
