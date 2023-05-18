use std::{convert::Infallible, num::TryFromIntError};

use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum PriceFeedsError {
    #[error("[Market Price; Feeds] {0}")]
    Std(#[from] StdError),

    #[error("[Market Price; Feeds] Given address already registered as a price feeder")]
    FeederAlreadyRegistered {},

    #[error("[Market Price; Feeds] Given address not registered as a price feeder")]
    FeederNotRegistered {},

    #[error("[Market Price; Feeds] No price")]
    NoPrice(),

    #[error("[Market Price; Feeds] Invalid price")]
    InvalidPrice(),

    #[error("[Market Price; Feeds] Found currency {0} expecting {1}")]
    UnexpectedCurrency(String, String),

    #[error("[Market Price; Feeds] {0}")]
    FromInfallible(#[from] Infallible),

    #[error("[Market Price; Feeds] Configuration error: {0}")]
    Configuration(String),

    #[error("[Market Price; Feeds] {0}")]
    TryFromInt(#[from] TryFromIntError),

    #[error("[Market Price; Feeds] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Market Price; Feeds] Unknown currency")]
    UnknownCurrency {},

    #[error("[Market Price; Feeds] {0}")]
    FeedSerdeError(String),
}

impl From<postcard::Error> for PriceFeedsError {
    fn from(err: postcard::Error) -> Self {
        Self::FeedSerdeError(format!("Error during (de-)serialization: {}", err))
    }
}

pub(crate) fn config_error_if(check: bool, msg: &str) -> Result<(), PriceFeedsError> {
    if check {
        Err(PriceFeedsError::Configuration(msg.into()))
    } else {
        Ok(())
    }
}
