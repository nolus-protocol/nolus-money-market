use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};

use crate::{
    api::{opened::OngoingTrx, StateResponse},
    contract::{cmd::LeaseState, Lease},
    error::ContractResult,
};

pub mod active;
mod balance;
mod event;
pub mod liquidation;
pub mod repay;

fn lease_state(
    lease: Lease,
    in_progress: Option<OngoingTrx>,
    now: Timestamp,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<StateResponse> {
    lease.execute(LeaseState::new(now, in_progress), querier)
}
