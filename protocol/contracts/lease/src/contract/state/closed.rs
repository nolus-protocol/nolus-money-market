use serde::{Deserialize, Serialize};

use access_control::permissions::SingleUserPermission;
use finance::duration::Duration;
use finance::instant::Instant;
use platform::{
    batch::{Batch, Emit, Emitter},
    message::Response as MessageResponse,
    state_machine::Response as StateMachineResponse,
};
use remote_lease::callback::RemoteLeaseCallback;
use sdk::cosmwasm_std::{Addr, Env, MessageInfo, QuerierWrapper};

use crate::{
    api::query::StateResponse,
    error::{ContractError, ContractResult},
    lease::LeaseDTO,
};

use super::{Handler, Response, drain::DrainAll};

const LATE_ACK_EVENT: &str = "ls-remote-lease-late-ack";
const EVENT_KEY_ID: &str = "id";
const EVENT_KEY_TERMINAL: &str = "terminal";
const EVENT_VALUE_TERMINAL: &str = "closed";

#[derive(Serialize, Deserialize)]
pub struct Closed {
    /// The remote-lease controller pinned by the closed lease, kept to
    /// authorise late-after-terminal callbacks without a leaser query
    remote_lease_controller: Addr,
}

impl From<&LeaseDTO> for Closed {
    fn from(lease: &LeaseDTO) -> Self {
        Self {
            remote_lease_controller: lease.remote_lease_controller.clone(),
        }
    }
}

impl Handler for Closed {
    fn state(
        self,
        _now: Instant,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
        Ok(StateResponse::Closed())
    }

    fn on_time_alarm(
        self,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        super::ignore_msg(self)
    }

    fn on_price_alarm(
        self,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        super::ignore_msg(self)
    }

    fn heal(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.drain(&env.contract.address, info.sender, querier)
    }

    /// Absorbs late-after-terminal callbacks. A heal issued while the
    /// drain's in-flight operation was still resolvable solicits a second
    /// acknowledgment that may land only after the close completed.
    /// Return `Ok` with an observability event so the controller's
    /// `ibc_packet_ack` commits and the relayer's retry loop unblocks.
    /// Idempotent — no state mutation.
    ///
    /// Authorised against the controller pinned at lease open — the same
    /// pin the drain's in-flight states authorise against.
    fn on_remote_lease_callback(
        self,
        _callback: RemoteLeaseCallback,
        info: MessageInfo,
        _querier: QuerierWrapper<'_>,
        env: Env,
    ) -> ContractResult<Response> {
        access_control::check(
            &SingleUserPermission::new(&self.remote_lease_controller),
            &info,
        )
        .map_err(ContractError::from)
        .map(|()| {
            let emitter = Emitter::of_type(LATE_ACK_EVENT)
                .emit(EVENT_KEY_ID, env.contract.address)
                .emit(EVENT_KEY_TERMINAL, EVENT_VALUE_TERMINAL);
            StateMachineResponse::from(
                MessageResponse::messages_with_event(Batch::default(), emitter),
                self,
            )
        })
    }
}

impl DrainAll for Closed {}
