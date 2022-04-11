#[cfg(not(feature = "library"))]
pub mod contract;
mod error;
#[cfg(not(feature = "library"))]
pub mod msg;
#[cfg(not(feature = "library"))]
pub mod state;

pub use crate::error::ContractError;
