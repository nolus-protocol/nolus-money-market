use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum AlarmError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Error on add alarm")]
    AddAlarm {},

    #[error("{0}")]
    Platform(#[from] platform::error::Error),
}
