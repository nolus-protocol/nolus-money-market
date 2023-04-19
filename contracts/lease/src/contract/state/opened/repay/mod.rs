use cosmwasm_std::{QuerierWrapper, Timestamp};

use crate::{
    api::{
        opened::{OngoingTrx, RepayTrx},
        PaymentCoin, StateResponse,
    },
    error::ContractResult,
    lease::LeaseDTO,
};

pub mod buy_lpn;
#[cfg(feature = "migration")]
pub mod v2;

fn query(
    lease: LeaseDTO,
    payment: PaymentCoin,
    in_progress: RepayTrx,
    now: Timestamp,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<StateResponse> {
    let in_progress = OngoingTrx::Repayment {
        payment,
        in_progress,
    };

    super::lease_state(lease, Some(in_progress), now, querier)
}
