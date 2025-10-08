use std::convert::Infallible;

use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum PriceFeedsError {
    #[error("[Market Price; Feeds] {0}")]
    Std(String),

    #[error("[Market Price; Feeds] No price")]
    NoPrice(),

    #[error("[Market Price; Feeds] {0}")]
    FromInfallible(#[from] Infallible),

    #[error("[Market Price; Feeds] Configuration error: {0}")]
    Configuration(String),

    #[error("[Market Price; Feeds] {0}")]
    Currency(#[from] currency::error::Error),

    #[error("[Market Price; Feeds] {0}")]
    Finance(#[from] finance::error::Error),

    // #[error("[Market Price; Feeds] {0}")]
    // FeedsRetrieve(String),
    #[error("[Market Price; Feeds] {0}")]
    FeedRead(String),

    #[error("[Market Price; Feeds] {0}")]
    FeedPush(String),

    #[error("[Market Price; Feeds] {0}")]
    FeedRemove(String),
}

impl PriceFeedsError {
    // pub(crate) fn feeds_retrieve(error: StdError) -> Self {
    //     Self::FeedsRetrieve(error.to_string())
    // }

    pub(crate) fn feed_read(error: StdError) -> Self {
        Self::FeedRead(error.to_string())
    }

    pub(crate) fn feed_push(error: StdError) -> Self {
        Self::FeedPush(error.to_string())
    }

    pub(crate) fn feed_remove(error: StdError) -> Self {
        Self::FeedRemove(error.to_string())
    }
}

impl From<StdError> for PriceFeedsError {
    fn from(value: StdError) -> Self {
        Self::Std(value.to_string())
    }
}

pub type Result<T> = std::result::Result<T, PriceFeedsError>;

pub(crate) fn config_error_if(check: bool, msg: &str) -> Result<()> {
    if check {
        Err(PriceFeedsError::Configuration(msg.into()))
    } else {
        Ok(())
    }
}
