use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::Addr;

/// The query API any contract who implements [AccessCheck] should respond to
///
/// The response to any variant is [AccessGranted]
#[derive(Serialize)]
#[cfg_attr(feature = "skel_testing", derive(Debug, PartialEq, Eq))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum AccessCheck {
    /// a `heal` on a lease at certain states is tried by the given user
    #[serde(rename = "check_anomaly_resolution_permission")]
    // provide more meaningfull name on the wire
    AnomalyResolution { by: Addr },
}

/// Response to any [AccessCheck] query
#[derive(Deserialize)]
#[cfg_attr(feature = "skel_testing", derive(Debug, PartialEq, Eq))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum AccessGranted {
    Yes,
    No,
}
