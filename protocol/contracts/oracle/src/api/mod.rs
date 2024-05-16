#[cfg(feature = "stub_alarms")]
pub use currencies::{Lpn as BaseCurrency, Lpn as StableCurrency, Lpns as BaseCurrencies};
//TODO switch the definition of StableCurrency to the one provided from currencies

#[cfg(feature = "contract")]
pub use contract::*;

#[cfg(feature = "stub_alarms")]
pub mod alarms;
#[cfg(feature = "contract")]
mod contract;
pub mod price;
#[cfg(feature = "stub_swap")]
pub mod swap;
