use thiserror::Error;

use sdk::cosmwasm_std::StdError;

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

    // #[error("[Market Price; Alarm] Failed to load \"above or equal price\"! Cause: {0}")]
    // AddAlarmLoadAboveOrEqual(String),
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

impl AlarmError {
    pub(crate) fn iterator_load_failed(error: StdError) -> Self {
        Self::IteratorLoadFailed(error.to_string())
    }

    pub(crate) fn in_delivery_is_empty_failed(error: StdError) -> Self {
        Self::InDeliveryIsEmptyFailed(error.to_string())
    }

    pub(crate) fn remove_below(error: StdError) -> Self {
        Self::RemoveBelow(error.to_string())
    }

    pub(crate) fn add_alarm_store_below(error: StdError) -> Self {
        Self::AddAlarmStoreBelow(error.to_string())
    }

    // pub(crate) fn add_alarm_load_above_or_equal(error: StdError) -> Self {
    //     Self::AddAlarmLoadAboveOrEqual(error.to_string())
    // }

    pub(crate) fn add_alarm_store_above_or_equal(error: StdError) -> Self {
        Self::AddAlarmStoreAboveOrEqual(error.to_string())
    }

    pub(crate) fn remove_above_or_equal(error: StdError) -> Self {
        Self::RemoveAboveOrEqual(error.to_string())
    }

    pub(crate) fn in_delivery_load_below(error: StdError) -> Self {
        Self::InDeliveryLoadBelow(error.to_string())
    }

    pub(crate) fn in_delivery_remove_below(error: StdError) -> Self {
        Self::InDeliveryRemoveBelow(error.to_string())
    }

    pub(crate) fn in_delivery_load_above_or_equal(error: StdError) -> Self {
        Self::InDeliveryLoadAboveOrEqual(error.to_string())
    }

    pub(crate) fn in_delivery_remove_above_or_equal(error: StdError) -> Self {
        Self::InDeliveryRemoveAboveOrEqual(error.to_string())
    }

    pub(crate) fn in_delivery_append(error: StdError) -> Self {
        Self::InDeliveryAppend(error.to_string())
    }

    pub(crate) fn last_delivered_remove(error: StdError) -> Self {
        Self::LastDeliveredRemove(error.to_string())
    }

    pub(crate) fn last_failed_remove(error: StdError) -> Self {
        Self::LastFailedRemove(error.to_string())
    }
}
