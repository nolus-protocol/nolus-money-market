//! # Acknowledgment-to-transfer correlation trust model
//!
//! `OperationResponse::TransferOut` carries no payload at all, so
//! acknowledgments correlate to transfers purely positionally: each one is
//! credited to the single in-flight transfer the `acks_left` countdown
//! tracks. This is a strictly weaker correlation than the swap leg's -
//! there is not even a `min_out`-style cross-check on the credited value.
//! The wire contract is frozen; a per-operation nonce is a cross-repo
//! follow-up. The positional assumption rests on the same pillars as
//! [`RemoteSwap`][super::remote_swap::RemoteSwap]: authorization of the
//! callback sender, the controller's exactly-one-packet delivery, and the
//! sequential one-in-flight emission.
//!
//! # Acknowledgment does not mean arrival
//!
//! The acknowledgment travels back on the lease channel while the
//! transferred funds travel on the paired ICS-20 transfer channel, and
//! IBC orders nothing across channels. An acknowledged transfer therefore
//! attests only that the remote side initiated it. The workflow completes
//! through [`FundsArrival`][super::funds_arrival::FundsArrival], which
//! polls the local account until every transferred coin has landed.
//!
//! # Error acknowledgments are absorbed, not retried
//!
//! Unlike the swap leg, an error acknowledgment does not collapse into the
//! timeout-retry path. A transfer error is plausibly persistent (remote
//! balance short, paired channel closed), and an error-triggered
//! re-emission has no packet-lifetime cadence - retrying it immediately
//! ping-pongs error acknowledgments at relayer speed. The error is
//! absorbed with a distinct event and the workflow waits for an operator
//! [`Handler::heal`]; a bounded recovery policy is a follow-up design.

use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
};

use serde::{Deserialize, Serialize};

use currency::Group;
use finance::{coin::CoinDTO, duration::Duration, instant::Instant};
use platform::{
    batch::{Batch, Emit, Emitter},
    ica::ErrorResponse as ICAErrorResponse,
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Addr, Binary, Env, MessageInfo, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    CoinsNb, Contract, Enterable, TimeAlarm,
    error::{Error, Result},
    impl_::{
        funds_arrival::FundsArrival,
        response::{self, ContinueResult, Handler, Result as HandlerResult},
        timeout,
    },
};

const EVENT_KEY_ABSORBED: &str = "absorbed";
const EVENT_KEY_ACKS_LEFT: &str = "acks-left";
const EVENT_KEY_HEAL: &str = "heal";
const EVENT_VALUE_REEMIT: &str = "re-emit";
const ABSORB_UNDECODABLE: &str = "undecodable-response";
const ABSORB_UNEXPECTED_VARIANT: &str = "unexpected-response-variant";
const ABSORB_REMOTE_ERROR: &str = "remote-error";

/// Specification of a remote-account drain
///
/// A standalone task contract rather than a [`SwapTask`][crate::SwapTask]
/// extension - a transfer has no oracle, no slippage and no output
/// currency, so extending the swap contract would force `unimplemented!`
/// stubs on every implementor.
pub trait RemoteTransferOutTask
where
    Self: Sized,
{
    type G: Group;
    type Label: Into<String>;
    type StateResponse;
    type Result;

    fn label(&self) -> Self::Label;

    fn time_alarm(&self) -> &TimeAlarmsRef;

    /// Authorise an inbound `RemoteLeaseCallback` against this task's
    /// owning contract.
    fn authz_remote_callback(&self, querier: QuerierWrapper<'_>, info: &MessageInfo) -> Result<()>;

    /// Provide the coins, at least one, this drain transfers out.
    /// The iteration is done always in the same order.
    fn coins(&self) -> impl IntoIterator<Item = CoinDTO<Self::G>>;

    /// Schedule a transfer of `coin` out of the remote account
    ///
    /// The transport guarantees a single response, error, or timeout
    /// per scheduled transfer.
    fn schedule_transfer_out(&self, coin: &CoinDTO<Self::G>) -> Result<Batch>;

    /// Validate a transfer response payload
    ///
    /// The payload carries no data; decoding only proves the response is
    /// the scheduled transfer's and not another operation's.
    fn decode_response(&self, payload: &[u8]) -> Result<()>;

    /// Have all the transferred coins arrived on the local `account`
    fn all_received(&self, account: &Addr, querier: QuerierWrapper<'_>) -> Result<bool>;

    /// The final transition of this drain workflow
    ///
    /// Invoked once every transfer is acknowledged and every coin has
    /// arrived on the local account.
    fn finish(self, env: &Env, querier: QuerierWrapper<'_>) -> Self::Result;

    fn state(
        self,
        in_progress: DrainStage,
        now: Instant,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse;
}

/// Progress of a remote-account drain workflow
pub enum DrainStage {
    /// Transfers still awaiting an acknowledgment
    TransferOut { acks_left: CoinsNb },
    /// Every transfer acknowledged, the coins not yet on the local account
    FundsArrival,
}

/// Transfer a list of coins out of a remote account, one in-flight at a time
///
/// The transfers are scheduled strictly sequentially - the next one goes
/// out only once the in-flight one gets acknowledged. The in-flight
/// transfer is identified by `acks_left` against the deterministic
/// [`RemoteTransferOutTask::coins`] order, so no coin list is persisted.
/// After the last acknowledgment the workflow proceeds to
/// [`FundsArrival`].
#[derive(Serialize, Deserialize)]
#[serde(
    bound(
        serialize = "Task: Serialize",
        deserialize = "Task: Deserialize<'de> + RemoteTransferOutTask"
    ),
    deny_unknown_fields,
    rename_all = "snake_case"
)]
pub struct RemoteTransferOut<Task, SEnum>
where
    Task: RemoteTransferOutTask,
{
    spec: Task,
    acks_left: CoinsNb,
    #[serde(skip)]
    _state_enum: PhantomData<SEnum>,
}

impl<Task, SEnum> RemoteTransferOut<Task, SEnum>
where
    Task: RemoteTransferOutTask,
{
    /// Entry point of the drain transfer sequence
    pub fn start(spec: Task) -> Result<Self> {
        let transfers_nb = spec.coins().into_iter().count();
        CoinsNb::try_from(transfers_nb)
            .map_err(|_too_many| Error::TransferOutLegsNbOverflow(CoinsNb::MAX))
            .and_then(|acks_left| {
                if acks_left == 0 {
                    Err(Error::MissingTransferOutLeg)
                } else {
                    Ok(Self::internal_new(spec, acks_left))
                }
            })
    }

    fn internal_new(spec: Task, acks_left: CoinsNb) -> Self {
        let ret = Self {
            spec,
            acks_left,
            _state_enum: PhantomData,
        };
        debug_assert!(ret.invariant_held());
        ret
    }

    fn invariant_held(&self) -> bool {
        0 < self.acks_left && usize::from(self.acks_left) <= self.transfers_nb()
    }

    fn transfers_nb(&self) -> usize {
        self.spec.coins().into_iter().count()
    }

    fn in_flight_transfer(&self) -> Result<CoinDTO<Task::G>> {
        debug_assert!(self.invariant_held());

        self.transfers_nb()
            .checked_sub(self.acks_left.into())
            .and_then(|transfer_index| self.spec.coins().into_iter().nth(transfer_index))
            .ok_or(Error::MissingTransferOutLeg)
    }

    /// Emit, or re-emit, the in-flight transfer
    ///
    /// Re-emissions repeat the original emission verbatim, keeping the
    /// recovery paths idempotent.
    fn schedule(&self) -> Result<Batch> {
        self.in_flight_transfer()
            .and_then(|coin| self.spec.schedule_transfer_out(&coin))
    }

    fn emit_acks_left(&self) -> Emitter {
        Emitter::of_type(self.spec.label())
            .emit_to_string_value(EVENT_KEY_ACKS_LEFT, self.acks_left)
    }

    fn emit_absorbed(&self, reason: &str) -> Emitter {
        Emitter::of_type(self.spec.label()).emit(EVENT_KEY_ABSORBED, reason)
    }

    fn emit_heal(&self) -> Emitter {
        Emitter::of_type(self.spec.label()).emit(EVENT_KEY_HEAL, EVENT_VALUE_REEMIT)
    }
}

impl<Task, SEnum> RemoteTransferOut<Task, SEnum>
where
    Task: RemoteTransferOutTask,
    Self: Handler<Response = SEnum, SwapResult = Task::Result> + Into<SEnum>,
    FundsArrival<Task, SEnum>: Handler<Response = SEnum, SwapResult = Task::Result> + Into<SEnum>,
{
    fn deliver_ack(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        debug_assert!(self.invariant_held());

        match self.acks_left.checked_sub(1) {
            None => Error::MissingTransferOutLeg.into(),
            Some(0) => FundsArrival::new(self.spec)
                .try_complete(querier, env)
                .map_into(),
            Some(acks_left) => Self::internal_new(self.spec, acks_left)
                .schedule_and_continue()
                .into(),
        }
    }

    fn schedule_and_continue(self) -> ContinueResult<Self> {
        self.schedule().and_then(|batch| {
            response::res_continue::<_, _, Self>(
                MessageResponse::messages_with_event(batch, self.emit_acks_left()),
                self,
            )
        })
    }

    fn absorb(self, reason: &str) -> ContinueResult<Self> {
        response::res_continue::<_, _, Self>(
            MessageResponse::messages_with_event(Batch::default(), self.emit_absorbed(reason)),
            self,
        )
    }
}

impl<Task, SEnum> Enterable for RemoteTransferOut<Task, SEnum>
where
    Task: RemoteTransferOutTask,
{
    fn enter(&self, _now: Instant, _querier: QuerierWrapper<'_>) -> Result<Batch> {
        self.schedule()
    }
}

impl<Task, SEnum> Handler for RemoteTransferOut<Task, SEnum>
where
    Task: RemoteTransferOutTask,
    Self: Into<SEnum>,
    FundsArrival<Task, SEnum>: Handler<Response = SEnum, SwapResult = Task::Result> + Into<SEnum>,
{
    type Response = SEnum;
    type SwapResult = Task::Result;

    fn authz_remote_callback(&self, querier: QuerierWrapper<'_>, info: &MessageInfo) -> Result<()> {
        self.spec.authz_remote_callback(querier, info)
    }

    /// Undecodable payloads and decodable-but-non-transfer responses are
    /// absorbed with distinct event reasons instead of erroring - an error
    /// would revert the controller's acknowledgment transaction and strand
    /// the workflow. A successfully validated acknowledgment advances the
    /// countdown and lets any downstream failure propagate.
    fn on_remote_response(
        self,
        data: Binary,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        match self.spec.decode_response(data.as_slice()) {
            Ok(()) => self.deliver_ack(querier, env),
            Err(Error::UnexpectedResponseVariant(_details)) => {
                self.absorb(ABSORB_UNEXPECTED_VARIANT).into()
            }
            Err(_undecodable) => self.absorb(ABSORB_UNDECODABLE).into(),
        }
    }

    /// See the module doc for why errors are absorbed rather than retried
    fn on_remote_error(
        self,
        _response: ICAErrorResponse,
        _querier: QuerierWrapper<'_>,
        _env: Env,
    ) -> HandlerResult<Self> {
        self.absorb(ABSORB_REMOTE_ERROR).into()
    }

    fn on_remote_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        let state_label = self.spec.label();
        timeout::on_timeout_retry(self, state_label, querier, env).into()
    }

    /// Re-emit the in-flight transfer verbatim
    ///
    /// The operator recovery for both an unresolvable packet and an
    /// absorbed error acknowledgment. See the module doc of
    /// [`RemoteSwap`][super::remote_swap::RemoteSwap] for the
    /// duplicate-acknowledgment risk a heal issued while the original
    /// operation is still resolvable creates - with no payload to
    /// cross-check, this transport is credulous to it by construction.
    fn heal(
        self,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _info: &MessageInfo,
    ) -> HandlerResult<Self> {
        self.schedule()
            .and_then(|batch| {
                response::res_continue::<_, _, Self>(
                    MessageResponse::messages_with_event(batch, self.emit_heal()),
                    self,
                )
            })
            .into()
    }
}

impl<Task, SEnum> Contract for RemoteTransferOut<Task, SEnum>
where
    Task: RemoteTransferOutTask,
{
    type StateResponse = Task::StateResponse;

    fn state(
        self,
        now: Instant,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        let acks_left = self.acks_left;
        self.spec.state(
            DrainStage::TransferOut { acks_left },
            now,
            due_projection,
            querier,
        )
    }
}

impl<Task, SEnum> Display for RemoteTransferOut<Task, SEnum>
where
    Task: RemoteTransferOutTask,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str("RemoteTransferOut at ")
            .and_then(|()| f.write_str(&self.spec.label().into()))
    }
}

impl<Task, SEnum> TimeAlarm for RemoteTransferOut<Task, SEnum>
where
    Task: RemoteTransferOutTask,
{
    fn setup_alarm(&self, r#for: Instant) -> Result<Batch> {
        self.spec
            .time_alarm()
            .setup_alarm(r#for)
            .map_err(Into::into)
    }
}

#[cfg(test)]
pub(super) mod mock {
    use serde::{Deserialize, Serialize};

    use currency::test::SuperGroup;
    use finance::{coin::CoinDTO, duration::Duration, instant::Instant};
    use platform::batch::Batch;
    use sdk::cosmwasm_std::{Addr, Env, MessageInfo, QuerierWrapper};
    use timealarms::stub::TimeAlarmsRef;

    use crate::{
        CoinsNb,
        error::{Error, Result},
    };

    use super::{DrainStage, RemoteTransferOutTask};

    pub const LABEL: &str = "RemoteTransferOutMock";
    pub const CONTROLLER: &str = "controller";
    pub const TIME_ALARMS: &str = "time_alarms";
    pub const OK_PAYLOAD: &[u8] = b"\"transfer-out-ok\"";
    pub const WRONG_VARIANT_PAYLOAD: &[u8] = b"wrong-variant";
    pub const FINISH_RESULT: &str = "finished";

    #[derive(Serialize, Deserialize, Clone, Debug)]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub struct MockSpec {
        coins: Vec<CoinDTO<SuperGroup>>,
        received: bool,
        time_alarms: TimeAlarmsRef,
    }

    #[derive(Serialize)]
    struct TransferOutRequest {
        coin: CoinDTO<SuperGroup>,
    }

    impl MockSpec {
        pub fn new(coins: Vec<CoinDTO<SuperGroup>>) -> Self {
            Self {
                coins,
                received: false,
                time_alarms: TimeAlarmsRef::unchecked(TIME_ALARMS),
            }
        }

        pub fn set_received(&mut self, received: bool) {
            self.received = received;
        }
    }

    impl RemoteTransferOutTask for MockSpec {
        type G = SuperGroup;
        type Label = String;
        type StateResponse = Option<CoinsNb>;
        type Result = &'static str;

        fn label(&self) -> Self::Label {
            String::from(LABEL)
        }

        fn time_alarm(&self) -> &TimeAlarmsRef {
            &self.time_alarms
        }

        fn authz_remote_callback(
            &self,
            _querier: QuerierWrapper<'_>,
            _info: &MessageInfo,
        ) -> Result<()> {
            Ok(())
        }

        fn coins(&self) -> impl IntoIterator<Item = CoinDTO<SuperGroup>> {
            self.coins.clone()
        }

        fn schedule_transfer_out(&self, coin: &CoinDTO<SuperGroup>) -> Result<Batch> {
            transfer_request(coin)
        }

        fn decode_response(&self, payload: &[u8]) -> Result<()> {
            if payload == OK_PAYLOAD {
                Ok(())
            } else if payload == WRONG_VARIANT_PAYLOAD {
                Err(Error::unexpected_response_variant(
                    "a non-transfer operation response",
                ))
            } else {
                Err(Error::remote_swap_client("an undecodable payload"))
            }
        }

        fn all_received(&self, _account: &Addr, _querier: QuerierWrapper<'_>) -> Result<bool> {
            Ok(self.received)
        }

        fn finish(self, _env: &Env, _querier: QuerierWrapper<'_>) -> Self::Result {
            FINISH_RESULT
        }

        fn state(
            self,
            in_progress: DrainStage,
            _now: Instant,
            _due_projection: Duration,
            _querier: QuerierWrapper<'_>,
        ) -> Self::StateResponse {
            match in_progress {
                DrainStage::TransferOut { acks_left } => Some(acks_left),
                DrainStage::FundsArrival => None,
            }
        }
    }

    pub fn transfer_request(coin: &CoinDTO<SuperGroup>) -> Result<Batch> {
        let mut batch = Batch::default();
        batch
            .schedule_execute_wasm_no_reply_no_funds(
                Addr::unchecked(CONTROLLER),
                &TransferOutRequest { coin: *coin },
            )
            .map(|()| batch)
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use currency::test::{SuperGroupTestC1, SuperGroupTestC2};
    use cw_time::IntoInstant;
    use finance::{
        coin::{Amount, Coin, CoinDTO},
        duration::Duration,
    };
    use platform::{
        batch::{Batch, Emit, Emitter},
        ica::ErrorResponse as ICAErrorResponse,
        message::Response as MessageResponse,
    };
    use sdk::cosmwasm_std::{
        Addr, Binary, Env, MessageInfo, QuerierWrapper,
        testing::{self, MockQuerier},
    };

    use crate::{
        CoinsNb, Contract, Enterable,
        error::Error,
        impl_::{
            drain::State as DrainState,
            response::{Handler, Result as HandlerResult},
        },
    };

    use super::mock::{self, MockSpec};

    type G = <MockSpec as super::RemoteTransferOutTask>::G;
    type Node = super::RemoteTransferOut<MockSpec, DrainState<MockSpec>>;

    #[test]
    fn start_rejects_an_empty_task() {
        assert!(matches!(
            Node::start(MockSpec::new(vec![])),
            Err(Error::MissingTransferOutLeg)
        ));
    }

    #[test]
    fn enter_schedules_the_first_transfer() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let env = testing::mock_env();

        let node = started();
        assert_acks_left(2, &node);
        assert_eq!(
            mock::transfer_request(&coin1(100)).expect("a valid transfer request"),
            node.enter(env.block.time.into_instant(), querier)
                .expect("the first transfer should be scheduled")
        );
    }

    #[test]
    fn ack_advances_to_the_next_transfer() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let (response, node) =
            continued(started().on_remote_response(ok_payload(), querier, testing::mock_env()));
        assert_eq!(transfer_response(&coin2(70), 1), response);
        assert_acks_left(1, &node);
    }

    #[test]
    fn last_ack_without_funds_waits_for_arrival() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let env = testing::mock_env();

        let node = after_first_ack(querier);
        let resp = match node.on_remote_response(ok_payload(), querier, env.clone()) {
            HandlerResult::Continue(Ok(resp)) => resp,
            HandlerResult::Continue(Err(err)) => panic!("expected a continuation, got {err}"),
            HandlerResult::Finished(_result) => panic!("expected a continuation, got a finish"),
        };
        assert!(matches!(resp.next_state, DrainState::FundsArrival(_)));
        assert_eq!(waiting_response(&env), resp.response);
    }

    #[test]
    fn last_ack_with_funds_finishes() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let mut node = after_first_ack(querier);
        node.spec.set_received(true);
        assert_eq!(
            mock::FINISH_RESULT,
            finished(node.on_remote_response(ok_payload(), querier, testing::mock_env()))
        );
    }

    /// An error acknowledgment must neither re-emit the in-flight transfer
    /// nor advance the countdown - recovery is an operator heal
    #[test]
    fn remote_error_absorbed_without_reemission() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let (response, node) = continued(started().on_remote_error(
            ICAErrorResponse::from(String::from("transfer failed")),
            querier,
            testing::mock_env(),
        ));
        assert_eq!(absorb_response("remote-error"), response);
        assert_acks_left(2, &node);
    }

    #[test]
    fn timeout_reemits_the_in_flight_transfer() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let env = testing::mock_env();

        let (response, node) = continued(started().on_remote_timeout(querier, env.clone()));
        assert_eq!(timeout_response(&coin1(100), &env), response);
        assert_acks_left(2, &node);
    }

    #[test]
    fn garbage_payload_absorbed_without_state_change() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let (response, node) = continued(started().on_remote_response(
            Binary::from(b"garbage".as_slice()),
            querier,
            testing::mock_env(),
        ));
        assert_eq!(absorb_response("undecodable-response"), response);
        assert_acks_left(2, &node);
    }

    #[test]
    fn wrong_variant_payload_absorbed_without_state_change() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let (response, node) = continued(started().on_remote_response(
            Binary::from(mock::WRONG_VARIANT_PAYLOAD),
            querier,
            testing::mock_env(),
        ));
        assert_eq!(absorb_response("unexpected-response-variant"), response);
        assert_acks_left(2, &node);
    }

    #[test]
    fn heal_reemits_the_in_flight_transfer() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let node = after_first_ack(querier);
        let info = MessageInfo {
            sender: Addr::unchecked(mock::CONTROLLER),
            funds: vec![],
        };
        let (response, node) = continued(node.heal(querier, testing::mock_env(), &info));
        assert_eq!(
            MessageResponse::messages_with_event(
                mock::transfer_request(&coin2(70)).expect("a valid transfer request"),
                Emitter::of_type(mock::LABEL).emit("heal", "re-emit"),
            ),
            response
        );
        assert_acks_left(1, &node);
    }

    #[test]
    fn state_serde_round_trips() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let node = after_first_ack(querier);
        let serialized = sdk::cosmwasm_std::to_json_vec(&node).expect("a serializable state");
        let restored: Node =
            sdk::cosmwasm_std::from_json(&serialized).expect("the state should round-trip");
        assert_acks_left(node.acks_left, &restored);
        assert_eq!(
            serialized,
            sdk::cosmwasm_std::to_json_vec(&restored).expect("a serializable state")
        );
    }

    #[test]
    fn contract_state_reports_acks_left() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let env = testing::mock_env();

        assert_eq!(
            Some(2),
            started().state(
                env.block.time.into_instant(),
                Duration::from_secs(0),
                querier
            )
        );
    }

    fn started() -> Node {
        Node::start(spec2()).expect("a non-empty task")
    }

    fn after_first_ack(querier: QuerierWrapper<'_>) -> Node {
        let (_response, node) =
            continued(started().on_remote_response(ok_payload(), querier, testing::mock_env()));
        node
    }

    fn spec2() -> MockSpec {
        MockSpec::new(vec![coin1(100), coin2(70)])
    }

    fn assert_acks_left(expected: CoinsNb, node: &Node) {
        assert_eq!(expected, node.acks_left);
    }

    fn continued(res: HandlerResult<Node>) -> (MessageResponse, Node) {
        match res {
            HandlerResult::Continue(Ok(resp)) => match resp.next_state {
                DrainState::TransferOut(node) => (resp.response, node),
                DrainState::FundsArrival(_arrival) => {
                    panic!("expected the transfer-out stage, got the arrival one")
                }
            },
            HandlerResult::Continue(Err(err)) => panic!("expected a continuation, got {err}"),
            HandlerResult::Finished(_result) => panic!("expected a continuation, got a finish"),
        }
    }

    fn finished(res: HandlerResult<Node>) -> &'static str {
        match res {
            HandlerResult::Finished(result) => result,
            HandlerResult::Continue(_resp) => panic!("expected a finish, got a continuation"),
        }
    }

    fn transfer_response(coin: &CoinDTO<G>, acks_left: CoinsNb) -> MessageResponse {
        MessageResponse::messages_with_event(
            mock::transfer_request(coin).expect("a valid transfer request"),
            Emitter::of_type(mock::LABEL).emit_to_string_value("acks-left", acks_left),
        )
    }

    fn timeout_response(coin: &CoinDTO<G>, env: &Env) -> MessageResponse {
        MessageResponse::messages_with_event(
            mock::transfer_request(coin).expect("a valid transfer request"),
            Emitter::of_type(mock::LABEL)
                .emit("id", env.contract.address.clone())
                .emit("timeout", "retry"),
        )
    }

    fn absorb_response(reason: &str) -> MessageResponse {
        MessageResponse::messages_with_event(
            Batch::default(),
            Emitter::of_type(mock::LABEL).emit("absorbed", reason),
        )
    }

    fn waiting_response(env: &Env) -> MessageResponse {
        MessageResponse::messages_with_event(
            crate::impl_::transfer_in::setup_alarm(
                &timealarms::stub::TimeAlarmsRef::unchecked(mock::TIME_ALARMS),
                env.block.time.into_instant(),
            )
            .expect("a valid alarm setup"),
            Emitter::of_type(mock::LABEL).emit("stage", "funds-arrival"),
        )
    }

    fn ok_payload() -> Binary {
        Binary::from(mock::OK_PAYLOAD)
    }

    fn coin1(amount: Amount) -> CoinDTO<G> {
        Coin::<SuperGroupTestC1>::new(amount).into()
    }

    fn coin2(amount: Amount) -> CoinDTO<G> {
        Coin::<SuperGroupTestC2>::new(amount).into()
    }
}
