use finance::liability::Zone;
use platform::{batch::Batch, message::Response as MessageResponse};

use crate::contract::Lease;

use super::event;

pub(super) fn build_resp(lease: &Lease, current_liability: Zone, alarms: Batch) -> MessageResponse {
    if let Some(events) = current_liability
        .low()
        .map(|low_level| event::emit_liquidation_warning(&lease.lease, &low_level))
    {
        MessageResponse::messages_with_event(alarms, events)
    } else {
        MessageResponse::messages_only(alarms)
    }
}
