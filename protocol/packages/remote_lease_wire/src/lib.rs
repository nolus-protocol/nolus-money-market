//! Wire-format types for the Nolus ↔ Solana remote-lease protocol.
//!
//! This crate has no monorepo-internal dependencies and is intended to be
//! consumed from outside the workspace (`git = "…"` or `path = "…"`). The
//! Nolus side reuses the same JSON byte sequences via the typed [`remote_lease`]
//! crate, which has wider deps for compile-time ticker enforcement.

pub mod callback;
pub mod coin;
pub mod envelope;
pub mod error;
pub mod msg;
pub mod response;
pub mod ticker;
pub mod version;

#[cfg(test)]
mod tests;

pub const VERSION: &str = "nls-remote-lease.v1";
pub const PORT_PREFIX: &str = "nls-remote-lease.";

pub fn port_id_for(dex: &str) -> String {
    format!("{PORT_PREFIX}{dex}")
}
