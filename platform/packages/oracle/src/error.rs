use thiserror::Error;

use currency::{Currency, SymbolOwned};
use sdk::cosmwasm_std::StdError;

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

    #[error("Mismatch of curencies, expected {expected:?}, found {found:?}")]
    CurrencyMismatch { expected: String, found: String },
}

pub fn currency_mismatch<ExpC>(found: SymbolOwned) -> Error
where
    ExpC: Currency,
{
    Error::CurrencyMismatch {
        expected: ExpC::TICKER.into(),
        found,
    }
}
