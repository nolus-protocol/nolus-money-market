use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum AlarmError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Error on add alarm")]
    AddAlarm {},

    #[error("{0}")]
    Platform(#[from] platform::error::Error),
}
