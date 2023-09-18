use dex::{Enterable, StartRemoteLocalState};
use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Env, QuerierWrapper, Timestamp};

use crate::{
    api::{
        opened::{LiquidateTrx, OngoingTrx},
        StateResponse,
    },
    contract::{
        cmd::{Closable, LiquidationDTO},
        state::{
            resp_delivery::{ForwardToDexEntry, ForwardToDexEntryContinue},
            Response,
        },
        Lease,
    },
    error::ContractResult,
};

pub mod full;
pub mod partial;
pub mod sell_asset;

pub(super) type StartState<Liquidation> =
    StartRemoteLocalState<Liquidation, ForwardToDexEntry, ForwardToDexEntryContinue>;
pub(crate) type DexState<Liquidation> =
    dex::StateLocalOut<Liquidation, ForwardToDexEntry, ForwardToDexEntryContinue>;

pub(super) fn start(
    lease: Lease,
    liquidation: LiquidationDTO,
    curr_request_response: MessageResponse,
    env: &Env,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<Response> {
    match liquidation {
        LiquidationDTO::Partial(spec) => {
            let start_state = partial::start(lease, spec);
            start_state
                .enter(env.block.time, querier)
                .map(|swap_msg| curr_request_response.merge_with(swap_msg))
                .map(|start_liq| Response::from(start_liq, partial::DexState::from(start_state)))
        }
        LiquidationDTO::Full(spec) => {
            let start_state = full::start(lease, spec);
            start_state
                .enter(env.block.time, querier)
                .map(|swap_msg| curr_request_response.merge_with(swap_msg))
                .map(|start_liq| Response::from(start_liq, full::DexState::from(start_state)))
        }
    }
    .map_err(Into::into)
}

fn query<RepayableT>(
    lease: Lease,
    repayable: RepayableT,
    in_progress: LiquidateTrx,
    now: Timestamp,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<StateResponse>
where
    RepayableT: Closable,
{
    let in_progress = OngoingTrx::Liquidation {
        liquidation: repayable.amount(&lease).clone(),
        in_progress,
    };

    super::lease_state(lease, Some(in_progress), now, querier)
}
