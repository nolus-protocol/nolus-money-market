//! # Funding the remote lease over the transfer channel
//!
//! The opening path funds the lease by ICS-20-transferring the downpayment
//! and the principal to the lease's Solana-side `LeaseAuthority` over the
//! paired transfer channel, then hands off to the opening swap. Unlike the
//! ICA transfer-out it replaces, there is no Interchain Account - the
//! receiver is the `LeaseAuthority` address the `OpenLease` acknowledgment
//! returned, carried by the task through [`FundingClient`].
//!
//! # One coin in flight at a time
//!
//! Each coin rides its own ICS-20 packet on the transfer channel, with its
//! own acknowledgment and timeout. The Cosmos sudo path discards the packet
//! sequence, so a callback can only be credited to the single in-flight
//! transfer the `acks_left` countdown tracks. The coins are therefore
//! scheduled strictly sequentially - the next one goes out only once the
//! in-flight one is acknowledged - so a re-emission can never duplicate an
//! already-landed coin. After the last acknowledgment proves the funds have
//! arrived, the workflow proceeds to the opening swap via [`NextLeg`].
//!
//! # Failure is forward-only
//!
//! A timeout or an error acknowledgment re-emits the in-flight coin verbatim:
//! ICS-20 refunds a failed transfer to the lease, and any earlier coin that
//! already landed on Solana cannot be pulled back, so the sequence only ever
//! moves forward. The customer-refund path stays reachable only before any
//! funding is emitted, in the `OpenLease` acknowledgment stage.

use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
};

use serde::{Deserialize, Serialize};

use cw_time::{IntoInstant, IntoTimestamp};
use finance::{coin::CoinDTO, duration::Duration, instant::Instant};
use platform::{
    bank_ibc::local::Sender as LocalSender,
    batch::{Batch, Emit, Emitter},
    ica::{ErrorResponse as ICAErrorResponse, HostAccount},
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Addr, Binary, Env, MessageInfo, QuerierWrapper};

use crate::{
    CoinsNb, Contract, ContractInSwap, Enterable, Stage, SwapTask as SwapTaskT, TimeAlarm,
    error::{Error, Result},
};

use super::{
    next_leg::NextLeg as NextLegT,
    response::{self, ContinueResult, Handler, Result as HandlerResult},
    timeout,
};

pub(super) const IBC_TIMEOUT: Duration = Duration::from_days(1); //enough for the relayers to process

const EVENT_KEY_ACKS_LEFT: &str = "acks-left";
const EVENT_KEY_HEAL: &str = "heal";
const EVENT_VALUE_REEMIT: &str = "re-emit";

/// The transfer-channel funding details a [`Funding`] leg needs but a plain
/// [`SwapTask`][SwapTaskT] does not carry
///
/// A separate trait rather than `SwapTask` methods so only the opening task
/// implements it; the swap tasks would be forced into `unimplemented!` stubs.
pub trait FundingClient
where
    Self: SwapTaskT,
{
    /// The Nolus account the funding transfers are sent from
    fn funding_sender(&self) -> &Addr;

    /// The Solana-side `LeaseAuthority` the funding transfers are addressed to
    fn funding_receiver(&self) -> &HostAccount;

    /// The local endpoint of the paired ICS-20 transfer channel
    fn transfer_channel(&self) -> &str;
}

/// Fund the remote lease by transferring a list of coins to its
/// `LeaseAuthority`, one in-flight at a time
///
/// The coins are scheduled strictly sequentially - the next one goes out only
/// once the in-flight one is acknowledged. The in-flight coin is identified by
/// `acks_left` against the deterministic [`SwapTask::coins`] order, so no coin
/// list is persisted. After the last acknowledgment the workflow proceeds with
/// `NextLeg`, the opening swap.
#[derive(Serialize, Deserialize)]
#[serde(
    bound(
        serialize = "SwapTask: Serialize",
        deserialize = "SwapTask: Deserialize<'de> + SwapTaskT"
    ),
    deny_unknown_fields,
    rename_all = "snake_case"
)]
pub struct Funding<SwapTask, SEnum, NextLeg> {
    spec: SwapTask,
    acks_left: CoinsNb,
    #[serde(skip)]
    _state_enum: PhantomData<SEnum>,
    #[serde(skip)]
    _next_leg: PhantomData<NextLeg>,
}

impl<SwapTask, SEnum, NextLeg> Funding<SwapTask, SEnum, NextLeg>
where
    SwapTask: SwapTaskT + FundingClient,
{
    /// Entry point of the funding sequence
    pub fn start(spec: SwapTask) -> Result<Self> {
        CoinsNb::try_from(Self::coins_nb(&spec))
            .map_err(|_too_many| Error::TransferOutLegsNbOverflow(CoinsNb::MAX))
            .and_then(|acks_left| {
                if acks_left == 0 {
                    Err(Error::MissingTransferOutLeg)
                } else {
                    Ok(Self::internal_new(spec, acks_left))
                }
            })
    }

    fn internal_new(spec: SwapTask, acks_left: CoinsNb) -> Self {
        let ret = Self {
            spec,
            acks_left,
            _state_enum: PhantomData,
            _next_leg: PhantomData,
        };
        debug_assert!(ret.invariant_held());
        ret
    }

    fn invariant_held(&self) -> bool {
        0 < self.acks_left && usize::from(self.acks_left) <= Self::coins_nb(&self.spec)
    }

    fn coins_nb(spec: &SwapTask) -> usize {
        spec.coins().into_iter().count()
    }

    fn in_flight_coin(&self) -> Result<CoinDTO<SwapTask::InG>> {
        debug_assert!(self.invariant_held());

        Self::coins_nb(&self.spec)
            .checked_sub(self.acks_left.into())
            .and_then(|coin_index| self.spec.coins().into_iter().nth(coin_index))
            .ok_or(Error::MissingTransferOutLeg)
    }

    /// Emit, or re-emit, the in-flight funding transfer
    ///
    /// Re-emissions repeat the original emission verbatim, keeping the
    /// timeout and heal recovery paths idempotent.
    fn schedule(&self, now: Instant) -> Result<Batch> {
        self.in_flight_coin().map(|coin| {
            let receiver = self.spec.funding_receiver();
            let mut sender = LocalSender::new(
                self.spec.transfer_channel(),
                self.spec.funding_sender(),
                receiver,
                (now + IBC_TIMEOUT).into_timestamp(),
                format!("Fund remote lease: {receiver}"),
            );
            sender.send(&coin);
            sender.into()
        })
    }

    fn emit_acks_left(&self) -> Emitter {
        Emitter::of_type(self.spec.label())
            .emit_to_string_value(EVENT_KEY_ACKS_LEFT, self.acks_left)
    }

    fn emit_heal(&self) -> Emitter {
        Emitter::of_type(self.spec.label()).emit(EVENT_KEY_HEAL, EVENT_VALUE_REEMIT)
    }
}

impl<SwapTask, SEnum, NextLeg> Funding<SwapTask, SEnum, NextLeg>
where
    SwapTask: SwapTaskT + FundingClient,
    Self: Handler<Response = SEnum, SwapResult = SwapTask::Result> + Into<SEnum>,
    NextLeg: NextLegT<SwapTask> + Handler<Response = SEnum, SwapResult = SwapTask::Result>,
{
    fn deliver_ack(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        debug_assert!(self.invariant_held());

        match self.acks_left.checked_sub(1) {
            None => Error::MissingTransferOutLeg.into(),
            Some(0) => NextLeg::enter_from(self.spec, querier, &env).map_into(),
            Some(acks_left) => {
                let now = env.block.time.into_instant();
                Self::internal_new(self.spec, acks_left)
                    .schedule_and_continue(now)
                    .into()
            }
        }
    }

    fn schedule_and_continue(self, now: Instant) -> ContinueResult<Self> {
        self.schedule(now).and_then(|batch| {
            response::res_continue::<_, _, Self>(
                MessageResponse::messages_with_event(batch, self.emit_acks_left()),
                self,
            )
        })
    }
}

impl<SwapTask, SEnum, NextLeg> Enterable for Funding<SwapTask, SEnum, NextLeg>
where
    SwapTask: SwapTaskT + FundingClient,
{
    fn enter(&self, now: Instant, _querier: QuerierWrapper<'_>) -> Result<Batch> {
        self.schedule(now)
    }
}

impl<SwapTask, SEnum, NextLeg> Handler for Funding<SwapTask, SEnum, NextLeg>
where
    SwapTask: SwapTaskT + FundingClient,
    Self: Into<SEnum>,
    NextLeg: NextLegT<SwapTask> + Handler<Response = SEnum, SwapResult = SwapTask::Result>,
{
    type Response = SEnum;
    type SwapResult = SwapTask::Result;

    fn authz_remote_callback(&self, querier: QuerierWrapper<'_>, info: &MessageInfo) -> Result<()> {
        self.spec.authz_remote_callback(querier, info)
    }

    /// A funding transfer's acknowledgment arrives over the sudo path on
    /// success only; its arrival is the proof the coin landed. The payload is
    /// the ICS-20 success constant and carries nothing to validate.
    fn on_response(
        self,
        _resp: Binary,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        self.deliver_ack(querier, env)
    }

    fn on_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
        let state_label = self.spec.label();
        timeout::on_timeout_retry(self, state_label, querier, env)
    }

    /// A funding error acknowledgment is forward-only, like a timeout: the
    /// transfer is refunded to the lease, so re-emitting the in-flight coin is
    /// the recovery.
    fn on_error(
        self,
        _response: ICAErrorResponse,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        self.on_timeout(querier, env).into()
    }

    /// Re-emit the in-flight funding transfer verbatim
    ///
    /// The permissionless operator recovery for an unresolvable packet. Like
    /// the drain transport, this payload-less transfer carries no per-emission
    /// nonce yet, so a heal issued while the original is still resolvable can
    /// duplicate the acknowledgment; nonce adoption is deferred to the
    /// remaining ibc-solray#142 phases.
    fn heal(
        self,
        _querier: QuerierWrapper<'_>,
        env: Env,
        _info: &MessageInfo,
    ) -> HandlerResult<Self> {
        self.schedule(env.block.time.into_instant())
            .and_then(|batch| {
                response::res_continue::<_, _, Self>(
                    MessageResponse::messages_with_event(batch, self.emit_heal()),
                    self,
                )
            })
            .into()
    }
}

impl<SwapTask, SEnum, NextLeg> Contract for Funding<SwapTask, SEnum, NextLeg>
where
    SwapTask: SwapTaskT + ContractInSwap<StateResponse = <SwapTask as SwapTaskT>::StateResponse>,
{
    type StateResponse = <SwapTask as SwapTaskT>::StateResponse;

    fn state(
        self,
        now: Instant,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        self.spec
            .state(Stage::TransferOut, now, due_projection, querier)
    }
}

impl<SwapTask, SEnum, NextLeg> Display for Funding<SwapTask, SEnum, NextLeg>
where
    SwapTask: SwapTaskT,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str("Funding at ")
            .and_then(|()| f.write_str(&self.spec.label().into()))
    }
}

impl<SwapTask, SEnum, NextLeg> TimeAlarm for Funding<SwapTask, SEnum, NextLeg>
where
    SwapTask: SwapTaskT,
{
    fn setup_alarm(&self, r#for: Instant) -> Result<Batch> {
        self.spec
            .time_alarm()
            .setup_alarm(r#for)
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use currency::test::{SuperGroup, SuperGroupTestC1, SuperGroupTestC2};
    use cw_time::IntoInstant;
    use finance::coin::{Coin, CoinDTO};
    use platform::ica::HostAccount;
    use sdk::cosmwasm_std::{
        Addr, MessageInfo, QuerierWrapper,
        testing::{self, MockQuerier},
    };

    use crate::{
        CoinsNb, Enterable, SwapTask, WithCalculator, WithOutputTask,
        error::{Error, Result as DexResult},
    };

    use super::{Funding, FundingClient};

    const CHANNEL: &str = "channel-7";
    const SENDER: &str = "nolus-lease";
    const RECEIVER: &str = "LeaseAuthorityPda11111111111111111111111111";

    type TestFunding = Funding<MockSpec, (), ()>;

    #[test]
    fn start_rejects_a_funding_without_coins() {
        assert!(matches!(
            TestFunding::start(MockSpec::new(vec![])),
            Err(Error::MissingTransferOutLeg)
        ));
    }

    #[test]
    fn start_sets_acks_left_to_the_coin_count() {
        assert_eq!(2, started().acks_left);
    }

    #[test]
    fn enter_schedules_the_first_coin() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let now = testing::mock_env().block.time.into_instant();

        assert!(
            !started()
                .enter(now, querier)
                .expect("the first funding transfer should be scheduled")
                .is_empty()
        );
    }

    #[test]
    fn serde_round_trips_carrying_acks_left() {
        let funding = started();
        let restored: TestFunding = sdk::cosmwasm_std::to_json_vec(&funding)
            .and_then(sdk::cosmwasm_std::from_json)
            .expect("the state should round-trip");
        assert_eq!(funding.acks_left, restored.acks_left);
        assert_eq!(funding.spec, restored.spec);
    }

    fn started() -> TestFunding {
        TestFunding::start(spec()).expect("a non-empty funding task")
    }

    fn spec() -> MockSpec {
        MockSpec::new(vec![
            Coin::<SuperGroupTestC1>::new(100).into(),
            Coin::<SuperGroupTestC2>::new(70).into(),
        ])
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    struct MockSpec {
        coins: Vec<CoinDTO<SuperGroup>>,
    }

    impl MockSpec {
        fn new(coins: Vec<CoinDTO<SuperGroup>>) -> Self {
            Self { coins }
        }
    }

    impl SwapTask for MockSpec {
        type InG = SuperGroup;
        type OutG = SuperGroup;
        type Label = String;
        type StateResponse = ();
        type Result = ();

        fn label(&self) -> Self::Label {
            String::from("FundingMock")
        }

        fn dex_account(&self) -> &crate::Account {
            unimplemented!("the funding node addresses the LeaseAuthority, not an ICA account")
        }

        fn time_alarm(&self) -> &timealarms::stub::TimeAlarmsRef {
            unimplemented!("the funding node tests do not set time alarms")
        }

        fn authz_remote_callback(
            &self,
            _querier: QuerierWrapper<'_>,
            _info: &MessageInfo,
        ) -> DexResult<()> {
            Ok(())
        }

        fn authz_anomaly_resolution(
            &self,
            _querier: QuerierWrapper<'_>,
            _info: &MessageInfo,
        ) -> DexResult<()> {
            Ok(())
        }

        fn timeout_retry_budget(&self) -> CoinsNb {
            0
        }

        fn slippage_escalation(&self) -> crate::SlippageEscalation {
            crate::SlippageEscalation::ReEmit
        }

        fn coins(&self) -> impl IntoIterator<Item = CoinDTO<SuperGroup>> {
            self.coins.clone()
        }

        fn with_slippage_calc<WithCalc>(&self, _with_calc: WithCalc) -> WithCalc::Output
        where
            WithCalc: WithCalculator<Self>,
        {
            unimplemented!("the funding node does not consult slippage")
        }

        fn into_output_task<Cmd>(self, _cmd: Cmd) -> Cmd::Output
        where
            Cmd: WithOutputTask<Self>,
        {
            unimplemented!("the funding node does not run the swap output task")
        }
    }

    impl FundingClient for MockSpec {
        fn funding_sender(&self) -> &Addr {
            const_sender()
        }

        fn funding_receiver(&self) -> &HostAccount {
            const_receiver()
        }

        fn transfer_channel(&self) -> &str {
            CHANNEL
        }
    }

    fn const_sender() -> &'static Addr {
        use std::sync::OnceLock;
        static SENDER_ADDR: OnceLock<Addr> = OnceLock::new();
        SENDER_ADDR.get_or_init(|| Addr::unchecked(SENDER))
    }

    fn const_receiver() -> &'static HostAccount {
        use std::sync::OnceLock;
        static RECEIVER_ADDR: OnceLock<HostAccount> = OnceLock::new();
        RECEIVER_ADDR.get_or_init(|| {
            HostAccount::try_from(String::from(RECEIVER)).expect("a valid host account")
        })
    }
}
