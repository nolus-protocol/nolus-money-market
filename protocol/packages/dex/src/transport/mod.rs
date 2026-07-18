use finance::duration::Duration;
pub use swap::{Error as SwapError, ExactAmountIn, Result as SwapResult, SwapPathSlice};
pub use transfer::{TransferOut, TransferOutFactory};

mod swap;
mod transfer;

/// IBC timeout — long enough for relayers to process.
pub const IBC_TIMEOUT: Duration = Duration::from_days(1);
