pub use self::connection::{ConnectionParams, Ics20Channel};
#[cfg(feature = "impl")]
pub use self::error::Error;
// TODO get rid of the glob use below
#[cfg(feature = "impl")]
pub use self::{connect::Connectable, impl_::*};

#[cfg(feature = "impl")]
mod connect;
mod connection;
#[cfg(feature = "impl")]
mod error;
#[cfg(feature = "impl")]
mod impl_;
pub mod swap;
