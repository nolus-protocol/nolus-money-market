use cosmwasm_std::{Addr, StdError};
use marketprice::{feeders::PriceFeedersError, market_price::PriceFeedsError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    PriceFeedersError(#[from] PriceFeedersError),

    #[error("{0}")]
    PriceFeedsError(#[from] PriceFeedsError),

    #[error("Unauthorized")]
    Unauthorized {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.

    #[error("Invalid feeder address")]
    InvalidAddress {},

    #[error("No feeder data for the specified address")]
    UnknownFeeder {},

    #[error("There are no authorized feeders")]
    NoAuthorizedFeeders {},

    #[error("Invalid alarm notification address: {0:?}")]
    InvalidAlarmAddess(Addr),

}
