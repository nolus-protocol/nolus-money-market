use cosmwasm_std::{Timestamp, QuerierWrapper};

use crate::{
    api::{opened::OngoingTrx, StateResponse},
    contract::cmd::LeaseState,
    error::ContractResult,
    lease::{with_lease, LeaseDTO},
};

pub mod active;
pub mod repay;

fn lease_state(
    lease: LeaseDTO,
    in_progress: Option<OngoingTrx>,
    now: Timestamp,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<StateResponse> {
    
    with_lease::execute(
        lease,
        LeaseState::new(now, in_progress),
        querier,
    )
}
