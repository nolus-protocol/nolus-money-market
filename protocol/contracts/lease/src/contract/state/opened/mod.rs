use finance::duration::Duration;
use finance::instant::Instant;
use sdk::cosmwasm_std::QuerierWrapper;

use crate::{
    api::query::{StateResponse, opened::Status},
    contract::{Lease, cmd::LeaseState},
    error::ContractResult,
};

pub mod active;
mod alarm;
mod balance;
pub mod close;
mod event;
mod payment;
mod permission;
pub mod repay;

fn lease_state(
    lease: Lease,
    status: Status,
    now: Instant,
    due_projection: Duration,
    querier: QuerierWrapper<'_>,
) -> ContractResult<StateResponse> {
    lease
        .lease
        .execute(LeaseState::new(now, due_projection, status), querier)
}
