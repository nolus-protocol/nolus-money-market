use cosmwasm_std::{Deps, Env};

use crate::{
    api::{
        opened::{OngoingTrx, RepayTrx},
        PaymentCoin, StateResponse,
    },
    error::ContractResult,
    lease::LeaseDTO,
};

pub mod buy_lpn;
pub mod transfer_in_init;
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

    super::query(lease, Some(in_progress), deps, env)
}
