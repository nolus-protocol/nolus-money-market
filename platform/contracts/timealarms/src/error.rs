use std::num::TryFromIntError;

use thiserror::Error;

use sdk::cosmwasm_std::{Addr, Timestamp};
use time_oracle::AlarmError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("[TimeAlarms] [Std] {0}")]
    Std(String),

    #[error("[TimeAlarms] {0}")]
    Versioning(#[from] versioning::Error),

    #[error("[TimeAlarms] Unauthorized")]
    Unauthorized {},

    #[error("[TimeAlarms] Invalid alarm notification address: {0:?}")]
    InvalidAlarmAddress(Addr),

    #[error("[TimeAlarms] Alarm is in the past: {0:?}")]
    InvalidAlarm(Timestamp),

    #[error("[TimeAlarms] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[TimeAlarms] {0}")]
    AlarmError(#[from] AlarmError),

    #[error("[TimeAlarms] integer conversion {0}")]
    Conversion(#[from] TryFromIntError),
}
