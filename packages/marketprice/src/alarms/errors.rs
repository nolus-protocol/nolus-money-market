use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum AlarmError {
    #[error("[Market Price] {0}")]
    Std(#[from] StdError),

    #[error("[Market Price] Error on adding alarm: {0}")]
    AddAlarm(String),

    #[error("[Market Price] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Market Price] {0}")]
    Finance(#[from] finance::error::Error),
}

pub fn add_alarm_error<T>(description: T) -> Result<(), AlarmError>
where
    T: Into<String>,
{
    Err(AlarmError::AddAlarm(description.into()))
}
