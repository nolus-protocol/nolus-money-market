use cosmwasm_std::{Addr, StdError};
use marketprice::{
    alarms::errors::AlarmError, feed::DenomPair, feeders::PriceFeedersError,
    market_price::PriceFeedsError,
};
use thiserror::Error;

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

    #[error("Unauthorized")]
    Unauthorized {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("Unsupported denom pairs")]
    UnsupportedDenomPairs {},

    #[error("Invalid feeder address")]
    InvalidAddress {},

    #[error("Invalid denom pair")]
    InvalidDenomPair(DenomPair),

    #[error("No feeder data for the specified address")]
    UnknownFeeder {},

    #[error("There are no authorized feeders")]
    NoAuthorizedFeeders {},

    #[error("Invalid alarm notification address: {0:?}")]
    InvalidAlarmAddress(Addr),

    #[error("ParseError {err:?}")]
    ParseError { err: String },
}
