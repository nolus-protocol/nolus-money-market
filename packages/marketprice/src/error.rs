use std::convert::Infallible;

use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum PriceFeedsError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Given address already registered as a price feeder")]
    FeederAlreadyRegistered {},

    #[error("Given address not registered as a price feeder")]
    FeederNotRegistered {},

    #[error("No price")]
    NoPrice(),

    #[error("Invalid price")]
    InvalidPrice(),

    #[error("Found currency {0} expecting {1}")]
    UnexpectedCurrency(String, String),

    #[error("{0}")]
    FromInfallible(#[from] Infallible),

    #[error("{0}")]
    Finance(#[from] finance::error::Error),

    #[error("Unknown currency")]
    UnknownCurrency {},

    #[error("{0}")]
    FeedSerdeError(String),
}

impl From<rmp_serde::decode::Error> for PriceFeedsError {
    fn from(err: rmp_serde::decode::Error) -> Self {
        Self::FeedSerdeError(format!("Error during deserialization: {}", err))
    }
}

impl From<rmp_serde::encode::Error> for PriceFeedsError {
    fn from(err: rmp_serde::encode::Error) -> Self {
        Self::FeedSerdeError(format!("Error during serialization: {}", err))
    }
}
