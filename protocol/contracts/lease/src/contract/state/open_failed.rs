use finance::{duration::Duration, instant::Instant};
use platform::{
    batch::{Batch, Emit, Emitter},
    message::Response as MessageResponse,
    state_machine::Response as StateMachineResponse,
};
use remote_lease::callback::{RemoteErrorMessage, RemoteLeaseCallback};
use sdk::cosmwasm_std::{Env, MessageInfo, QuerierWrapper};
use serde::{Deserialize, Serialize};

use crate::{
    api::query::StateResponse as QueryStateResponse,
    contract::api::Contract,
    error::ContractResult,
};

use super::Response;

const LATE_ACK_EVENT: &str = "ls-remote-lease-late-ack";

#[derive(Serialize, Deserialize)]
pub(crate) struct OpenFailed {
    reason: RemoteErrorMessage,
}

impl OpenFailed {
    pub(crate) fn new(reason: RemoteErrorMessage) -> Self {
        Self { reason }
    }
}

impl Contract for OpenFailed {
    fn state(
        self,
        _now: Instant,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> ContractResult<QueryStateResponse> {
        Ok(QueryStateResponse::OpenFailed {
            reason: self.reason,
        })
    }

    /// Absorbs late-after-terminal callbacks. The remote-lease IBC
    /// channel is UNORDERED, so the original packet's success ack may
    /// still land here after a timeout already moved us to this
    /// terminal. Return `Ok` with an observability event so the
    /// controller's `ibc_packet_ack` commits and the relayer's retry
    /// loop unblocks. Idempotent — no state mutation.
    fn on_remote_lease_callback(
        self,
        _callback: RemoteLeaseCallback,
        _info: MessageInfo,
        _querier: QuerierWrapper<'_>,
        env: Env,
    ) -> ContractResult<Response> {
        let emitter = Emitter::of_type(LATE_ACK_EVENT)
            .emit("id", env.contract.address)
            .emit("terminal", "open_failed");
        Ok(StateMachineResponse::from(
            MessageResponse::messages_with_event(Batch::default(), emitter),
            super::State::from(self),
        ))
    }
}
