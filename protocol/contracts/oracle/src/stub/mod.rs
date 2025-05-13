#[cfg(feature = "stub_alarms")]
pub use alarms::*;
#[cfg(feature = "stub_price")]
pub use price::*;
#[cfg(feature = "stub_swap")]
pub use swap::*;

#[cfg(feature = "stub_alarms")]
mod alarms;
#[cfg(feature = "stub_price")]
mod price;
#[cfg(feature = "stub_swap")]
mod swap;
