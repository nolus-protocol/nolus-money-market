use cosmwasm_std::{Deps, Env};

use crate::{
    api::{
        opened::{OngoingTrx, RepayTrx},
        PaymentCoin, StateResponse,
    },
    contract::cmd::LeaseState,
    error::ContractResult,
    lease::{with_lease, LeaseDTO},
};

pub mod buy_lpn;
pub mod transfer_in;
pub mod transfer_out;

fn query(
    lease: LeaseDTO,
    payment: PaymentCoin,
    in_progress: RepayTrx,
    deps: &Deps,
    env: &Env,
) -> ContractResult<StateResponse> {
    let in_progress = OngoingTrx::Repayment {
        payment,
        in_progress,
    };

    with_lease::execute(
        lease,
        LeaseState::new(env.block.time, Some(in_progress)),
        &env.contract.address,
        &deps.querier,
    )
}
