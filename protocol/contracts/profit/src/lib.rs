#[cfg(feature = "stub")]
pub mod stub;

#[cfg(feature = "contract")]
mod access_control;

#[cfg(feature = "contract")]
pub mod contract;

#[cfg(feature = "contract")]
pub mod error;

pub mod msg;

#[cfg(feature = "contract")]
mod profit;

#[cfg(feature = "contract")]
pub mod reserve;

#[cfg(feature = "contract")]
pub mod result;

#[cfg(feature = "contract")]
mod state;

pub type CadenceHours = u16;
