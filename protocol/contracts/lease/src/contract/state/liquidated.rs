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

#[cfg(all(feature = "internal.test.contract", test))]
mod tests {
    use platform::{
        batch::{Batch, Emit, Emitter},
        message::Response as MessageResponse,
    };
    use remote_lease::{
        callback::{RemoteLeaseCallback, RemoteOperationOutcome},
        response::{TransferOutResponse, WireOperationResponse},
    };
    use sdk::cosmwasm_std::{
        Addr, Empty, MessageInfo, QuerierWrapper,
        testing::{self, MockQuerier},
    };

    use crate::{
        contract::state::{Response, State, handler::Handler},
        error::ContractError,
        lease::tests::{REMOTE_LEASE_CONTROLLER, open_lease_dto},
    };

    use super::Liquidated;

    const STRANGER: &str = "stranger";

    #[test]
    fn late_ack_from_the_controller_is_absorbed() {
        let response =
            deliver(info(REMOTE_LEASE_CONTROLLER)).expect("the late ack should be absorbed");

        assert!(matches!(response.next_state, State::Liquidated(_)));
        assert_eq!(absorb_response(), response.response);
    }

    #[test]
    fn late_ack_from_a_stranger_rejected() {
        assert!(matches!(
            deliver(info(STRANGER)),
            Err(ContractError::Unauthorized(_))
        ));
    }

    fn deliver(info: MessageInfo) -> Result<Response, ContractError> {
        let mock_querier = MockQuerier::<Empty>::default();
        liquidated().on_remote_lease_callback(
            late_transfer_out_ack(),
            info,
            QuerierWrapper::new(&mock_querier),
            testing::mock_env(),
        )
    }

    fn liquidated() -> Liquidated {
        Liquidated::from(&open_lease_dto())
    }

    fn late_transfer_out_ack() -> RemoteLeaseCallback {
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationOk(WireOperationResponse::TransferOut(
                TransferOutResponse {},
            )),
        }
    }

    fn info(sender: &str) -> MessageInfo {
        MessageInfo {
            sender: Addr::unchecked(sender),
            funds: vec![],
        }
    }

    fn absorb_response() -> MessageResponse {
        let emitter = Emitter::of_type("ls-remote-lease-late-ack")
            .emit("id", testing::mock_env().contract.address)
            .emit("state", "liquidated");
        MessageResponse::messages_with_event(Batch::default(), emitter)
    }
}

const LATE_ACK_EVENT: &str = "ls-remote-lease-late-ack";
const EVENT_KEY_ID: &str = "id";
const EVENT_KEY_STATE: &str = "state";
const EVENT_VALUE_STATE: &str = "liquidated";

#[derive(Serialize, Deserialize)]
pub struct Liquidated {
    /// The remote-lease controller pinned by the liquidated lease, kept to
    /// authorise late-after-terminal callbacks without a leaser query
    remote_lease_controller: Addr,
}

impl From<&LeaseDTO> for Liquidated {
    fn from(lease: &LeaseDTO) -> Self {
        Self {
            remote_lease_controller: lease.remote_lease_controller.clone(),
        }
    }
}

impl Handler for Liquidated {
    fn state(
        self,
        _now: Instant,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
        Ok(StateResponse::Liquidated())
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

    /// Absorbs late-after-terminal callbacks. The remote-lease IBC channel is
    /// UNORDERED, so a superseded acknowledgment may still land after the
    /// liquidation drove the lease to this terminal. Return `Ok` with an
    /// observability event so the controller's `ibc_packet_ack` commits and the
    /// relayer's retry loop unblocks. Idempotent — no state mutation.
    ///
    /// Authorised against the controller pinned at lease open — the same pin the
    /// drain's in-flight states authorise against.
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
                .emit(EVENT_KEY_STATE, EVENT_VALUE_STATE);
            StateMachineResponse::from(
                MessageResponse::messages_with_event(Batch::default(), emitter),
                self,
            )
        })
    }
}

impl DrainAll for Liquidated {}
