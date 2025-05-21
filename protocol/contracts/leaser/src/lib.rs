pub use self::{
    contract::{execute, instantiate, migrate, query, reply, sudo},
    error::ContractError,
};

mod cmd;
mod contract;
mod customer;
pub mod error;
mod finance;
mod lease;
mod leaser;
mod migrate;
pub mod msg;
pub mod result;
mod state;

// for some reason 'allow-unwrap-in-tests' clippy configuration does not recognize 'test' config when combined with other
#[cfg(test)]
#[cfg(feature = "internal.test.testing")]
mod tests;
