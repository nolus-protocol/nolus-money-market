use cosmwasm_std::{QuerierWrapper, Timestamp};

use crate::{
    api::{
        opened::{LiquidateTrx, OngoingTrx},
        LeaseCoin, StateResponse,
    },
    error::ContractResult,
    lease::LeaseDTO,
};

pub mod sell_asset;

fn query(
    lease: LeaseDTO,
    liquidation: LeaseCoin,
    in_progress: LiquidateTrx,
    now: Timestamp,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<StateResponse> {
    let in_progress = OngoingTrx::Liquidation {
        liquidation,
        in_progress,
    };

    super::lease_state(lease, Some(in_progress), now, querier)
}
