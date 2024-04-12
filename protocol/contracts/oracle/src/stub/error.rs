use std::result::Result as StdResult;

use thiserror::Error;

use currency::SymbolOwned;
use sdk::cosmwasm_std::StdError;

pub type Result<T> = StdResult<T, Error>;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Oracle; Stub] Failed to query configuration! Cause: {0}")]
    StubConfigQuery(StdError),

    #[error("[Oracle] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Oracle] Failed to fetch price for the pair {from}/{to}! Possibly no price is available! Cause: {error}")]
    FailedToFetchPrice {
        from: SymbolOwned,
        to: SymbolOwned,
        error: StdError,
    },

    #[error("[Oracle] {0}")]
    CurrencyMismatch(#[from] currency::error::Error),
}
