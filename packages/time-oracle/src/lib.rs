use thiserror::Error;

use sdk::cosmwasm_std::StdError;

pub use crate::{
    alarms::{AlarmDispatcher, Alarms, Id},
    time_oracle::TimeOracle,
};

mod alarms;
mod time_oracle;

pub use alarms::AlarmsCount;

#[derive(Error, Debug, PartialEq)]
pub enum AlarmError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Error on add alarm")]
    AddAlarm {},

    #[error("{0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Market Price] {0}")]
    Math(#[from] std::num::TryFromIntError),
}
