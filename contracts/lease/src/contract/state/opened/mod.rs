use cosmwasm_std::{QuerierWrapper, Timestamp};

use crate::{
    api::{opened::OngoingTrx, StateResponse},
    contract::cmd::LeaseState,
    error::ContractResult,
    lease::{with_lease, LeaseDTO},
};

pub mod active;
pub mod liquidation;
pub mod repay;
#[cfg(feature = "migration")]
pub mod v2;

fn lease_state(
    lease: LeaseDTO,
    in_progress: Option<OngoingTrx>,
    now: Timestamp,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<StateResponse> {
    with_lease::execute(lease, LeaseState::new(now, in_progress), querier)
}
