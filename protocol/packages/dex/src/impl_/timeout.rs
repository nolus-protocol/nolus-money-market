use platform::{
    batch::{Emit, Emitter},
    message::Response as MessageResponse,
    state_machine::Response as StateMachineResponse,
};
use sdk::cosmwasm_std::{Addr, Env, QuerierWrapper};

use crate::{Enterable, error::Result};
use cw_time::IntoInstant;

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
    on_timeout_retry_extended(current_state, state_label, querier, env, |emitter| emitter)
}

/// Re-emit like [`on_timeout_retry`], with `extend` appending extra
/// attributes to the retry event. The #660 liquidation requote carries the
/// previous and the freshly re-quoted floor there, or marks a skipped
/// requote, on top of the base `timeout = retry`.
pub(crate) fn on_timeout_retry_extended<S, SEnum, L, Extend>(
    current_state: S,
    state_label: L,
    querier: QuerierWrapper<'_>,
    env: Env,
    extend: Extend,
) -> Result<StateMachineResponse<SEnum>>
where
    S: Enterable + Into<SEnum>,
    L: Into<String>,
    Extend: FnOnce(Emitter) -> Emitter,
{
    current_state
        .enter(env.block.time.into_instant(), querier)
        .map(|batch| {
            let emitter = extend(emit_timeout(state_label, env.contract.address));

            StateMachineResponse::from(
                MessageResponse::messages_with_event(batch, emitter),
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
