use thiserror::Error;

use sdk::cosmwasm_std::{Addr, StdError};

#[derive(Debug, PartialEq, Error)]
pub enum ContractError {
    #[error("[Profit] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[Profit] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Profit] {0}")]
    Unauthorized(#[from] access_control::Unauthorized),

    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("[Profit] Invalid contract address {0}")]
    InvalidContractAddress(Addr),

    #[error("[Profit] Invalid alarm notification address: {0:?}")]
    InvalidAlarmAddress(Addr),

    #[error("[Profit] Alarm comming from unknown address: {0:?}")]
    UnrecognisedAlarm(Addr),

    #[error(
        "[Profit] Invalid time configuration. Current profit transfer time is before the last transfer time"
    )]
    InvalidTimeConfiguration {},

    #[error("[Profit] EmptyBalance. No profit to dispatch")]
    EmptyBalance {},
}
