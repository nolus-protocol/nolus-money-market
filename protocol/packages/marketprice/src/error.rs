use std::{convert::Infallible, fmt::Debug};

use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum PriceFeedsError {
    #[error("[Market Price; Feeds] {0}")]
    Std(#[from] StdError),

    #[error("[Market Price; Feeds] No price")]
    NoPrice(),

    #[error("[Market Price; Feeds] {0}")]
    FromInfallible(#[from] Infallible),

    #[error("[Market Price; Feeds] Configuration error: {0}")]
    Configuration(String),

    #[error("[Market Price; Feeds] {0}")]
    Currency(#[from] currency::error::Error),

    #[error("[Market Price; Feeds] Computation overflow during `{details}`")]
    ComputationOverflow { details: String },

    #[error("[Market Price; Feeds] {0}")]
    FeedsRetrieve(StdError),

    #[error("[Market Price; Feeds] {0}")]
    FeedRead(StdError),

    #[error("[Market Price; Feeds] {0}")]
    FeedPush(StdError),

    #[error("[Market Price; Feeds] {0}")]
    FeedRemove(StdError),
}

pub type Result<T> = std::result::Result<T, PriceFeedsError>;

impl PriceFeedsError {
    pub fn overflow_add<L, R>(lhs: L, rhs: R) -> Self
    where
        L: Debug,
        R: Debug,
    {
        Self::ComputationOverflow {
            details: format!("({:?} + {:?})", lhs, rhs),
        }
    }

    pub fn overflow_cross_rate<L, R>(lhs: L, rhs: R) -> Self
    where
        L: Debug,
        R: Debug,
    {
        Self::ComputationOverflow {
            details: format!("({:?}.cross_with({:?}))", lhs, rhs),
        }
    }

    pub fn overflow_lossy_mul<L, R>(lhs: L, rhs: R) -> Self
    where
        L: Debug,
        R: Debug,
    {
        Self::ComputationOverflow {
            details: format!("({:?}.lossy_mul({:?}))", lhs, rhs),
        }
    }
}

pub(crate) fn config_error_if(check: bool, msg: &str) -> Result<()> {
    if check {
        Err(PriceFeedsError::Configuration(msg.into()))
    } else {
        Ok(())
    }
}
