//! Best-effort close of the Solana-side lease account
//!
//! Entered after the customer payout goes out — never before it. The
//! `CloseLease` rides the payout transaction as a reply-on-error
//! sub-message, so a synchronous controller failure (for example, a
//! non-operational channel) is absorbed by [`Handler::reply`] instead of
//! reverting the payout. Asynchronous failures follow the drain leg's
//! pattern: an error acknowledgment is absorbed with an event and NOT
//! auto-retried — recovery is the permissionless `Heal`, which re-emits
//! the close verbatim — while a timeout re-emits directly. A success
//! acknowledgment completes the lifecycle into the [`Closed`] terminal.

use serde::{Deserialize, Serialize};

use access_control::permissions::SingleUserPermission;
use finance::{duration::Duration, instant::Instant};
use platform::{
    batch::{Batch, Emit, Emitter, ReplyId},
    message::Response as MessageResponse,
    state_machine::Response as StateMachineResponse,
};
use remote_lease::{
    callback::RemoteLeaseCallback,
    msg::CloseLeaseParams,
    response::{CloseLeaseResponse, WireOperationResponse},
    stub::{ControllerInnerMessage, Lease as ControllerLease},
};
use sdk::cosmwasm_std::{Addr, Env, MessageInfo, QuerierWrapper, Reply};

use crate::{
    api::query::StateResponse,
    contract::state::{Handler, Response, closed::Closed},
    error::{ContractError, ContractResult},
    event::Type,
    lease::LeaseDTO,
};

/// Routed back only while this state is current, so uniqueness within the
/// state suffices; distinct from the dex delivery id to keep traces
/// unambiguous.
const CLOSE_LEASE_REPLY_ID: ReplyId = 142;

const EVENT_KEY_ID: &str = "id";
const EVENT_KEY_REMOTE_LEASE: &str = "remote-lease";
const EVENT_VALUE_CLOSED: &str = "closed";
const EVENT_KEY_ABSORBED: &str = "absorbed";
const EVENT_KEY_HEAL: &str = "heal";
const EVENT_VALUE_REEMIT: &str = "re-emit";
const EVENT_KEY_TIMEOUT: &str = "timeout";
const EVENT_VALUE_RETRY: &str = "retry";
const ABSORB_REMOTE_ERROR: &str = "remote-error";
const ABSORB_UNEXPECTED_VARIANT: &str = "unexpected-response-variant";
const ABSORB_EMISSION_FAILED: &str = "emission-failed";
const ABSORB_UNEXPECTED_REPLY: &str = "unexpected-reply";

/// Await the `CloseLease` acknowledgment after the customer payout
///
/// Reports [`StateResponse::Closed()`] — the customer-facing close
/// completed with the payout; the pending remote cleanup is
/// protocol-internal and observable through this state's events.
#[derive(Serialize, Deserialize)]
pub(crate) struct ClosingRemoteLease {
    /// The remote-lease controller pinned by the lease, the close's
    /// emission target and the only authorised callback sender
    remote_lease_controller: Addr,
}

impl ClosingRemoteLease {
    /// Emit, or re-emit, the `CloseLease` verbatim
    ///
    /// Always a reply-on-error sub-message: no emission of this
    /// best-effort operation may revert the transaction carrying it —
    /// the payout transaction on entry, the controller's timeout
    /// delivery or an operator heal afterwards.
    pub(super) fn schedule_close(&self) -> ContractResult<Batch> {
        ControllerLease::new(&self.remote_lease_controller)
            .close(
                CloseLeaseParams {},
                CloseLeaseParams::TIMEOUT,
                CLOSE_LEASE_REPLY_ID,
                |params, timeout| ControllerExecuteMsg::CloseLease { params, timeout },
            )
            .map_err(Into::into)
    }

    fn authz_callback(&self, info: &MessageInfo) -> ContractResult<()> {
        access_control::check(
            &SingleUserPermission::new(&self.remote_lease_controller),
            info,
        )
        .map_err(ContractError::from)
    }

    fn into_closed(self, env: &Env) -> Response {
        let emitter = self
            .emitter()
            .emit(EVENT_KEY_ID, env.contract.address.clone())
            .emit(EVENT_KEY_REMOTE_LEASE, EVENT_VALUE_CLOSED);
        StateMachineResponse::from(
            MessageResponse::messages_with_event(Batch::default(), emitter),
            Closed::new(self.remote_lease_controller),
        )
    }

    fn absorb(self, reason: &str) -> Response {
        let emitter = self.emitter().emit(EVENT_KEY_ABSORBED, reason);
        StateMachineResponse::from(
            MessageResponse::messages_with_event(Batch::default(), emitter),
            self,
        )
    }

    fn reemit(self, emitter: Emitter) -> ContractResult<Response> {
        self.schedule_close().map(|reemission| {
            StateMachineResponse::from(
                MessageResponse::messages_with_event(reemission, emitter),
                self,
            )
        })
    }

    fn emitter(&self) -> Emitter {
        Emitter::of_type(Type::ClosingRemoteLease)
    }
}

impl From<&LeaseDTO> for ClosingRemoteLease {
    fn from(lease: &LeaseDTO) -> Self {
        Self {
            remote_lease_controller: lease.remote_lease_controller.clone(),
        }
    }
}

impl Handler for ClosingRemoteLease {
    fn state(
        self,
        _now: Instant,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
        Ok(StateResponse::Closed())
    }

    /// Absorb a synchronous `CloseLease` emission failure
    ///
    /// Total by design: the reply fires inside the transaction this
    /// state's protection exists for, so no input may turn into an `Err`.
    /// The result needs no inspection — the sub-message is scheduled
    /// reply-on-error, so a success reply cannot arrive. A foreign reply
    /// id cannot arrive either while this state holds the only
    /// reply-carrying sub-message, yet it is absorbed under its own
    /// reason rather than mislabelled as an emission failure.
    fn reply(
        self,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        msg: Reply,
    ) -> ContractResult<Response> {
        let reason = if msg.id == CLOSE_LEASE_REPLY_ID {
            ABSORB_EMISSION_FAILED
        } else {
            ABSORB_UNEXPECTED_REPLY
        };
        Ok(self.absorb(reason))
    }

    fn on_time_alarm(
        self,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        super::super::ignore_msg(self)
    }

    fn on_price_alarm(
        self,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        super::super::ignore_msg(self)
    }

    fn heal(
        self,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        let emitter = self.emitter().emit(EVENT_KEY_HEAL, EVENT_VALUE_REEMIT);
        self.reemit(emitter)
    }

    fn on_remote_lease_callback(
        self,
        callback: RemoteLeaseCallback,
        info: MessageInfo,
        _querier: QuerierWrapper<'_>,
        env: Env,
    ) -> ContractResult<Response> {
        self.authz_callback(&info).and_then(|()| match callback {
            RemoteLeaseCallback::OperationOk(WireOperationResponse::CloseLease(
                CloseLeaseResponse {},
            )) => Ok(self.into_closed(&env)),
            RemoteLeaseCallback::OperationOk(_unexpected) => {
                Ok(self.absorb(ABSORB_UNEXPECTED_VARIANT))
            }
            RemoteLeaseCallback::OperationErr(_reason) => Ok(self.absorb(ABSORB_REMOTE_ERROR)),
            RemoteLeaseCallback::OperationTimeout => {
                let emitter = self
                    .emitter()
                    .emit(EVENT_KEY_ID, env.contract.address.clone())
                    .emit(EVENT_KEY_TIMEOUT, EVENT_VALUE_RETRY);
                self.reemit(emitter)
            }
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ControllerExecuteMsg {
    CloseLease {
        params: CloseLeaseParams,
        timeout: Duration,
    },
}

impl ControllerInnerMessage for ControllerExecuteMsg {}

#[cfg(all(feature = "internal.test.contract", test))]
mod tests {
    use cw_time::IntoInstant;
    use finance::duration::Duration;
    use platform::{
        batch::{Batch, Emit, Emitter},
        message::Response as MessageResponse,
    };
    use remote_lease::{
        callback::{RemoteErrorMessage, RemoteLeaseCallback},
        msg::CloseLeaseParams,
        response::{CloseLeaseResponse, TransferOutResponse, WireOperationResponse},
    };
    use sdk::cosmwasm_std::{
        self, Addr, Binary, Empty, MessageInfo, QuerierWrapper, Reply, SubMsgResult,
        testing::{self, MockQuerier},
    };

    use crate::{
        api::query::StateResponse,
        contract::state::{Response, State, handler::Handler},
        error::ContractError,
        event::Type,
    };

    use super::{CLOSE_LEASE_REPLY_ID, ClosingRemoteLease};

    const CONTROLLER: &str = "controller";
    const STRANGER: &str = "stranger";

    #[test]
    fn close_lease_msg_shape_matches_the_controller_wire() {
        let params = CloseLeaseParams {};
        let msg = super::ControllerExecuteMsg::CloseLease {
            params: params.clone(),
            timeout: CloseLeaseParams::TIMEOUT,
        };

        let expected = format!(
            r#"{{"close_lease":{{"params":{},"timeout":{}}}}}"#,
            cosmwasm_std::to_json_string(&params).expect("the params should serialize"),
            cosmwasm_std::to_json_string(&CloseLeaseParams::TIMEOUT)
                .expect("the timeout should serialize"),
        );
        assert_eq!(
            expected,
            cosmwasm_std::to_json_string(&msg).expect("the message should serialize")
        );
    }

    #[test]
    fn schedule_close_targets_the_controller() {
        assert_eq!(1, close_emission().len());
    }

    #[test]
    fn ok_ack_reaches_the_closed_terminal() {
        let response = deliver(ok_close_ack(), controller_info());
        assert!(matches!(response.next_state, State::Closed(_)));
        assert_eq!(
            MessageResponse::messages_with_event(
                Batch::default(),
                emitter()
                    .emit("id", testing::mock_env().contract.address)
                    .emit("remote-lease", "closed"),
            ),
            response.response
        );
    }

    #[test]
    fn error_ack_absorbed_without_reemission() {
        let reason =
            RemoteErrorMessage::new("balance mismatch with solana state").expect("within the cap");
        let response = deliver(RemoteLeaseCallback::OperationErr(reason), controller_info());
        assert_still_closing(&response);
        assert_eq!(absorb_response("remote-error"), response.response);
    }

    #[test]
    fn unexpected_ok_ack_absorbed_without_reemission() {
        let response = deliver(
            RemoteLeaseCallback::OperationOk(WireOperationResponse::TransferOut(
                TransferOutResponse {},
            )),
            controller_info(),
        );
        assert_still_closing(&response);
        assert_eq!(
            absorb_response("unexpected-response-variant"),
            response.response
        );
    }

    #[test]
    fn timeout_reemits_the_close() {
        let response = deliver(RemoteLeaseCallback::OperationTimeout, controller_info());
        assert_still_closing(&response);
        assert_eq!(
            MessageResponse::messages_with_event(
                close_emission(),
                emitter()
                    .emit("id", testing::mock_env().contract.address)
                    .emit("timeout", "retry"),
            ),
            response.response
        );
    }

    #[test]
    fn callback_from_a_stranger_rejected() {
        let mock_querier = MockQuerier::<Empty>::default();
        assert!(matches!(
            closing().on_remote_lease_callback(
                ok_close_ack(),
                info(STRANGER),
                QuerierWrapper::new(&mock_querier),
                testing::mock_env(),
            ),
            Err(ContractError::Unauthorized(_))
        ));
    }

    #[test]
    fn heal_reemits_the_close() {
        let mock_querier = MockQuerier::<Empty>::default();
        let response = closing()
            .heal(
                QuerierWrapper::new(&mock_querier),
                testing::mock_env(),
                info(STRANGER),
            )
            .expect("the heal should re-emit");
        assert_still_closing(&response);
        assert_eq!(
            MessageResponse::messages_with_event(
                close_emission(),
                emitter().emit("heal", "re-emit")
            ),
            response.response
        );
    }

    #[test]
    fn reply_absorbs_an_emission_failure() {
        let mock_querier = MockQuerier::<Empty>::default();
        let response = closing()
            .reply(
                QuerierWrapper::new(&mock_querier),
                testing::mock_env(),
                Reply {
                    id: CLOSE_LEASE_REPLY_ID,
                    payload: Binary::default(),
                    gas_used: 0,
                    result: SubMsgResult::Err(String::from("channel not operational")),
                },
            )
            .expect("the reply absorber should be total");
        assert_still_closing(&response);
        assert_eq!(absorb_response("emission-failed"), response.response);
    }

    #[test]
    fn foreign_reply_absorbed_under_its_own_reason() {
        let mock_querier = MockQuerier::<Empty>::default();
        let response = closing()
            .reply(
                QuerierWrapper::new(&mock_querier),
                testing::mock_env(),
                Reply {
                    id: CLOSE_LEASE_REPLY_ID + 1,
                    payload: Binary::default(),
                    gas_used: 0,
                    result: SubMsgResult::Err(String::from("not ours")),
                },
            )
            .expect("the reply absorber should be total");
        assert_still_closing(&response);
        assert_eq!(absorb_response("unexpected-reply"), response.response);
    }

    #[test]
    fn stale_alarms_ignored() {
        let mock_querier = MockQuerier::<Empty>::default();
        let time_alarm = closing()
            .on_time_alarm(
                QuerierWrapper::new(&mock_querier),
                testing::mock_env(),
                info(STRANGER),
            )
            .expect("a stale time alarm should be ignored");
        assert_still_closing(&time_alarm);
        assert_eq!(MessageResponse::default(), time_alarm.response);

        let price_alarm = closing()
            .on_price_alarm(
                QuerierWrapper::new(&mock_querier),
                testing::mock_env(),
                info(STRANGER),
            )
            .expect("a stale price alarm should be ignored");
        assert_still_closing(&price_alarm);
        assert_eq!(MessageResponse::default(), price_alarm.response);
    }

    #[test]
    fn state_reports_closed() {
        let mock_querier = MockQuerier::<Empty>::default();
        assert!(matches!(
            closing()
                .state(
                    testing::mock_env().block.time.into_instant(),
                    Duration::from_secs(0),
                    QuerierWrapper::new(&mock_querier),
                )
                .expect("the state query should succeed"),
            StateResponse::Closed()
        ));
    }

    #[test]
    fn state_serde_round_trips() {
        let serialized = cosmwasm_std::to_json_vec(&closing()).expect("a serializable state");
        let restored: ClosingRemoteLease =
            cosmwasm_std::from_json(&serialized).expect("the state should round-trip");
        assert_eq!(
            serialized,
            cosmwasm_std::to_json_vec(&restored).expect("a serializable state")
        );
    }

    fn closing() -> ClosingRemoteLease {
        ClosingRemoteLease {
            remote_lease_controller: Addr::unchecked(CONTROLLER),
        }
    }

    fn deliver(callback: RemoteLeaseCallback, info: MessageInfo) -> Response {
        let mock_querier = MockQuerier::<Empty>::default();
        closing()
            .on_remote_lease_callback(
                callback,
                info,
                QuerierWrapper::new(&mock_querier),
                testing::mock_env(),
            )
            .expect("the callback should be processed")
    }

    fn assert_still_closing(response: &Response) {
        assert!(matches!(response.next_state, State::ClosingRemoteLease(_)));
    }

    fn ok_close_ack() -> RemoteLeaseCallback {
        RemoteLeaseCallback::OperationOk(WireOperationResponse::CloseLease(CloseLeaseResponse {}))
    }

    fn controller_info() -> MessageInfo {
        info(CONTROLLER)
    }

    fn info(sender: &str) -> MessageInfo {
        MessageInfo {
            sender: Addr::unchecked(sender),
            funds: vec![],
        }
    }

    fn close_emission() -> Batch {
        closing().schedule_close().expect("a valid close emission")
    }

    fn absorb_response(reason: &str) -> MessageResponse {
        MessageResponse::messages_with_event(Batch::default(), emitter().emit("absorbed", reason))
    }

    fn emitter() -> Emitter {
        Emitter::of_type(Type::ClosingRemoteLease)
    }
}
