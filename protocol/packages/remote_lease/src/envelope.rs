use serde::{Deserialize, Serialize};

use crate::msg::LeaseOperationsMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PacketEnvelope {
    // Wire-format only. The consumer (CosmWasm controller) MUST call
    // `deps.api.addr_validate(&self.lease)` before dispatching, and MUST
    // additionally verify the address belongs to a registered Lease
    // instance (`info.sender.code_id == Config.lease_code` at the
    // ExecuteMsg entry point that produced this packet). Format validation
    // alone is not authorisation.
    pub lease: String,
    pub operation: LeaseOperationsMsg,
}
