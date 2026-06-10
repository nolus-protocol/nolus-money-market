//! # Acknowledgment-to-leg correlation trust model
//!
//! `OperationResponse::Swap` carries no leg identifier, so acknowledgments
//! correlate to legs positionally: each one is credited to the single
//! in-flight leg the `acks_left` countdown tracks. The wire contract is
//! frozen; a per-leg nonce is a cross-repo follow-up. The positional
//! assumption rests on:
//!
//! - authorization - only the remote-lease controller passes
//!   [`Handler::authz_remote_callback`], so callbacks cannot be forged;
//! - the controller's delivery semantics - every emitted operation becomes
//!   exactly one IBC packet addressed back to this contract, and IBC core's
//!   packet-commitment bookkeeping makes the packet's acknowledgment and
//!   timeout paths mutually exclusive and at-most-once;
//! - sequential emission - the next leg goes out only once the in-flight
//!   one is acknowledged, so the regular flow keeps at most one operation
//!   outstanding;
//! - the pinned per-leg floor (`in_flight_min_out`) - a stray duplicate is
//!   mis-credited only if it also clears the *next* leg's own pinned floor.
//!
//! Residual risk: the controller keeps no per-lease in-flight bookkeeping
//! and the channel is unordered, so a [`Handler::heal`] issued while the
//! original packet is still resolvable solicits a second operation whose
//! callback is positionally credited as well. The transport records no
//! emission time and sets no alarm, leaving nothing to gate a heal on;
//! `heal` therefore stays permissionless but re-emits the leg verbatim -
//! same coin-in, same pinned floor - and operators must invoke it only
//! once the in-flight operation is known to be unresolvable (e.g., its
//! packet expired with no relayed timeout).

use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
};

use serde::{Deserialize, Serialize};

use currency::{CurrencyDTO, CurrencyDef, Group, MemberOf};
use finance::{
    coin::{Coin, CoinDTO, WithCoin},
    duration::Duration,
    instant::Instant,
    zero::Zero,
};
use platform::{
    batch::{Batch, Emit, Emitter},
    ica::ErrorResponse as ICAErrorResponse,
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Binary, Env, MessageInfo, QuerierWrapper};

use crate::{
    CoinsNb, Contract, ContractInRemoteSwap, Enterable, SlippageCalculator, SwapOutputTask,
    SwapTask as SwapTaskT, TimeAlarm, WithCalculator, WithOutputTask,
    error::{Error, Result},
    impl_::{
        next_leg::NextLeg,
        response::{self, ContinueResult, Handler, Result as HandlerResult},
        timeout,
    },
};

#[cfg(feature = "migration")]
use super::migration::{InspectSpec, MigrateSpec};

const EVENT_KEY_ABSORBED: &str = "absorbed";
const EVENT_KEY_ANOMALY: &str = "anomaly";
const EVENT_KEY_HEAL: &str = "heal";
const EVENT_KEY_TOTAL_OUT: &str = "total-out";
const EVENT_VALUE_REEMIT: &str = "re-emit";
const ABSORB_UNDECODABLE: &str = "undecodable-response";
const ABSORB_UNEXPECTED_VARIANT: &str = "unexpected-response-variant";
const ABSORB_CURRENCY_MISMATCH: &str = "out-currency-mismatch";
const ANOMALY_UNDER_MIN_OUT: &str = "under-min-out";

/// Transport of swap legs to a remote, non-ICA counterparty
///
/// Implemented by the swap specification itself since it is the only party
/// aware of the transport specifics. Bound as an extension of
/// [`SwapTask`][crate::SwapTask] to reuse its `InG`/`OutG` group associated
/// types instead of re-stating them as type parameters.
pub trait RemoteSwapClient
where
    Self: SwapTaskT,
{
    /// Schedule a swap of `coin_in` with the remote counterparty
    ///
    /// The transport guarantees a single response, error, or timeout
    /// per scheduled swap.
    fn schedule_swap(
        &self,
        coin_in: &CoinDTO<Self::InG>,
        min_out: &CoinDTO<Self::OutG>,
    ) -> Result<Batch>;

    /// Decode a swap response payload into the swapped-out coin
    fn decode_response(&self, payload: &[u8]) -> Result<CoinDTO<Self::OutG>>;
}

/// Swap a list of coins on a remote network, one in-flight leg at a time
///
/// Coins already denominated in the output currency are folded into the
/// accumulated total without a swap. The remaining coins, the swap legs, are
/// scheduled strictly sequentially - the next leg goes out only once the
/// in-flight one gets acknowledged. The in-flight leg is identified by
/// `acks_left` against the deterministic [`SwapTask::coins`] order, so no
/// coin list is persisted. The slippage floor promised for the in-flight
/// leg is pinned in `in_flight_min_out` when the leg is opened and reused
/// verbatim by every re-emission - acknowledgment validation never
/// consults the live oracle.
#[derive(Serialize, Deserialize)]
#[serde(
    bound(
        serialize = "SwapTask: Serialize",
        deserialize = "SwapTask: Deserialize<'de> + SwapTaskT"
    ),
    deny_unknown_fields,
    rename_all = "snake_case"
)]
pub struct RemoteSwap<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
{
    spec: SwapTask,
    acks_left: CoinsNb,
    total_out: CoinDTO<SwapTask::OutG>,
    in_flight_min_out: CoinDTO<SwapTask::OutG>,
    #[serde(skip)]
    _state_enum: PhantomData<SEnum>,
}

struct StartOrFinish<'env, 'querier, SEnum> {
    env: &'env Env,
    querier: QuerierWrapper<'querier>,
    _state_enum: PhantomData<SEnum>,
}

struct LegMinOut<'querier, SwapTask>
where
    SwapTask: SwapTaskT,
{
    coin_in: CoinDTO<SwapTask::InG>,
    out_currency: CurrencyDTO<SwapTask::OutG>,
    querier: QuerierWrapper<'querier>,
}

struct FinishWithTotal<'env, 'querier, SwapTask, HandlerT>
where
    SwapTask: SwapTaskT,
{
    total_out: CoinDTO<SwapTask::OutG>,
    env: &'env Env,
    querier: QuerierWrapper<'querier>,
    _handler: PhantomData<HandlerT>,
}

impl<SwapTask, SEnum> RemoteSwap<SwapTask, SEnum>
where
    SwapTask: SwapTaskT + RemoteSwapClient,
    Self: Handler<Response = SEnum, SwapResult = SwapTask::Result> + Into<SEnum>,
{
    /// Entry point of the remote swap leg sequence
    ///
    /// Folds the coins already in the output currency and schedules the
    /// first swap leg. If no coin needs a swap the task finishes
    /// synchronously.
    pub fn start(spec: SwapTask, env: &Env, querier: QuerierWrapper<'_>) -> HandlerResult<Self> {
        spec.into_output_task(StartOrFinish {
            env,
            querier,
            _state_enum: PhantomData::<SEnum>,
        })
    }

    /// An acknowledgment below the leg's pinned floor is a counterparty
    /// contract violation - the remote side enforces the very `min_out`
    /// this node emitted, persisted in `in_flight_min_out`. Validating
    /// against the pinned value rather than a fresh oracle quote keeps the
    /// check aligned with the promise actually made: price drift can
    /// neither retroactively reclassify a compliant, already-executed swap
    /// as underpaid nor admit an amount below the promised floor. The leg
    /// is re-emitted instead of accepted or absorbed, mirroring the error
    /// treatment: only the in-flight leg retries, the accumulated progress
    /// stays intact.
    fn deliver_ack(
        self,
        coin_out: CoinDTO<SwapTask::OutG>,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        if coin_out.currency() != self.total_out.currency() {
            return self.absorb(ABSORB_CURRENCY_MISMATCH).into();
        }
        if coin_out.amount() < self.in_flight_min_out.amount() {
            self.reemit_underpaid().into()
        } else {
            self.apply_ack(coin_out, querier, env)
        }
    }

    fn apply_ack(
        self,
        coin_out: CoinDTO<SwapTask::OutG>,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        debug_assert!(self.invariant_held());

        let total_out = add_coins(self.total_out, &coin_out);
        match self.acks_left.checked_sub(1) {
            None => Error::MissingSwapLeg.into(),
            Some(0) => self.spec.into_output_task(FinishWithTotal {
                total_out,
                env: &env,
                querier,
                _handler: PhantomData::<Self>,
            }),
            Some(acks_left) => Self::open_leg(self.spec, acks_left, total_out, querier)
                .and_then(Self::schedule_and_continue)
                .into(),
        }
    }

    fn schedule_and_continue(self) -> ContinueResult<Self> {
        self.schedule().and_then(|batch| {
            response::res_continue::<_, _, Self>(
                MessageResponse::messages_with_event(batch, self.emit_total_out()),
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

    fn reemit_underpaid(self) -> ContinueResult<Self> {
        self.schedule().and_then(|batch| {
            response::res_continue::<_, _, Self>(
                MessageResponse::messages_with_event(batch, self.emit_anomaly()),
                self,
            )
        })
    }

    fn emit_total_out(&self) -> Emitter {
        Emitter::of_type(self.spec.label()).emit_coin_dto(EVENT_KEY_TOTAL_OUT, &self.total_out)
    }

    fn emit_absorbed(&self, reason: &str) -> Emitter {
        Emitter::of_type(self.spec.label()).emit(EVENT_KEY_ABSORBED, reason)
    }

    fn emit_anomaly(&self) -> Emitter {
        Emitter::of_type(self.spec.label()).emit(EVENT_KEY_ANOMALY, ANOMALY_UNDER_MIN_OUT)
    }

    fn emit_heal(&self) -> Emitter {
        Emitter::of_type(self.spec.label()).emit(EVENT_KEY_HEAL, EVENT_VALUE_REEMIT)
    }
}

impl<SwapTask, SEnum> RemoteSwap<SwapTask, SEnum>
where
    SwapTask: SwapTaskT + RemoteSwapClient,
{
    /// Emit, or re-emit, the in-flight leg with its pinned floor
    ///
    /// Re-emissions repeat the exact promise of the original emission so
    /// the counterparty enforces one and the same floor however many
    /// times the leg goes out.
    fn schedule(&self) -> Result<Batch> {
        self.in_flight_leg()
            .and_then(|coin_in| self.spec.schedule_swap(&coin_in, &self.in_flight_min_out))
    }
}

impl<SwapTask, SEnum> RemoteSwap<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
{
    /// Open the leg `acks_left` points at, pinning its slippage floor
    ///
    /// The single place the oracle is consulted - the resulting floor is
    /// what the leg's emission and every re-emission promise, and what its
    /// acknowledgment is validated against.
    fn open_leg(
        spec: SwapTask,
        acks_left: CoinsNb,
        total_out: CoinDTO<SwapTask::OutG>,
        querier: QuerierWrapper<'_>,
    ) -> Result<Self> {
        in_flight_leg(&spec, total_out.currency(), acks_left)
            .and_then(|coin_in| leg_min_out(&spec, coin_in, total_out.currency(), querier))
            .map(|min_out| Self::internal_new(spec, acks_left, total_out, min_out))
    }

    fn internal_new(
        spec: SwapTask,
        acks_left: CoinsNb,
        total_out: CoinDTO<SwapTask::OutG>,
        in_flight_min_out: CoinDTO<SwapTask::OutG>,
    ) -> Self {
        let ret = Self {
            spec,
            acks_left,
            total_out,
            in_flight_min_out,
            _state_enum: PhantomData,
        };
        debug_assert!(ret.invariant_held());
        ret
    }

    fn invariant_held(&self) -> bool {
        0 < self.acks_left
            && usize::from(self.acks_left) <= self.legs_nb()
            && self.in_flight_min_out.currency() == self.total_out.currency()
    }

    fn legs_nb(&self) -> usize {
        swappable_coins(&self.spec, self.total_out.currency()).count()
    }

    fn in_flight_leg(&self) -> Result<CoinDTO<SwapTask::InG>> {
        debug_assert!(self.invariant_held());

        in_flight_leg(&self.spec, self.total_out.currency(), self.acks_left)
    }
}

impl<SwapTask, SEnum> Enterable for RemoteSwap<SwapTask, SEnum>
where
    SwapTask: SwapTaskT + RemoteSwapClient,
{
    fn enter(&self, _now: Instant, _querier: QuerierWrapper<'_>) -> Result<Batch> {
        self.schedule()
    }
}

impl<SwapTask, SEnum> NextLeg<SwapTask> for RemoteSwap<SwapTask, SEnum>
where
    SwapTask: SwapTaskT + RemoteSwapClient,
    Self: Handler<Response = SEnum, SwapResult = SwapTask::Result> + Into<SEnum>,
{
    /// Delegating to [`RemoteSwap::start`] keeps the fold semantics: coins
    /// already denominated in the output currency never become swap legs,
    /// and a task with nothing to swap finishes synchronously instead of
    /// waiting for an acknowledgment that would never arrive.
    fn enter_from(spec: SwapTask, querier: QuerierWrapper<'_>, env: &Env) -> HandlerResult<Self> {
        Self::start(spec, env, querier)
    }
}

impl<SwapTask, SEnum> Handler for RemoteSwap<SwapTask, SEnum>
where
    SwapTask: SwapTaskT + RemoteSwapClient,
    Self: Into<SEnum>,
{
    type Response = SEnum;
    type SwapResult = SwapTask::Result;

    fn authz_remote_callback(&self, querier: QuerierWrapper<'_>, info: &MessageInfo) -> Result<()> {
        self.spec.authz_remote_callback(querier, info)
    }

    /// Undecodable payloads, decodable-but-non-swap responses, and
    /// unexpected output currencies are absorbed with distinct event
    /// reasons instead of erroring - an error would revert the
    /// controller's acknowledgment transaction and strand the workflow,
    /// while the distinct reasons let operators tell wire garbage apart
    /// from protocol confusion. A successfully decoded acknowledgment,
    /// though, runs the regular flow and lets any downstream failure
    /// propagate.
    fn on_remote_response(
        self,
        data: Binary,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        match self.spec.decode_response(data.as_slice()) {
            Ok(coin_out) => self.deliver_ack(coin_out, querier, env),
            Err(Error::UnexpectedResponseVariant(_details)) => {
                self.absorb(ABSORB_UNEXPECTED_VARIANT).into()
            }
            Err(_undecodable) => self.absorb(ABSORB_UNDECODABLE).into(),
        }
    }

    /// Anomalies are deliberately not routed through the spec's
    /// `on_anomaly` - its `Retry` treatment rebuilds the node from the spec
    /// and would re-issue the already-acknowledged legs. Only the in-flight
    /// leg is re-emitted, preserving the accumulated progress.
    fn on_remote_error(
        self,
        _response: ICAErrorResponse,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        self.on_remote_timeout(querier, env)
    }

    fn on_remote_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        let state_label = self.spec.label();
        timeout::on_timeout_retry(self, state_label, querier, env).into()
    }

    /// The only operator recovery on this transport - there is neither a
    /// sudo timeout nor a time alarm - hence re-emitting the in-flight leg
    /// must stay idempotent: the re-emission repeats the pinned
    /// `in_flight_min_out`, the exact promise of the original emission.
    /// See the module doc for the duplicate-acknowledgment risk a heal
    /// issued while the original operation is still resolvable creates.
    fn heal(self, _querier: QuerierWrapper<'_>, _env: Env) -> HandlerResult<Self> {
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

impl<SwapTask, SEnum> Contract for RemoteSwap<SwapTask, SEnum>
where
    SwapTask:
        SwapTaskT + ContractInRemoteSwap<StateResponse = <SwapTask as SwapTaskT>::StateResponse>,
{
    type StateResponse = <SwapTask as SwapTaskT>::StateResponse;

    fn state(
        self,
        now: Instant,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        self.spec
            .state(self.acks_left, now, due_projection, querier)
    }
}

impl<SwapTask, SEnum> Display for RemoteSwap<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str("RemoteSwap at ")
            .and_then(|()| f.write_str(&Into::<String>::into(self.spec.label())))
    }
}

impl<SwapTask, SEnum> TimeAlarm for RemoteSwap<SwapTask, SEnum>
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

#[cfg(feature = "migration")]
impl<SwapTask, SwapTaskNew, SEnum, SEnumNew> MigrateSpec<SwapTask, SwapTaskNew, SEnumNew>
    for RemoteSwap<SwapTask, SEnum>
where
    Self: Sized,
    SwapTask: SwapTaskT,
    SwapTaskNew: SwapTaskT<OutG = SwapTask::OutG>,
    RemoteSwap<SwapTaskNew, SEnumNew>: Into<SEnumNew>,
{
    type Out = RemoteSwap<SwapTaskNew, SEnumNew>;

    /// The in-flight progress - `acks_left`, `total_out`, and the pinned
    /// `in_flight_min_out` - is carried over instead of rebuilding from
    /// the spec: a rebuild would re-issue the already-acknowledged legs
    /// and re-price the promise made for the in-flight one.
    fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
    where
        MigrateFn: FnOnce(SwapTask) -> SwapTaskNew,
    {
        Self::Out::internal_new(
            migrate_fn(self.spec),
            self.acks_left,
            self.total_out,
            self.in_flight_min_out,
        )
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, R, SEnum> InspectSpec<SwapTask, R> for RemoteSwap<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
{
    fn inspect_spec<InspectFn>(&self, inspect_fn: InspectFn) -> R
    where
        InspectFn: FnOnce(&SwapTask) -> R,
    {
        inspect_fn(&self.spec)
    }
}

impl<SwapTask, SEnum> WithOutputTask<SwapTask> for StartOrFinish<'_, '_, SEnum>
where
    SwapTask: SwapTaskT + RemoteSwapClient,
    RemoteSwap<SwapTask, SEnum>:
        Handler<Response = SEnum, SwapResult = SwapTask::Result> + Into<SEnum>,
{
    type Output = HandlerResult<RemoteSwap<SwapTask, SEnum>>;

    fn on<OutC, OutputTaskT>(self, task: OutputTaskT) -> Self::Output
    where
        OutC: CurrencyDef,
        OutC::Group: MemberOf<<SwapTask::OutG as Group>::TopG> + MemberOf<SwapTask::OutG>,
        OutputTaskT: SwapOutputTask<SwapTask, OutC = OutC>,
    {
        let (out_total, legs_nb) = fold_out_coins::<OutC, SwapTask>(task.as_spec());
        if legs_nb == 0 {
            response::res_finished(task.finish(out_total, self.env, self.querier))
        } else {
            CoinsNb::try_from(legs_nb)
                .map_err(|_too_many| Error::SwapLegsNbOverflow(CoinsNb::MAX))
                .and_then(|acks_left| {
                    RemoteSwap::open_leg(
                        task.into_spec(),
                        acks_left,
                        out_total.into(),
                        self.querier,
                    )
                })
                .and_then(RemoteSwap::schedule_and_continue)
                .into()
        }
    }
}

impl<SwapTask> WithCalculator<SwapTask> for LegMinOut<'_, SwapTask>
where
    SwapTask: SwapTaskT,
{
    type Output = Result<CoinDTO<SwapTask::OutG>>;

    fn on<CalculatorT>(self, calculator: &CalculatorT) -> Self::Output
    where
        CalculatorT: SlippageCalculator<SwapTask::InG>,
        <<CalculatorT as SlippageCalculator<SwapTask::InG>>::OutC as CurrencyDef>::Group:
            MemberOf<SwapTask::OutG> + MemberOf<<SwapTask::InG as Group>::TopG>,
    {
        debug_assert!(self.out_currency == *CalculatorT::OutC::dto());

        calculator
            .min_output(&self.coin_in, self.querier)
            .map(Into::into)
    }
}

impl<SwapTask, HandlerT> WithOutputTask<SwapTask> for FinishWithTotal<'_, '_, SwapTask, HandlerT>
where
    SwapTask: SwapTaskT,
    HandlerT: Handler<SwapResult = SwapTask::Result>,
{
    type Output = HandlerResult<HandlerT>;

    fn on<OutC, OutputTaskT>(self, task: OutputTaskT) -> Self::Output
    where
        OutC: CurrencyDef,
        OutC::Group: MemberOf<<SwapTask::OutG as Group>::TopG> + MemberOf<SwapTask::OutG>,
        OutputTaskT: SwapOutputTask<SwapTask, OutC = OutC>,
    {
        response::res_finished(task.finish(
            self.total_out.as_specific(OutC::dto()),
            self.env,
            self.querier,
        ))
    }
}

fn swappable_coins<SwapTask>(
    spec: &SwapTask,
    out_currency: CurrencyDTO<SwapTask::OutG>,
) -> impl Iterator<Item = CoinDTO<SwapTask::InG>>
where
    SwapTask: SwapTaskT,
{
    spec.coins()
        .into_iter()
        .filter(move |coin| coin.currency() != out_currency)
}

fn in_flight_leg<SwapTask>(
    spec: &SwapTask,
    out_currency: CurrencyDTO<SwapTask::OutG>,
    acks_left: CoinsNb,
) -> Result<CoinDTO<SwapTask::InG>>
where
    SwapTask: SwapTaskT,
{
    swappable_coins(spec, out_currency)
        .count()
        .checked_sub(acks_left.into())
        .and_then(|leg_index| swappable_coins(spec, out_currency).nth(leg_index))
        .ok_or(Error::MissingSwapLeg)
}

fn leg_min_out<SwapTask>(
    spec: &SwapTask,
    coin_in: CoinDTO<SwapTask::InG>,
    out_currency: CurrencyDTO<SwapTask::OutG>,
    querier: QuerierWrapper<'_>,
) -> Result<CoinDTO<SwapTask::OutG>>
where
    SwapTask: SwapTaskT,
{
    spec.with_slippage_calc(LegMinOut {
        coin_in,
        out_currency,
        querier,
    })
}

fn fold_out_coins<OutC, SwapTask>(spec: &SwapTask) -> (Coin<OutC>, usize)
where
    OutC: CurrencyDef,
    OutC::Group: MemberOf<<SwapTask::OutG as Group>::TopG> + MemberOf<SwapTask::OutG>,
    SwapTask: SwapTaskT,
{
    spec.coins()
        .into_iter()
        .fold((Coin::ZERO, 0), |(out_total, legs_nb), coin| {
            if coin.currency() == *OutC::dto() {
                (
                    out_total
                        + coin
                            .into_super_group::<<SwapTask::InG as Group>::TopG>()
                            .as_specific(OutC::dto()),
                    legs_nb,
                )
            } else {
                (out_total, legs_nb + 1)
            }
        })
}

fn add_coins<G>(total: CoinDTO<G>, more: &CoinDTO<G>) -> CoinDTO<G>
where
    G: Group,
{
    struct AddOther<'more, G>
    where
        G: Group,
    {
        more: &'more CoinDTO<G>,
    }

    impl<G> WithCoin<G> for AddOther<'_, G>
    where
        G: Group,
    {
        type Outcome = CoinDTO<G>;

        fn on<C>(self, total: Coin<C>) -> Self::Outcome
        where
            C: CurrencyDef,
            C::Group: MemberOf<G> + MemberOf<G::TopG>,
        {
            (total + self.more.as_specific(C::dto())).into()
        }
    }

    debug_assert_eq!(total.currency(), more.currency());

    total.with_coin(AddOther { more })
}

#[cfg(test)]
pub(super) mod mock {
    use serde::{Deserialize, Serialize};

    use currency::{
        CurrencyDTO, Group, MemberOf,
        test::{SuperGroup, SuperGroupTestC1},
    };
    use finance::coin::{Amount, Coin, CoinDTO};
    use oracle::{
        api::swap::{Result as SwapPathResult, SwapTarget},
        stub::SwapPath,
    };
    use platform::batch::Batch;
    use sdk::cosmwasm_std::{Addr, Env, MessageInfo, QuerierWrapper};
    use timealarms::stub::TimeAlarmsRef;

    use crate::{
        Account, AnomalyTreatment, CoinsNb, ContractInRemoteSwap, SlippageCalculator,
        SwapOutputTask, SwapTask, WithCalculator, WithOutputTask,
        error::{Error, Result},
    };

    use super::RemoteSwapClient;

    pub const LABEL: &str = "RemoteSwapMock";
    pub const CONTROLLER: &str = "controller";
    pub const WRONG_VARIANT_PAYLOAD: &[u8] = b"wrong-variant";

    const DEFAULT_FLOOR: Amount = 1;

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub struct MockSpec {
        coins: Vec<CoinDTO<SuperGroup>>,
        floor: Amount,
    }

    #[derive(Serialize)]
    struct SwapRequest {
        coin_in: CoinDTO<SuperGroup>,
        min_out: CoinDTO<SuperGroup>,
    }

    struct FloorCalculator {
        floor: Amount,
    }

    struct NoSwapPath;

    impl MockSpec {
        pub fn new(coins: Vec<CoinDTO<SuperGroup>>) -> Self {
            Self {
                coins,
                floor: DEFAULT_FLOOR,
            }
        }

        pub fn set_floor(&mut self, floor: Amount) {
            self.floor = floor;
        }
    }

    impl SwapTask for MockSpec {
        type InG = SuperGroup;
        type OutG = SuperGroup;
        type Label = String;
        type StateResponse = CoinsNb;
        type Result = CoinDTO<SuperGroup>;

        fn label(&self) -> Self::Label {
            String::from(LABEL)
        }

        fn dex_account(&self) -> &Account {
            unimplemented!("the remote swap node must not use the ICA account")
        }

        fn oracle(&self) -> &impl SwapPath<SuperGroup> {
            &NoSwapPath
        }

        fn time_alarm(&self) -> &TimeAlarmsRef {
            unimplemented!("the remote swap node tests do not set time alarms")
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

        fn with_slippage_calc<WithCalc>(&self, with_calc: WithCalc) -> WithCalc::Output
        where
            WithCalc: WithCalculator<Self>,
        {
            with_calc.on(&FloorCalculator { floor: self.floor })
        }

        fn into_output_task<Cmd>(self, cmd: Cmd) -> Cmd::Output
        where
            Cmd: WithOutputTask<Self>,
        {
            cmd.on(self)
        }
    }

    impl SwapOutputTask<Self> for MockSpec {
        type OutC = SuperGroupTestC1;

        fn as_spec(&self) -> &Self {
            self
        }

        fn into_spec(self) -> Self {
            self
        }

        fn on_anomaly(self) -> AnomalyTreatment<Self> {
            unreachable!("the remote swap node must not route anomalies through the spec")
        }

        fn finish(
            self,
            amount_out: Coin<Self::OutC>,
            _env: &Env,
            _querier: QuerierWrapper<'_>,
        ) -> CoinDTO<SuperGroup> {
            amount_out.into()
        }
    }

    impl ContractInRemoteSwap for MockSpec {
        type StateResponse = CoinsNb;

        fn state(
            self,
            acks_left: CoinsNb,
            _now: finance::instant::Instant,
            _due_projection: finance::duration::Duration,
            _querier: QuerierWrapper<'_>,
        ) -> Self::StateResponse {
            acks_left
        }
    }

    impl RemoteSwapClient for MockSpec {
        fn schedule_swap(
            &self,
            coin_in: &CoinDTO<SuperGroup>,
            min_out: &CoinDTO<SuperGroup>,
        ) -> Result<Batch> {
            swap_request(coin_in, min_out)
        }

        fn decode_response(&self, payload: &[u8]) -> Result<CoinDTO<SuperGroup>> {
            if payload == WRONG_VARIANT_PAYLOAD {
                Err(Error::unexpected_response_variant(
                    "a non-swap operation response",
                ))
            } else {
                sdk::cosmwasm_std::from_json(payload).map_err(Error::remote_swap_client)
            }
        }
    }

    impl SlippageCalculator<SuperGroup> for FloorCalculator {
        type OutC = SuperGroupTestC1;

        fn min_output(
            &self,
            _input: &CoinDTO<SuperGroup>,
            _querier: QuerierWrapper<'_>,
        ) -> Result<Coin<SuperGroupTestC1>> {
            Ok(Coin::new(self.floor))
        }
    }

    impl SwapPath<SuperGroup> for NoSwapPath {
        fn swap_path<SwapIn, SwapOut>(
            &self,
            _from: CurrencyDTO<SwapIn>,
            _to: CurrencyDTO<SwapOut>,
            _querier: QuerierWrapper<'_>,
        ) -> SwapPathResult<Vec<SwapTarget<SuperGroup>>>
        where
            SwapIn: Group + MemberOf<SuperGroup>,
            SwapOut: Group + MemberOf<SuperGroup>,
        {
            unimplemented!("the remote swap node must not consult the swap path oracle")
        }
    }

    pub fn swap_request(
        coin_in: &CoinDTO<SuperGroup>,
        min_out: &CoinDTO<SuperGroup>,
    ) -> Result<Batch> {
        let mut batch = Batch::default();
        batch
            .schedule_execute_wasm_no_reply_no_funds(
                Addr::unchecked(CONTROLLER),
                &SwapRequest {
                    coin_in: *coin_in,
                    min_out: *min_out,
                },
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
        Binary, Env, QuerierWrapper,
        testing::{self, MockQuerier},
    };

    use crate::{
        CoinsNb, Contract,
        impl_::response::{Handler, Result as HandlerResult},
    };

    use super::mock::{self, MockSpec};

    type OutG = <MockSpec as crate::SwapTask>::OutG;
    type Node = super::RemoteSwap<MockSpec, TestState>;

    enum TestState {
        RemoteSwap(Node),
    }

    impl From<Node> for TestState {
        fn from(node: Node) -> Self {
            Self::RemoteSwap(node)
        }
    }

    #[test]
    fn start_schedules_first_leg_and_folds_out_coins() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let (response, node) = continued(Node::start(spec3(), &testing::mock_env(), querier));
        assert_eq!(
            leg_response(&coin_in(100), &min_out(), &coin_out(50)),
            response
        );
        assert_node(2, &coin_out(50), &min_out(), &node);
    }

    #[test]
    fn start_finishes_when_all_coins_are_in_out_currency() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let spec = MockSpec::new(vec![coin_out(50), coin_out(70)]);

        assert_eq!(
            coin_out(120),
            finished(Node::start(spec, &testing::mock_env(), querier))
        );
    }

    #[test]
    fn ack_accumulates_and_schedules_next_leg() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let (_response, node) = continued(Node::start(spec3(), &testing::mock_env(), querier));
        let (response, node) = continued(node.on_remote_response(
            payload(&coin_out(30)),
            querier,
            testing::mock_env(),
        ));
        assert_eq!(
            leg_response(&coin_in(70), &min_out(), &coin_out(80)),
            response
        );
        assert_node(1, &coin_out(80), &min_out(), &node);
    }

    /// The pinned floor, not a fresh quote, validates the acknowledgment:
    /// raising what a recompute would demand must not reject an
    /// acknowledgment compliant with the floor promised at emission.
    #[test]
    fn ack_at_pinned_floor_accepted_despite_higher_current_floor() {
        const PINNED_FLOOR: Amount = 30;
        const RAISED_FLOOR: Amount = 100;
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let mut spec = spec3();
        spec.set_floor(PINNED_FLOOR);
        let (_response, mut node) = continued(Node::start(spec, &testing::mock_env(), querier));
        node.spec.set_floor(RAISED_FLOOR);

        let (response, node) = continued(node.on_remote_response(
            payload(&coin_out(PINNED_FLOOR)),
            querier,
            testing::mock_env(),
        ));
        // the next leg, though, pins the raised floor at its emission
        assert_eq!(
            leg_response(
                &coin_in(70),
                &coin_out(RAISED_FLOOR),
                &coin_out(50 + PINNED_FLOOR)
            ),
            response
        );
        assert_node(
            1,
            &coin_out(50 + PINNED_FLOOR),
            &coin_out(RAISED_FLOOR),
            &node,
        );
    }

    #[test]
    fn final_ack_finishes_with_accumulated_total() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let node = after_first_ack(querier);
        assert_eq!(
            coin_out(120),
            finished(node.on_remote_response(payload(&coin_out(40)), querier, testing::mock_env()))
        );
    }

    #[test]
    fn timeout_reemits_only_the_in_flight_leg() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let env = testing::mock_env();

        let (response, node) =
            continued(after_first_ack(querier).on_remote_timeout(querier, env.clone()));
        assert_eq!(timeout_response(&coin_in(70), &env), response);
        assert_node(1, &coin_out(80), &min_out(), &node);
    }

    /// The re-emission repeats the floor pinned at the leg's emission even
    /// if a recompute at acknowledgment time would demand more.
    #[test]
    fn underpaid_ack_reemits_with_the_pinned_floor() {
        const PINNED_FLOOR: Amount = 40;
        const RAISED_FLOOR: Amount = 100;
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let mut spec = spec3();
        spec.set_floor(PINNED_FLOOR);
        let (_response, mut node) = continued(Node::start(spec, &testing::mock_env(), querier));
        node.spec.set_floor(RAISED_FLOOR);

        let (response, node) = continued(node.on_remote_response(
            payload(&coin_out(PINNED_FLOOR - 1)),
            querier,
            testing::mock_env(),
        ));
        assert_eq!(
            MessageResponse::messages_with_event(
                mock::swap_request(&coin_in(100), &coin_out(PINNED_FLOOR))
                    .expect("a valid swap request"),
                Emitter::of_type(mock::LABEL).emit("anomaly", "under-min-out"),
            ),
            response
        );
        assert_node(2, &coin_out(50), &coin_out(PINNED_FLOOR), &node);
    }

    #[test]
    fn error_reemits_preserving_progress() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let env = testing::mock_env();

        let (response, node) = continued(after_first_ack(querier).on_remote_error(
            ICAErrorResponse::from(String::from("swap failed")),
            querier,
            env.clone(),
        ));
        assert_eq!(timeout_response(&coin_in(70), &env), response);
        assert_node(1, &coin_out(80), &min_out(), &node);
    }

    #[test]
    fn garbage_payload_absorbed_without_state_change() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let (_response, node) = continued(Node::start(spec3(), &testing::mock_env(), querier));
        let (response, node) = continued(node.on_remote_response(
            Binary::from(b"garbage".as_slice()),
            querier,
            testing::mock_env(),
        ));
        assert_eq!(absorb_response("undecodable-response"), response);
        assert_node(2, &coin_out(50), &min_out(), &node);
    }

    #[test]
    fn wrong_variant_payload_absorbed_without_state_change() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let (_response, node) = continued(Node::start(spec3(), &testing::mock_env(), querier));
        let (response, node) = continued(node.on_remote_response(
            Binary::from(mock::WRONG_VARIANT_PAYLOAD),
            querier,
            testing::mock_env(),
        ));
        assert_eq!(absorb_response("unexpected-response-variant"), response);
        assert_node(2, &coin_out(50), &min_out(), &node);
    }

    #[test]
    fn mismatched_out_currency_absorbed_without_state_change() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let (_response, node) = continued(Node::start(spec3(), &testing::mock_env(), querier));
        let (response, node) =
            continued(node.on_remote_response(payload(&coin_in(30)), querier, testing::mock_env()));
        assert_eq!(absorb_response("out-currency-mismatch"), response);
        assert_node(2, &coin_out(50), &min_out(), &node);
    }

    /// Healing repeats the floor pinned when the leg was opened - a floor
    /// raised afterwards must not leak into the re-emission.
    #[test]
    fn heal_reemits_the_in_flight_leg_with_the_pinned_floor() {
        const RAISED_FLOOR: Amount = 100;
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let mut node = after_first_ack(querier);
        node.spec.set_floor(RAISED_FLOOR);

        let (response, node) = continued(node.heal(querier, testing::mock_env()));
        assert_eq!(
            MessageResponse::messages_with_event(
                mock::swap_request(&coin_in(70), &min_out()).expect("a valid swap request"),
                Emitter::of_type(mock::LABEL).emit("heal", "re-emit"),
            ),
            response
        );
        assert_node(1, &coin_out(80), &min_out(), &node);
    }

    #[test]
    fn state_serde_round_trips() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let node = after_first_ack(querier);
        let restored: Node = sdk::cosmwasm_std::to_json_vec(&node)
            .and_then(sdk::cosmwasm_std::from_json)
            .expect("the state should round-trip");
        assert_eq!(node.spec, restored.spec);
        assert_node(
            node.acks_left,
            &node.total_out,
            &node.in_flight_min_out,
            &restored,
        );
    }

    #[test]
    fn contract_state_reports_acks_left() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let env = testing::mock_env();

        let (_response, node) = continued(Node::start(spec3(), &env, querier));
        assert_eq!(
            2,
            node.state(
                env.block.time.into_instant(),
                Duration::from_secs(0),
                querier
            )
        );
    }

    fn assert_node(
        expected_acks: CoinsNb,
        expected_total: &CoinDTO<OutG>,
        expected_pinned: &CoinDTO<OutG>,
        node: &Node,
    ) {
        assert_eq!(expected_acks, node.acks_left);
        assert_eq!(*expected_total, node.total_out);
        assert_eq!(*expected_pinned, node.in_flight_min_out);
    }

    fn continued(res: HandlerResult<Node>) -> (MessageResponse, Node) {
        match res {
            HandlerResult::Continue(Ok(resp)) => {
                let TestState::RemoteSwap(node) = resp.next_state;
                (resp.response, node)
            }
            HandlerResult::Continue(Err(err)) => panic!("expected a continuation, got {err}"),
            HandlerResult::Finished(_total) => panic!("expected a continuation, got a finish"),
        }
    }

    fn finished(res: HandlerResult<Node>) -> CoinDTO<OutG> {
        match res {
            HandlerResult::Finished(total) => total,
            HandlerResult::Continue(_resp) => panic!("expected a finish, got a continuation"),
        }
    }

    fn after_first_ack(querier: QuerierWrapper<'_>) -> Node {
        let (_response, node) = continued(Node::start(spec3(), &testing::mock_env(), querier));
        let (_response, node) = continued(node.on_remote_response(
            payload(&coin_out(30)),
            querier,
            testing::mock_env(),
        ));
        node
    }

    fn spec3() -> MockSpec {
        MockSpec::new(vec![coin_in(100), coin_out(50), coin_in(70)])
    }

    fn leg_response(
        leg: &CoinDTO<OutG>,
        min_out: &CoinDTO<OutG>,
        total: &CoinDTO<OutG>,
    ) -> MessageResponse {
        MessageResponse::messages_with_event(
            mock::swap_request(leg, min_out).expect("a valid swap request"),
            Emitter::of_type(mock::LABEL).emit_coin_dto("total-out", total),
        )
    }

    fn timeout_response(leg: &CoinDTO<OutG>, env: &Env) -> MessageResponse {
        MessageResponse::messages_with_event(
            mock::swap_request(leg, &min_out()).expect("a valid swap request"),
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

    fn payload(coin: &CoinDTO<OutG>) -> Binary {
        sdk::cosmwasm_std::to_json_binary(coin).expect("a serializable coin")
    }

    fn coin_out(amount: Amount) -> CoinDTO<OutG> {
        Coin::<SuperGroupTestC1>::new(amount).into()
    }

    fn coin_in(amount: Amount) -> CoinDTO<OutG> {
        Coin::<SuperGroupTestC2>::new(amount).into()
    }

    fn min_out() -> CoinDTO<OutG> {
        coin_out(1)
    }
}
