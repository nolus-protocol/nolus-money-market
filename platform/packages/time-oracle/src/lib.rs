use thiserror::Error;

use sdk::cosmwasm_std::StdError;

pub use crate::alarms::Alarms;

mod alarms;

#[cfg(feature = "migrate")]
pub mod migrate_v1;

#[derive(Error, Debug, PartialEq)]
pub enum AlarmError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("[Time Oracle] Alarms delivery queue is empty! Cause: {0}")]
    EmptyAlarmsInDeliveryQueue(String),

    #[error("[Time Oracle] Alarms delivery queue is not empty! Cause: {0}")]
    NonEmptyAlarmsInDeliveryQueue(String),
}
