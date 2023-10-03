use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{
    api::PositionClose,
    contract::{cmd::ValidateClosePosition, state::Response, Lease},
    error::ContractResult,
};

use super::ClosePositionTask;

pub mod full;
pub mod partial;

pub(in crate::contract::state::opened) fn start(
    close: PositionClose,
    lease: Lease,
    env: &Env,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<Response> {
    match close {
        PositionClose::PartialClose(spec) => lease
            .execute(ValidateClosePosition::new(&spec), querier)
            .and_then(|()| spec.start(lease, MessageResponse::default(), env, querier)),
        PositionClose::FullClose(spec) => {
            spec.start(lease, MessageResponse::default(), env, querier)
        }
    }
}
