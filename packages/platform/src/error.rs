use cosmwasm_std::StdError;
use thiserror::Error;

use finance::currency::Currency;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Platform] Found currency {0} expecting {1}")]
    UnexpectedCurrency(String, String),

    #[error("[Platform] Expecting funds of {0} but found none")]
    NoFunds(String),

    #[error("[Platform] Expecting funds of {0} but found extra ones")]
    UnexpectedFunds(String),

    #[error("[Platform] [Std] {0}")]
    CosmWasmError(#[from] StdError),
}

impl Error {
    pub fn no_funds<C>() -> Self
    where
        C: Currency,
    {
        Self::NoFunds(C::SYMBOL.into())
    }

    pub fn unexpected_funds<C>() -> Self
    where
        C: Currency,
    {
        Self::UnexpectedFunds(C::SYMBOL.into())
    }
}

pub type Result<T> = core::result::Result<T, Error>;
