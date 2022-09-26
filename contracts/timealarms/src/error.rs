use cosmwasm_std::{Addr, StdError};
use thiserror::Error;
use time_oracle::AlarmError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("[TimeAlarms] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[TimeAlarms] Unauthorized")]
    Unauthorized {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("[TimeAlarms] Invalid alarm notification address: {0:?}")]
    InvalidAlarmAddress(Addr),

    #[error("[Platform] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Oracle] {0}")]
    AlarmError(#[from] AlarmError),
}
