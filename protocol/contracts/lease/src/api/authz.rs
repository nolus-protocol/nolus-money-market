use serde::{Deserialize, Serialize};

use currency::{Currency, Group, MemberOf};
use oracle_platform::OracleRef;
use sdk::cosmwasm_std::Addr;

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
