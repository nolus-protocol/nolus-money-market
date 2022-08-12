use cosmwasm_std::{Addr, StdError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    LppError(#[from] lpp::error::ContractError),

    #[error("{0}")]
    Platform(#[from] platform::error::Error),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },

    #[error("{symbol:?}")]
    UnknownCurrency { symbol: String },

    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("Invalid contract address {0}")]
    InvalidContractAddress(Addr),

    #[error("Invalid alarm notification address: {0:?}")]
    InvalidAlarmAddress(Addr),

    #[error("Alarm comming from unknown address: {0:?}")]
    UnrecognisedAlarm(Addr),

    #[error("Invalid time configuration. Current reward distribution time is before the last distribution time")]
    InvalidTimeConfiguration {},

    #[error("Alarm time validation failed")]
    AlarmTimeValidation {},

    #[error("Zero Reward")]
    ZeroReward {},
}
