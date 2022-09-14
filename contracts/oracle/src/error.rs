use std::convert::Infallible;

use cosmwasm_std::{Addr, StdError};
use finance::currency::SymbolOwned;
use marketprice::{alarms::errors::AlarmError, error::PriceFeedsError, feeders::PriceFeedersError};
use thiserror::Error;

use crate::state::supported_pairs::ResolutionPath;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    PriceFeedersError(#[from] PriceFeedersError),

    #[error("{0}")]
    PriceFeedsError(#[from] PriceFeedsError),

    #[error("{0}")]
    HooksError(#[from] AlarmError),

    #[error("{0}")]
    Finance(#[from] finance::error::Error),

    #[error("{0}")]
    FromInfallible(#[from] Infallible),

    #[error("Unauthorized")]
    Unauthorized {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("Unsupported denom pairs")]
    UnsupportedDenomPairs {},

    #[error("Invalid feeder address")]
    InvalidAddress {},

    #[error("Invalid denom pair")]
    InvalidDenomPair((SymbolOwned, SymbolOwned)),

    #[error("Invalid denom pair")]
    InvalidResolutionPath(ResolutionPath),

    #[error("No feeder data for the specified address")]
    UnknownFeeder {},

    #[error("There are no authorized feeders")]
    NoAuthorizedFeeders {},

    #[error("Invalid alarm notification address: {0:?}")]
    InvalidAlarmAddress(Addr),

    #[error("ParseError {err:?}")]
    ParseError { err: String },

    #[error("{0}")]
    Platform(#[from] platform::error::Error),

    #[error("Unknown currency")]
    UnknownCurrency {},
}
