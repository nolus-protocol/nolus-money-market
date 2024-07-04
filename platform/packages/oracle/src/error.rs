use std::result::Result as StdResult;

use thiserror::Error;

use currency::{CurrencyDTO, Group};
use sdk::cosmwasm_std::StdError;

pub type Result<T, G> = StdResult<T, Error<G>>;

#[derive(Error, Debug, PartialEq)]
pub enum Error<G>
where
    G: Group,
{
    #[error("[Oracle; Stub] Failed to query configuration! Cause: {0}")]
    StubConfigQuery(StdError),

    #[error("[Oracle] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Oracle] Failed to fetch price for the pair {from}/{to}! Possibly no price is available! Cause: {error}")]
    FailedToFetchPrice {
        from: CurrencyDTO<G>,
        to: String,
        error: StdError,
    },

    #[error("[Oracle; Stub] Mismatch of curencies, expected {expected:?}, found {found:?}")]
    CurrencyMismatch {
        expected: CurrencyDTO<G>,
        found: CurrencyDTO<G>,
    },
}

pub fn currency_mismatch<G>(expected: CurrencyDTO<G>, found: CurrencyDTO<G>) -> Error<G>
where
    G: Group,
{
    Error::CurrencyMismatch { expected, found }
}

pub fn failed_to_fetch_price<G, QuoteG>(
    from: CurrencyDTO<G>,
    to: CurrencyDTO<QuoteG>,
    error: StdError,
) -> Error<G>
where
    G: Group,
    QuoteG: Group,
{
    Error::FailedToFetchPrice {
        from,
        to: to.to_string(),
        error,
    }
}
