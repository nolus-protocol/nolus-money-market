use dex::Enterable;
use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{
    contract::{
        cmd::LiquidationDTO,
        state::{
            opened::{close::DexState, payment::Repayable},
            Response, State,
        },
        Lease,
    },
    error::ContractResult,
};

use super::Closable;

pub mod full;
pub mod partial;

pub(in crate::contract::state::opened) fn start(
    lease: Lease,
    liquidation: LiquidationDTO,
    curr_request_response: MessageResponse,
    env: &Env,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<Response> {
    match liquidation {
        LiquidationDTO::Partial(spec) => start_impl::<_, partial::RepayableImpl>(
            lease,
            spec,
            curr_request_response,
            env,
            querier,
        ),
        LiquidationDTO::Full(spec) => {
            start_impl::<_, full::RepayableImpl>(lease, spec, curr_request_response, env, querier)
        }
    }
    .map_err(Into::into)
}

fn start_impl<Spec, RepayableT>(
    lease: Lease,
    spec: Spec,
    curr_request_response: MessageResponse,
    env: &Env,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<Response>
where
    Spec: Into<RepayableT>,
    RepayableT: Closable + Repayable,
    DexState<RepayableT>: Into<State>,
{
    let start_state = super::start(lease, spec.into());
    start_state
        .enter(env.block.time, querier)
        .map(|swap_msg| curr_request_response.merge_with(swap_msg))
        .map(|start| Response::from(start, DexState::<RepayableT>::from(start_state)))
        .map_err(Into::into)
}
