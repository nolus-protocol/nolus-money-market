use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};

use crate::{
    api::{opened::OngoingTrx, StateResponse},
    contract::cmd::LeaseState,
    error::ContractResult,
    lease::{with_lease, LeaseDTO},
};

pub mod active;
mod balance;
mod event;
pub mod liquidation;
pub mod repay;

fn lease_state(
    lease: LeaseDTO,
    in_progress: Option<OngoingTrx>,
    now: Timestamp,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<StateResponse> {
    with_lease::execute(lease, LeaseState::new(now, in_progress), querier)
}
