#[cfg(not(feature = "library"))]
pub mod contract;
mod error;
#[cfg(not(feature = "library"))]
pub mod state;
#[cfg(not(feature = "library"))]
pub mod msg;

pub use crate::error::ContractError;
