pub mod config;
pub mod tests;

pub use nostr_sdk::Client as NostrClient;

pub use crate::tests::run;
