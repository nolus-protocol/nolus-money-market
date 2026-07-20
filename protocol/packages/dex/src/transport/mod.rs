#[cfg(feature = "impl")]
use finance::duration::Duration;

pub use self::remote_lease::{Error as SwapError, Result as SwapResult, Transport};
#[cfg(feature = "impl")]
pub use self::{
    remote_lease::Factory as RemoteLeaseTransportFactory,
    transfer::{TransferOut, TransferOutFactory},
};

mod remote_lease;
#[cfg(feature = "impl")]
mod transfer;

/// IBC timeout — long enough for relayers to process.
#[cfg(feature = "impl")]
pub const IBC_TIMEOUT: Duration = Duration::from_days(1);
