use platform::{
    batch::{Emit, Emitter},
    message::Response as MessageResponse,
    state_machine::Response as StateMachineResponse,
};
use sdk::cosmwasm_std::{Addr, Env, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::error::Result;

use super::{
    connectable::DexConnectable,
    ica_connector::{Enterable, IcaConnector},
    ica_recover::InRecovery,
};

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
        let emitter = emit_timeout(state_label, env.contract.address, TimeoutPolicy::Retry);

        StateMachineResponse::from(
            MessageResponse::messages_with_events(batch, emitter),
            current_state,
        )
    })
}

pub(crate) fn on_timeout_repair_channel<S, L, SEnum, SwapResult>(
    current_state: S,
    state_label: L,
    time_alarms: TimeAlarmsRef,
    env: Env,
) -> Result<StateMachineResponse<SEnum>>
where
    S: Enterable + DexConnectable + Into<SEnum>,
    SEnum: From<IcaConnector<InRecovery<S, SEnum>, SwapResult>>,
    L: Into<String>,
{
    let emitter = emit_timeout(
        state_label,
        env.contract.address,
        TimeoutPolicy::RepairICS27Channel,
    );
    let recover_ica = IcaConnector::new(InRecovery::new(current_state, time_alarms));
    let resp = MessageResponse::messages_with_events(recover_ica.enter(), emitter);
    Ok(StateMachineResponse::from(resp, recover_ica))
}

#[derive(Debug)]
enum TimeoutPolicy {
    Retry,
    RepairICS27Channel,
}

fn emit_timeout<L>(state_label: L, contract: Addr, policy: TimeoutPolicy) -> Emitter
where
    L: Into<String>,
{
    Emitter::of_type(state_label)
        .emit("id", contract)
        .emit("timeout", format!("{:?}", policy))
}
