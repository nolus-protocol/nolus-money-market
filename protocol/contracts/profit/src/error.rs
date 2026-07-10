use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Debug, Error)]
pub enum ContractError {
    #[error("[Profit] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[Profit] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Profit] {0}")]
    Unauthorized(#[from] access_control::error::Error),

    #[error("[Profit] Failed to update software! Cause: {0}")]
    UpdateSoftware(versioning::Error),

    #[error("[Profit] {0}")]
    TimeAlarm(#[from] timealarms::stub::Error),
}
