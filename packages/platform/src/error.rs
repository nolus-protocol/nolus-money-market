use thiserror::Error;

use finance::currency::Currency;
use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Platform] Expecting funds of {0} but found none")]
    NoFunds(String),

    #[error("[Platform] Expecting funds but found none")]
    NoFundsAny(),

    #[error("[Platform] Expecting funds of {0} but found extra ones")]
    UnexpectedFunds(String),

    #[error("[Platform] Expecting funds consisting of a single coin but found more coins")]
    UnexpectedFundsAny(),

    #[error("[Platform] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Platform] [Std] {0}")]
    CosmWasmError(#[from] StdError),

    #[error("[Lease] [Deserialization] {0}")]
    Deserialization(#[from] serde_json_wasm::de::Error),
}

impl Error {
    pub fn no_funds<C>() -> Self
    where
        C: Currency,
    {
        Self::NoFunds(C::TICKER.into())
    }

    pub fn unexpected_funds<C>() -> Self
    where
        C: Currency,
    {
        Self::UnexpectedFunds(C::TICKER.into())
    }
}

pub type Result<T> = core::result::Result<T, Error>;
