use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{
    contract::{cmd::LiquidationDTO, state::Response, Lease},
    error::ContractResult,
};

use super::ClosePositionTask;

pub mod full;
pub mod partial;

pub(in crate::contract::state::opened) fn start(
    lease: Lease,
    liquidation: LiquidationDTO,
    curr_request_response: MessageResponse,
    env: &Env,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<Response> {
    // TODO abstract LiquidationDTO-to-ClosePositionTask to avoid this match
    match liquidation {
        LiquidationDTO::Partial(spec) => {
            partial::RepayableImpl::from(spec).start(lease, curr_request_response, env, querier)
        }
        LiquidationDTO::Full(spec) => {
            full::RepayableImpl::from(spec).start(lease, curr_request_response, env, querier)
        }
    }
    .map_err(Into::into)
}