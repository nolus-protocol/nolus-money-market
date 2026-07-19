#[cfg(feature = "impl")]
use finance::duration::Duration;

pub use swap::{Error as SwapError, ExactAmountIn, Result as SwapResult, SwapPathSlice};
#[cfg(feature = "impl")]
pub use transfer::{TransferOut, TransferOutFactory};

mod swap;
#[cfg(feature = "impl")]
mod transfer;

/// IBC timeout — long enough for relayers to process.
#[cfg(feature = "impl")]
pub const IBC_TIMEOUT: Duration = Duration::from_days(1);
