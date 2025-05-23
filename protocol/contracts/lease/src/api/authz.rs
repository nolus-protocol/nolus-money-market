use serde::{Deserialize, Serialize};

use access_control::AccessPermission;
use sdk::cosmwasm_std::Addr;
use timealarms::stub::TimeAlarmsRef;

/// Request for a permission check
///
/// The query API any contract who implements [AccessCheck] should respond to
///
/// The response to any variant is [AccessGranted]
#[derive(Serialize)]
#[cfg_attr(feature = "skel_testing", derive(Debug, Deserialize, PartialEq, Eq))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum AccessCheck {
    /// Check for a permission to user to execute a `heal` on a lease with anomaly
    // a meaningfull name on the wire
    #[serde(rename = "check_anomaly_resolution_permission")]
    AnomalyResolution { by: Addr },
}

/// Response to any [AccessCheck] query
#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "skel_testing", derive(Debug, PartialEq, Eq))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum AccessGranted {
    Yes,
    No,
}

pub struct TimeAlarmDelivery<'a> {
    time_alarms_ref: &'a TimeAlarmsRef,
}

impl<'a> TimeAlarmDelivery<'a> {
    pub fn new(time_alarms_ref: &'a TimeAlarmsRef) -> Self {
        Self { time_alarms_ref }
    }
}

impl AccessPermission for TimeAlarmDelivery<'_> {
    fn is_granted_to(&self, caller: &Addr) -> bool {
        self.time_alarms_ref.owned_by(caller)
    }
}
