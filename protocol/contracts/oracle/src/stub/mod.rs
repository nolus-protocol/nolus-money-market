#[cfg(feature = "stub_alarms")]
pub use alarms::*;
pub use error::{Error, Result};
pub use price::convert;
#[cfg(feature = "stub_swap")]
pub use swap::*;

#[cfg(feature = "stub_alarms")]
mod alarms;
mod error;
mod price;
#[cfg(feature = "stub_swap")]
mod swap;
