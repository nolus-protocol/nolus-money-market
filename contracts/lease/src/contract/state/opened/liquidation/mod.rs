use cosmwasm_std::{QuerierWrapper, Timestamp};

use crate::{
    api::{
        opened::{LiquidateTrx, OngoingTrx},
        StateResponse,
    },
    error::ContractResult,
    lease::{LeaseDTO, LiquidationDTO},
};

pub mod sell_asset;

fn query(
    lease: LeaseDTO,
    liquidation: LiquidationDTO,
    in_progress: LiquidateTrx,
    now: Timestamp,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<StateResponse> {
    let in_progress = OngoingTrx::Liquidation {
        liquidation: liquidation.amount(&lease).clone(),
        in_progress,
    };

    super::lease_state(lease, Some(in_progress), now, querier)
}
