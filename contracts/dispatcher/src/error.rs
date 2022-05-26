use cosmwasm_std::{Addr, StdError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },

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
}
