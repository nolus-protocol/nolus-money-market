use std::convert::Infallible;

use cosmwasm_std::{Addr, StdError};
use thiserror::Error;

use finance::currency::{Currency, SymbolOwned};
use marketprice::{alarms::errors::AlarmError, error::PriceFeedsError, feeders::PriceFeedersError};

use crate::state::supported_pairs::ResolutionPath;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("[Oracle] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[Oracle] {0}")]
    PriceFeedersError(#[from] PriceFeedersError),

    #[error("[Oracle] {0}")]
    PriceFeedsError(#[from] PriceFeedsError),

    #[error("[Oracle] {0}")]
    HooksError(#[from] AlarmError),

    #[error("[Oracle] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Oracle] {0}")]
    FromInfallible(#[from] Infallible),

    #[error("[Oracle] Unauthorized")]
    Unauthorized {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("[Oracle] Unsupported denom pairs")]
    UnsupportedDenomPairs {},

    #[error("[Oracle] Invalid feeder address")]
    InvalidAddress {},

    #[error("Invalid denom pair")]
    InvalidDenomPair((SymbolOwned, SymbolOwned)),

    #[error("[Oracle] Invalid denom pair")]
    InvalidResolutionPath(ResolutionPath),

    #[error("[Oracle] No feeder data for the specified address")]
    UnknownFeeder {},

    #[error("[Oracle] There are no authorized feeders")]
    NoAuthorizedFeeders {},

    #[error("[Oracle] Invalid alarm notification address: {0:?}")]
    InvalidAlarmAddress(Addr),

    #[error("[Oracle] ParseError {err:?}")]
    ParseError { err: String },

    #[error("[Oracle] Configuration error: {0}")]
    Configuration(String),

    #[error("[Oracle] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Oracle] Unknown currency")]
    UnknownCurrency {},

    #[error("Mismatch of curencies, expected {expected:?}, found {found:?}")]
    CurrencyMismatch { expected: String, found: String },
}

pub fn currency_mismatch<ExpC>(found: SymbolOwned) -> ContractError
where
    ExpC: Currency,
{
    ContractError::CurrencyMismatch {
        expected: ExpC::SYMBOL.into(),
        found,
    }
}
