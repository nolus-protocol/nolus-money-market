use std::result::Result as StdResult;

use thiserror::Error;

use currency::{error::Error as CurrencyError, CurrencyDTO, Group};
use sdk::cosmwasm_std::StdError;

pub type Result<T> = StdResult<T, Error>;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Oracle; Stub] Failed to query configuration! Cause: {0}")]
    StubConfigQuery(StdError),

    #[error("[Oracle; Stub] Invalid configuration! Cause: {0}")]
    StubConfigInvalid(CurrencyError),

    #[error("[Oracle] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Oracle] Failed to fetch price for the pair {from}/{to}! Possibly no price is available! Cause: {error}")]
    FailedToFetchPrice {
        from: String,
        to: String,
        error: StdError,
    },
}

pub fn failed_to_fetch_price<G, QuoteG>(
    from: CurrencyDTO<G>,
    to: CurrencyDTO<QuoteG>,
    error: StdError,
) -> Error
where
    G: Group,
    QuoteG: Group,
{
    Error::FailedToFetchPrice {
        from: from.to_string(),
        to: to.to_string(),
        error,
    }
}
