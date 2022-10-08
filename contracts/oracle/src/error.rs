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

    #[error("[Oracle] Unauthorized")]
    Unauthorized {},

    #[error("[Oracle] Unsupported denom pairs")]
    UnsupportedDenomPairs {},

    #[error("[Oracle] Invalid feeder address")]
    InvalidAddress {},

    #[error("[Oracle] Invalid denom pair ({0}, {1})")]
    InvalidDenomPair(SymbolOwned, SymbolOwned),

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

    #[error("Mismatch of curencies, expected {expected:?}, found {found:?}")]
    CurrencyMismatch { expected: String, found: String },

    #[error("[Oracle][Base='{base}'] Unsupported currency '{unsupported}'")]
    UnsupportedCurrency {
        base: SymbolOwned,
        unsupported: SymbolOwned,
    },
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

pub fn unsupported_currency<C>(unsupported: &SymbolOwned) -> ContractError
where
    C: Currency,
{
    ContractError::UnsupportedCurrency {
        base: C::SYMBOL.into(),
        unsupported: unsupported.into(),
    }
}
