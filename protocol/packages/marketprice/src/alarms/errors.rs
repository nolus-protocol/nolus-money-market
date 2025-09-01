use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum AlarmError {
    #[error("[Market Price; Alarm] Failed to load next subscriber! Cause: {0}")]
    IteratorLoadFailed(String),

    #[error("[Market Price; Alarm] Failed to load in delivery queue length! Cause: {0}")]
    InDeliveryIsEmptyFailed(String),

    #[error("[Market Price; Alarm] Failed to remove \"below price\"! Cause: {0}")]
    RemoveBelow(String),

    #[error("[Market Price; Alarm] Failed to store new \"below price\" alarm! Cause: {0}")]
    AddAlarmStoreBelow(String),

    #[error("[Market Price; Alarm] Failed to load \"above or equal price\"! Cause: {0}")]
    AddAlarmLoadAboveOrEqual(String),

    #[error("[Market Price; Alarm] Failed to store new \"above or equal price\"! Cause: {0}")]
    AddAlarmStoreAboveOrEqual(String),

    #[error("[Market Price; Alarm] Failed to remove \"above or equal price\"! Cause: {0}")]
    RemoveAboveOrEqual(String),

    #[error("[Market Price; Alarm; InDelivery] Failed to load \"below price\"! Cause: {0}")]
    InDeliveryLoadBelow(String),

    #[error("[Market Price; Alarm; In Delivery] Failed to remove \"below price\"! Cause: {0}")]
    InDeliveryRemoveBelow(String),

    #[error(
        "[Market Price; Alarm; In Delivery] Failed to load \"above or equal price\"! Cause: {0}"
    )]
    InDeliveryLoadAboveOrEqual(String),

    #[error(
        "[Market Price; Alarm; In Delivery] Failed to remove \"above or equal price\"! Cause: {0}"
    )]
    InDeliveryRemoveAboveOrEqual(String),

    #[error(
        "[Market Price; Alarm; In Delivery] Failed to append alarm in \"in delivery\" queue! Cause: {0}"
    )]
    InDeliveryAppend(String),

    #[error("[Market Price; Alarm] Failed to remove last delivered alarm from queue! Cause: {0}")]
    LastDeliveredRemove(String),

    #[error("[Market Price; Alarm] Failed to remove last failed alarm from queue! Cause: {0}")]
    LastFailedRemove(String),

    #[error("[Market Price; Alarm] Alarms delivery queue is empty! Cause: {0}")]
    EmptyAlarmsInDeliveryQueue(String),

    #[error("[Market Price; Alarm] Alarms delivery queue is not empty! Cause: {0}")]
    NonEmptyAlarmsInDeliveryQueue(String),
}
