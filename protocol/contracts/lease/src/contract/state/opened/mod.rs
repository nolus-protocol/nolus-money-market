use finance::duration::Duration;
use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};

use crate::{
    api::query::{opened::OngoingTrx, StateResponse},
    contract::{cmd::LeaseState, Lease},
    error::ContractResult,
};

pub mod active;
mod alarm;
mod balance;
pub mod close;
mod event;
mod payment;
pub mod repay;

fn lease_state(
    lease: Lease,
    in_progress: Option<OngoingTrx>,
    now: Timestamp,
    due_projection: Duration,
    querier: QuerierWrapper<'_>,
) -> ContractResult<StateResponse> {
    lease
        .lease
        .execute(LeaseState::new(now, due_projection, in_progress), querier)
}
