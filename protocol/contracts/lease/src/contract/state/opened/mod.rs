use access_control::SingleUserPermission;
use finance::duration::Duration;
use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};

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

pub type ChangeClosePolicyPermission<'a> = SingleUserPermission<'a>;
pub type ClosePositionPermission<'a> = SingleUserPermission<'a>;

fn lease_state(
    lease: Lease,
    status: Status,
    now: Timestamp,
    due_projection: Duration,
    querier: QuerierWrapper<'_>,
) -> ContractResult<StateResponse> {
    lease
        .lease
        .execute(LeaseState::new(now, due_projection, status), querier)
}
