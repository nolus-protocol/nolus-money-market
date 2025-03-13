use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{
    api::query::opened::Cause as ApiCause,
    contract::{Lease, cmd::LiquidationDTO, state::Response},
    error::ContractResult,
    position::Cause,
};

use super::ClosePositionTask;

pub mod full;
pub mod partial;

pub(in crate::contract::state) fn start(
    lease: Lease,
    liquidation: LiquidationDTO,
    curr_request_response: MessageResponse,
    env: &Env,
    querier: QuerierWrapper<'_>,
) -> ContractResult<Response> {
    match liquidation {
        LiquidationDTO::Partial(spec) => spec.start(lease, curr_request_response, env, querier),
        LiquidationDTO::Full(spec) => spec.start(lease, curr_request_response, env, querier),
    }
}

impl From<Cause> for ApiCause {
    fn from(value: Cause) -> Self {
        match value {
            Cause::Liability {
                ltv: _,
                healthy_ltv: _,
            } => ApiCause::Liability,
            Cause::Overdue() => ApiCause::Overdue,
        }
    }
}
