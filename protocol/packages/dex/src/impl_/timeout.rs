use platform::{
    batch::{Emit, Emitter},
    message::Response as MessageResponse,
    state_machine::Response as StateMachineResponse,
};
use sdk::cosmwasm_std::{Addr, Env, QuerierWrapper};

use crate::{Enterable, error::Result};

pub(crate) fn on_timeout_retry<S, SEnum, L>(
    current_state: S,
    state_label: L,
    querier: QuerierWrapper<'_>,
    env: Env,
) -> Result<StateMachineResponse<SEnum>>
where
    S: Enterable + Into<SEnum>,
    L: Into<String>,
{
    current_state.enter(env.block.time, querier).map(|batch| {
        let emitter = emit_timeout(state_label, env.contract.address);

        StateMachineResponse::from(
            MessageResponse::messages_with_events(batch, emitter),
            current_state,
        )
    })
}

fn emit_timeout<L>(state_label: L, contract: Addr) -> Emitter
where
    L: Into<String>,
{
    Emitter::of_type(state_label)
        .emit("id", contract)
        .emit("timeout", "retry")
}
