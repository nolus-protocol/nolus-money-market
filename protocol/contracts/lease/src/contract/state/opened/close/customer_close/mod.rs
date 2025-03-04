use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{
    api::position::{FullClose, PositionClose},
    contract::{
        Lease,
        cmd::ValidateClosePosition,
        state::{Response, event},
    },
    error::ContractResult,
    position::CloseStrategy,
};

use super::ClosePositionTask;

pub mod full;
pub mod partial;

pub(in super::super) fn start(
    close: PositionClose,
    lease: Lease,
    env: &Env,
    querier: QuerierWrapper<'_>,
) -> ContractResult<Response> {
    match close {
        PositionClose::PartialClose(spec) => lease
            .lease
            .clone()
            .execute(ValidateClosePosition::new(&spec), querier)
            .and_then(|()| spec.start(lease, MessageResponse::default(), env, querier)),
        PositionClose::FullClose(spec) => {
            spec.start(lease, MessageResponse::default(), env, querier)
        }
    }
}

pub(in super::super) fn auto_start(
    strategy: CloseStrategy,
    lease: Lease,
    env: &Env,
    querier: QuerierWrapper<'_>,
) -> ContractResult<Response> {
    let events = event::emit_auto_close(strategy, env, &lease.lease.addr);
    FullClose {}.start(lease, events.into(), env, querier)
}
