use currencies::Lpns;

pub use self::{
    contract::{execute, instantiate, migrate, query, reply, sudo},
    error::ContractError,
};

mod cmd;
mod contract;
pub mod error;
mod leaser;
mod migrate;
pub mod msg;
pub mod result;
mod state;

#[cfg(test)]
mod tests;

pub(crate) type LpnCurrencies = Lpns;
