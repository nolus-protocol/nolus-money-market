use serde::{Deserialize, Serialize};

use access_control::AccessPermission;
use currency::{Currency, Group, MemberOf};
use sdk::cosmwasm_std::Addr;
use timealarms::stub::TimeAlarmsRef;
use crate::finance::OracleRef;

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

// TimeAlarmDelivery is a permission check used on on_time_alarm
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

// PriceAlarmDelivery is a permission check used on on_price_alarm
pub struct PriceAlarmDelivery<'a, QuoteC, QuoteG>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    oracle_ref: &'a OracleRef<QuoteC, QuoteG>,
}

impl<'a, QuoteC, QuoteG> PriceAlarmDelivery<'a, QuoteC, QuoteG>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    pub fn new(oracle_ref: &'a OracleRef<QuoteC, QuoteG>) -> Self {
        Self { oracle_ref }
    }
}

impl<QuoteC, QuoteG> AccessPermission for PriceAlarmDelivery<'_, QuoteC, QuoteG>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    fn is_granted_to(&self, caller: &Addr) -> bool {
        self.oracle_ref.owned_by(caller)
    }
}

