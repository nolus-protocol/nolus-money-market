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
    contract::{api::Contract, finalize::LeasesRef},
    error::{ContractError, ContractResult},
};

use super::Response;

const LATE_ACK_EVENT: &str = "ls-remote-lease-late-ack";

#[derive(Serialize, Deserialize)]
pub(crate) struct OpenFailed {
    reason: RemoteErrorMessage,
    leases: LeasesRef,
}

impl OpenFailed {
    pub(crate) fn new(reason: RemoteErrorMessage, leases: LeasesRef) -> Self {
        Self { reason, leases }
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
    ///
    /// Gated by the same `remote_lease_callback_permission` check the
    /// in-flight states use, so a third party cannot spam late-ack
    /// events against a terminal lease.
    fn on_remote_lease_callback(
        self,
        _callback: RemoteLeaseCallback,
        info: MessageInfo,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> ContractResult<Response> {
        access_control::check(
            &self.leases.remote_lease_callback_permission(querier),
            &info,
        )
        .map_err(ContractError::from)?;
        let emitter = Emitter::of_type(LATE_ACK_EVENT)
            .emit("id", env.contract.address)
            .emit("terminal", "open_failed");
        Ok(StateMachineResponse::from(
            MessageResponse::messages_with_event(Batch::default(), emitter),
            super::State::from(self),
        ))
    }
}

#[cfg(all(feature = "internal.test.contract", test))]
mod tests {
    use platform::{
        batch::{Batch, Emit, Emitter},
        message::Response as MessageResponse,
    };
    use remote_lease::{
        callback::{RemoteErrorMessage, RemoteLeaseCallback, RemoteOperationOutcome},
        response::{OpenLeaseResponse, RemoteLeaseId, WireOperationResponse},
    };
    use sdk::cosmwasm_std::{
        self, Addr, ContractResult as CwContractResult, Empty, MessageInfo, QuerierWrapper,
        SystemResult, WasmQuery,
        testing::{self, MockQuerier},
    };

    use crate::{
        api::authz::{AccessCheck, AccessGranted},
        contract::{api::Contract, finalize::LeasesRef, state::State},
        error::ContractError,
    };

    use super::OpenFailed;

    const LEASER: &str = "leaser";
    const CONTROLLER: &str = "controller";
    const STRANGER: &str = "stranger";
    const REMOTE_LEASE_ID: &str = "StubPda1111111111111111111111111111";

    #[test]
    fn late_ack_from_authorized_sender_is_absorbed() {
        let response = deliver(AccessGranted::Yes, CONTROLLER).expect("the late ack is absorbed");

        assert!(matches!(response.next_state, State::OpenFailed(_)));
        assert_eq!(absorb_response(), response.response);
    }

    #[test]
    fn late_ack_from_unauthorized_sender_rejected() {
        assert!(matches!(
            deliver(AccessGranted::No, STRANGER),
            Err(ContractError::Unauthorized(_))
        ));
    }

    fn deliver(granted: AccessGranted, sender: &str) -> Result<super::Response, ContractError> {
        let mock_querier = access_querier(granted);
        open_failed().on_remote_lease_callback(
            late_open_ack(),
            info(sender),
            QuerierWrapper::new(&mock_querier),
            testing::mock_env(),
        )
    }

    fn open_failed() -> OpenFailed {
        OpenFailed::new(
            RemoteErrorMessage::from_static("a prior open failure"),
            LeasesRef::unchecked(Addr::unchecked(LEASER)),
        )
    }

    fn late_open_ack() -> RemoteLeaseCallback {
        RemoteLeaseCallback {
            nonce: 0,
            outcome: RemoteOperationOutcome::OperationOk(WireOperationResponse::OpenLease(
                OpenLeaseResponse {
                    remote_lease_id: RemoteLeaseId::new(REMOTE_LEASE_ID.to_owned())
                        .expect("a base58 sample"),
                },
            )),
        }
    }

    fn access_querier(granted: AccessGranted) -> MockQuerier<Empty> {
        let mut mock_querier = MockQuerier::<Empty>::default();
        mock_querier.update_wasm(move |query| {
            let WasmQuery::Smart { msg, .. } = query else {
                unimplemented!("only smart queries are expected")
            };
            let _: AccessCheck =
                cosmwasm_std::from_json(msg).expect("a remote-lease callback permission query");
            SystemResult::Ok(CwContractResult::Ok(
                cosmwasm_std::to_json_binary(&granted).expect("the verdict should serialize"),
            ))
        });
        mock_querier
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
            .emit("terminal", "open_failed");
        MessageResponse::messages_with_event(Batch::default(), emitter)
    }
}
