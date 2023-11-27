use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};

use crate::{
    api::{opened::OngoingTrx, StateResponse},
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
    querier: QuerierWrapper<'_>,
) -> ContractResult<StateResponse> {
    lease
        .lease
        .execute(LeaseState::new(now, in_progress), querier)
}
