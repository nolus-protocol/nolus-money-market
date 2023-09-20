use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{
    contract::{cmd::LiquidationDTO, state::Response, Lease},
    error::ContractResult,
};

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
        LiquidationDTO::Partial(spec) => super::start_impl::<_, partial::RepayableImpl>(
            lease,
            spec,
            curr_request_response,
            env,
            querier,
        ),
        LiquidationDTO::Full(spec) => super::start_impl::<_, full::RepayableImpl>(
            lease,
            spec,
            curr_request_response,
            env,
            querier,
        ),
    }
    .map_err(Into::into)
}
