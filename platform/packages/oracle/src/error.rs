use std::result::Result as StdResult;

use thiserror::Error;

use currency::{CurrencyDTO, Group, SymbolStatic, Tickers, error::Error as CurrencyError};
use finance::error::Error as FinanceError;
use sdk::cosmwasm_std::StdError;

pub type Result<T> = StdResult<T, Error>;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Oracle; Stub] Failed to query configuration! Cause: {0}")]
    StubConfigQuery(StdError),

    #[error("[Oracle; Stub] Invalid configuration! Cause: {0}")]
    StubConfigInvalid(CurrencyError),

    #[error("[Oracle] {0}")]
    Finance(#[from] FinanceError),

    // TODO replace SymbolStatic and SymbolOwned with CurrencyDTO<G> where approptiate, i.e. the string represent a currency
    #[error(
        "[Oracle] Failed to fetch price for the pair {from}/{to}! Possibly no price is available! Cause: {error}"
    )]
    FailedToFetchPrice {
        from: SymbolStatic,
        to: SymbolStatic,
        error: StdError,
    },
}

// TODO: Replace `FinanceError::Overflow` with a generic template with parameters (next branch)
impl Error {
    pub fn overflow(msg: &'static str) -> Self {
        Error::Finance(FinanceError::Overflow(msg))
    }
}

pub fn failed_to_fetch_price<G, QuoteG>(
    from: &CurrencyDTO<G>,
    to: &CurrencyDTO<QuoteG>,
    error: StdError,
) -> Error
where
    G: Group,
    QuoteG: Group,
{
    Error::FailedToFetchPrice {
        from: from.into_symbol::<Tickers<G>>(),
        to: to.into_symbol::<Tickers<G>>(),
        error,
    }
}
