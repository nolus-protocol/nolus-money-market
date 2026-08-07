use std::result::Result as StdResult;

use thiserror::Error;

use oracle_platform::error::Error as PlatformError;

pub use coin_to_out::{CoinToOut, ToQuote};
pub use convert::{from_quote, to_quote};

mod coin_to_out;
mod convert;

pub type Result<T> = StdResult<T, Error>;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Oracle; Price; Stub] Failed to convert from the quote currency! Cause: {0}")]
    FromQuoteConvert(PlatformError),

    #[error("[Oracle; Price; Stub] Failed to convert to the quote currency! Cause: {0}")]
    ToQuoteConvert(PlatformError),
}
