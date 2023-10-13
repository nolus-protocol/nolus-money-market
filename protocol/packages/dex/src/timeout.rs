use platform::{
    batch::{Emit, Emitter},
    message::Response as MessageResponse,
    state_machine::Response as StateMachineResponse,
};
use sdk::cosmwasm_std::{Addr, Deps, Env};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    connectable::DexConnectable,
    entry_delay::EntryDelay,
    error::Result,
    ica_connector::{Enterable, IcaConnector},
    ica_recover::InRecovery,
};

pub(crate) fn on_timeout_retry<S, SEnum, L>(
    current_state: S,
    state_label: L,
    deps: Deps<'_>,
    env: Env,
) -> Result<StateMachineResponse<SEnum>>
where
    S: Enterable + Into<SEnum>,
    L: Into<String>,
{
    let emitter = emit_timeout(
        state_label,
        env.contract.address.clone(),
        TimeoutPolicy::Retry,
    );
    let batch = current_state.enter(env.block.time, &deps.querier)?;
    Ok(StateMachineResponse::from(
        MessageResponse::messages_with_events(batch, emitter),
        current_state,
    ))
}

pub(crate) fn on_timeout_repair_channel<S, L, SEnum, SwapResult>(
    current_state: S,
    state_label: L,
    time_alarms: TimeAlarmsRef,
    env: Env,
) -> Result<StateMachineResponse<SEnum>>
where
    S: Enterable + DexConnectable + Into<SEnum>,
    EntryDelay<IcaConnector<InRecovery<S, SEnum>, SwapResult>>: Into<SEnum>,
    SEnum: From<IcaConnector<InRecovery<S, SEnum>, SwapResult>>,
    EntryDelay<S>: Into<SEnum>,
    L: Into<String>,
{
    let emitter = emit_timeout(
        state_label,
        env.contract.address,
        TimeoutPolicy::RepairICS27Channel,
    );
    let pre_recover_ica = EntryDelay::new(
        IcaConnector::new(InRecovery::new(current_state, time_alarms.clone())),
        time_alarms,
    );
    pre_recover_ica
        .enter(env.block.time)
        .map(|batch| MessageResponse::messages_with_events(batch, emitter))
        .map(|resp| StateMachineResponse::from(resp, pre_recover_ica))
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
