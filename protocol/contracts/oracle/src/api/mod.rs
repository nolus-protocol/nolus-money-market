#[cfg(feature = "stub_alarms")]
use currencies::Lpns;

#[cfg(feature = "contract")]
pub use contract::*;

#[cfg(feature = "stub_alarms")]
pub mod alarms;
#[cfg(feature = "contract")]
mod contract;
#[cfg(feature = "stub_swap")]
pub mod swap;

#[cfg(feature = "stub_alarms")]
pub type BaseCurrencyGroup = Lpns;
