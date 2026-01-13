use thiserror::Error;

use sdk::cosmwasm_std::{Addr, StdError};

#[derive(Debug, PartialEq, Error)]
pub enum ContractError {
    #[error("[Profit] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[Profit] {0}")]
    Dex(#[from] dex::Error),

    #[error("[Profit] {0}")]
    PriceOracle(#[from] oracle_platform::error::Error),

    #[error("[Profit] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Profit] {0}")]
    Unauthorized(#[from] access_control::error::Error),

    #[error("[Profit] Failed to update software! Cause: {0}")]
    UpdateSoftware(versioning::Error),

    #[error("[Profit] {0}")]
    TimeAlarm(#[from] timealarms::stub::Error),

    #[error("[Profit] Invalid contract address {0}")]
    InvalidContractAddress(Addr),

    #[error("[Profit] Invalid alarm notification address: {0:?}")]
    InvalidAlarmAddress(Addr),

    #[error("[Profit] Alarm comming from unknown address: {0:?}")]
    UnrecognisedAlarm(Addr),

    #[error("[Profit] Operation is not supported at this time. Cause: {0}")]
    UnsupportedOperation(String),

    #[error(
        "[Profit] Invalid time configuration. Current profit transfer time is before the last transfer time"
    )]
    InvalidTimeConfiguration {},

    #[error("[Profit] EmptyBalance. No profit to dispatch")]
    EmptyBalance {},
}

impl ContractError {
    pub(crate) fn unsupported_operation(msg: &'static str) -> Self {
        Self::UnsupportedOperation(String::from(msg))
    }
}
