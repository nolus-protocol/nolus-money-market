use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum AlarmError {
    #[error("[Market Price; Alarm] {0}")]
    Std(#[from] StdError),

    #[error("[Market Price; Alarm] Alarms delivery queue is empty! Cause: {0}")]
    EmptyAlarmsInDeliveryQueue(String),

    #[error("[Market Price; Alarm] Alarms delivery queue is not empty! Cause: {0}")]
    NonEmptyAlarmsInDeliveryQueue(String),
}
