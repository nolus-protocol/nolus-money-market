use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};

use crate::{
    api::{
        opened::{OngoingTrx, RepayTrx},
        PaymentCoin, StateResponse,
    },
    contract::Lease,
    error::ContractResult,
};

pub mod buy_lpn;

fn query(
    lease: Lease,
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
