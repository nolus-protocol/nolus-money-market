use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum AlarmError {
    #[error("[Market Price; Alarm] Failed to load next subscriber! Context: {0}")]
    IteratorLoadFailed(StdError),

    #[error("[Market Price; Alarm] Failed to load in delivery queue length! Context: {0}")]
    InDeliveryIsEmptyFailed(StdError),

    #[error("[Market Price; Alarm] Failed to remove \"below price\"! Context: {0}")]
    RemoveBelow(StdError),

    #[error("[Market Price; Alarm] Failed to remove \"above or equal price\"! Context: {0}")]
    RemoveAboveOrEqual(StdError),

    #[error("[Market Price; Alarm] Failed to load \"below price\"! Context: {0}")]
    InDeliveryLoadBelow(StdError),

    #[error("[Market Price; Alarm] Failed to remove \"below price\"! Context: {0}")]
    InDeliveryRemoveBelow(StdError),

    #[error("[Market Price; Alarm] Failed to load \"above or equal price\"! Context: {0}")]
    InDeliveryLoadAboveOrEqual(StdError),

    #[error("[Market Price; Alarm] Failed to remove \"above or equal price\"! Context: {0}")]
    InDeliveryRemoveAboveOrEqual(StdError),

    #[error("[Market Price; Alarm] Failed to append alarm in \"in delivery\" queue! Context: {0}")]
    InDeliveryAppend(StdError),

    #[error(
        "[Market Price; Alarm] Failed to remove last delivered alarm from queue! Context: {0}"
    )]
    LastDeliveredRemove(StdError),

    #[error("[Market Price; Alarm] Failed to remove last failed alarm from queue! Context: {0}")]
    LastFailedRemove(StdError),

    #[error("[Market Price; Alarm] Failed to remove last failed alarm from queue! Context: {0}")]
    AddAlarmInternal(StdError),

    #[error("[Market Price; Alarm] Alarms delivery queue is empty! Cause: {0}")]
    EmptyAlarmsInDeliveryQueue(String),

    #[error("[Market Price; Alarm] Alarms delivery queue is not empty! Cause: {0}")]
    NonEmptyAlarmsInDeliveryQueue(String),
}
