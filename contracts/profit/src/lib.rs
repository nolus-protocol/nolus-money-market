pub use crate::error::ContractError;

pub mod error;
pub mod msg;

#[cfg(any(feature = "stub", test))]
pub mod stub;

#[cfg(any(feature = "contract", test))]
mod access_control;
#[cfg(any(feature = "contract", test))]
pub mod contract;
#[cfg(any(feature = "contract", test))]
pub mod profit;
#[cfg(any(feature = "contract", test))]
pub mod state;

#[cfg(test)]
mod tests;
