pub mod callback;
pub mod envelope;
pub mod error;
pub mod msg;
pub mod response;
pub mod version;

#[cfg(feature = "stub")]
pub mod stub;

#[cfg(test)]
mod tests;

pub const VERSION: &str = "nls-remote-lease.v1";
pub const PORT_PREFIX: &str = "nls-remote-lease.";

pub fn port_id_for(dex: &str) -> String {
    format!("{PORT_PREFIX}{dex}")
}
