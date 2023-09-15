use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};

use crate::{
    api::{
        opened::{LiquidateTrx, OngoingTrx},
        StateResponse,
    },
    contract::{cmd::LiquidationDTO, Lease},
    error::ContractResult,
};

pub mod sell_asset;

fn query(
    lease: Lease,
    liquidation: LiquidationDTO,
    in_progress: LiquidateTrx,
    now: Timestamp,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<StateResponse> {
    let in_progress = OngoingTrx::Liquidation {
        liquidation: liquidation.amount(&lease.lease).clone(),
        in_progress,
    };

    super::lease_state(lease, Some(in_progress), now, querier)
}
