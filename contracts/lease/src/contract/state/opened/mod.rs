use cosmwasm_std::{Deps, Env};

use crate::{
    api::{opened::OngoingTrx, StateResponse},
    contract::cmd::LeaseState,
    error::ContractResult,
    lease::{with_lease, LeaseDTO},
};

pub mod active;
pub mod repay;

fn query(
    lease: LeaseDTO,
    in_progress: Option<OngoingTrx>,
    deps: &Deps,
    env: &Env,
) -> ContractResult<StateResponse> {
    // TODO think on taking benefit from having a LppView trait
    with_lease::execute(
        lease,
        LeaseState::new(env.block.time, in_progress),
        &env.contract.address,
        &deps.querier,
    )
}
