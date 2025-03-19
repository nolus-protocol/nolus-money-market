#[cfg(feature = "impl")]
pub use self::error::Error;
// TODO get rid of the glob use below
#[cfg(feature = "impl")]
pub use self::{
    connect::{Connectable, ConnectionParams, Ics20Channel},
    impl_::*,
};

#[cfg(feature = "impl")]
mod connect;
#[cfg(feature = "impl")]
mod error;
#[cfg(feature = "impl")]
mod impl_;
pub mod swap;
