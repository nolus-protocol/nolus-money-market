#[cfg(feature = "contract")]
mod error;
pub mod msg;
#[cfg(feature = "contract")]
pub mod result;

#[cfg(any(feature = "stub", test))]
pub mod stub;

#[cfg(feature = "contract")]
mod alarms;
#[cfg(feature = "contract")]
pub mod contract;
