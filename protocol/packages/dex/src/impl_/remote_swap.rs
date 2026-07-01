//! # Acknowledgment-to-leg correlation trust model
//!
//! Every emission rides a per-emission `nonce` on the packet envelope. The
//! controller reads it back from the original outbound packet on ack/timeout
//! and returns it in the callback, so an acknowledgment is credited to the
//! exact emission that solicited it: a callback whose nonce differs from the
//! in-flight one is absorbed (`nonce-mismatch`) without touching progress. The
//! node still tracks the in-flight leg positionally through the `acks_left`
//! countdown; the nonce disambiguates *which emission* of that leg a callback
//! belongs to. Correctness rests on:
//!
//! - authorization - only the remote-lease controller passes
//!   [`Handler::authz_remote_callback`], so callbacks cannot be forged;
//! - the controller's delivery semantics - every emitted operation becomes
//!   exactly one IBC packet addressed back to this contract, and IBC core's
//!   packet-commitment bookkeeping makes the packet's acknowledgment and
//!   timeout paths mutually exclusive and at-most-once;
//! - the strictly-monotonic `in_flight_nonce` - every emission, including each
//!   re-emission and operator [`Handler::heal`], bumps it, so a packet
//!   superseded by a later emission carries a smaller nonce and its late
//!   callback is rejected. This closes the duplicate-acknowledgment window that
//!   a `heal` issued while the original packet is still resolvable would
//!   otherwise open: the original's late ack no longer matches the in-flight
//!   emission and is absorbed instead of positionally mis-credited to a
//!   consecutive leg sharing the input currency;
//! - the pinned per-leg floor (`in_flight_min_out`) - a re-emission repeats the
//!   exact promise of the original, so the counterparty enforces one and the
//!   same floor however many times the leg goes out.
//!
//! `heal` therefore stays permissionless and re-emits the leg verbatim - same
//! coin-in, same pinned floor, fresh nonce - leaving idempotency to the nonce
//! match rather than to operator timing.

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
    CoinsNb, Contract, ContractInRemoteSwap, Enterable, SlippageCalculator, SlippageEscalation,
    SwapOutputTask, SwapTask as SwapTaskT, TimeAlarm, WithCalculator, WithOutputTask,
    error::{Error, Result},
    impl_::{
        SlippageAnomaly,
        next_leg::NextLeg,
        response::{self, ContinueResult, Handler, Result as HandlerResult},
        timeout,
    },
};

const EVENT_KEY_ABSORBED: &str = "absorbed";
const EVENT_KEY_ANOMALY: &str = "anomaly";
const EVENT_KEY_HEAL: &str = "heal";
const EVENT_KEY_TOTAL_OUT: &str = "total-out";
const EVENT_VALUE_REEMIT: &str = "re-emit";
const ABSORB_UNDECODABLE: &str = "undecodable-response";
const ABSORB_UNEXPECTED_VARIANT: &str = "unexpected-response-variant";
const ABSORB_CURRENCY_MISMATCH: &str = "out-currency-mismatch";
const ABSORB_OUTPUT_OVERFLOW: &str = "output-overflow";
const ABSORB_NONCE_MISMATCH: &str = "nonce-mismatch";
/// Predecessor nonce for the very first emission of a node: nothing has been
/// emitted yet, so the first leg opens at `NO_PRIOR_NONCE + 1`.
const NO_PRIOR_NONCE: u64 = 0;
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
    /// per scheduled swap. `nonce` is the per-emission correlation identifier
    /// the node assigns; it must ride the packet envelope so the controller can
    /// return it in the callback and the node can match the acknowledgment to
    /// this emission.
    fn schedule_swap(
        &self,
        coin_in: &CoinDTO<Self::InG>,
        min_out: &CoinDTO<Self::OutG>,
        nonce: u64,
    ) -> Result<Batch>;

    /// Decode a swap response payload into the swapped-out coin
    fn decode_response(&self, payload: &[u8]) -> Result<CoinDTO<Self::OutG>>;

    /// Clean-unwind the swap inputs home after a zero-acked hard error
    ///
    /// Invoked by [`RemoteSwap::on_remote_error`] only when
    /// [`SwapTask::unwind_on_zero_acked`][crate::SwapTask::unwind_on_zero_acked]
    /// returns `true` and no leg has acknowledged yet, so nothing has been
    /// swapped on the remote side and every input is still recoverable. The
    /// spec owns the transition because the unwind drains over a transport the
    /// node has no view of; it returns the task's regular [`SwapTask::Result`]
    /// so the caller finishes the swap workflow into whatever next state the
    /// unwind enters.
    ///
    /// Specs that do not opt in never reach this path - the predicate gates it
    /// - so they return a visible error rather than driving an unwind.
    fn unwind(self, querier: QuerierWrapper<'_>, env: &Env) -> Self::Result;
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
    #[serde(default)]
    timeouts: CoinsNb,
    #[serde(default)]
    errors: CoinsNb,
    /// Strictly-monotonic per-emission correlation nonce; bumped on every
    /// emission and re-emission, matched against the callback. `#[serde(default)]`
    /// lets a node persisted before #636 load with a zero nonce, matching the
    /// zero an old, nonce-less in-flight packet decodes to.
    #[serde(default)]
    in_flight_nonce: u64,
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
    _finisher: PhantomData<HandlerT>,
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

    /// An acknowledgment overflowing the accumulated total comes from a
    /// counterparty in breach of the coin amount bounds and is absorbed
    /// like the other malformed payloads - an error would revert the
    /// controller's acknowledgment transaction and strand the workflow.
    fn apply_ack(
        self,
        coin_out: CoinDTO<SwapTask::OutG>,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        debug_assert!(self.invariant_held());

        match add_coins(self.total_out, &coin_out) {
            None => self.absorb(ABSORB_OUTPUT_OVERFLOW).into(),
            Some(total_out) => self.finish_or_open_next(total_out, querier, env),
        }
    }

    fn finish_or_open_next(
        self,
        total_out: CoinDTO<SwapTask::OutG>,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        match self.acks_left.checked_sub(1) {
            None => Error::MissingSwapLeg.into(),
            Some(0) => self.spec.into_output_task(FinishWithTotal {
                total_out,
                env: &env,
                querier,
                _finisher: PhantomData::<Self>,
            }),
            Some(acks_left) => Self::open_leg(
                self.spec,
                acks_left,
                total_out,
                querier,
                self.in_flight_nonce,
            )
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
        let node = self.with_bumped_nonce();
        node.schedule().and_then(|batch| {
            response::res_continue::<_, _, Self>(
                MessageResponse::messages_with_event(batch, node.emit_anomaly()),
                node,
            )
        })
    }

    /// Re-emit the in-flight leg verbatim after a transient failure, keeping
    /// the pinned floor and the accumulated progress intact. The fresh nonce
    /// supersedes the timed-out packet so a late callback for it is absorbed.
    fn reemit_in_flight(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        let node = self.with_bumped_nonce();
        let leg_label = node.spec.label();
        timeout::on_timeout_retry(node, leg_label, querier, env).into()
    }

    /// Re-emit the in-flight leg after a heal, signalling the recovery with
    /// the heal event. The node carries the floor freshly pinned by
    /// [`RemoteSwap::open_leg`] and the retry counters it reset to zero.
    pub(super) fn reemit_healed(self) -> ContinueResult<Self> {
        self.schedule().and_then(|batch| {
            response::res_continue::<_, _, Self>(
                MessageResponse::messages_with_event(batch, self.emit_heal()),
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
        self.in_flight_leg().and_then(|coin_in| {
            self.spec
                .schedule_swap(&coin_in, &self.in_flight_min_out, self.in_flight_nonce)
        })
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
    /// acknowledgment is validated against. Re-opening the SAME leg, as a
    /// heal does, re-quotes the floor and resets the retry counters to zero.
    /// `prev_nonce` is the nonce of the emission this open supersedes; the new
    /// leg takes `prev_nonce + 1`, keeping the per-node nonce strictly
    /// monotonic across legs and across a heal-from-terminal re-open.
    pub(super) fn open_leg(
        spec: SwapTask,
        acks_left: CoinsNb,
        total_out: CoinDTO<SwapTask::OutG>,
        querier: QuerierWrapper<'_>,
        prev_nonce: u64,
    ) -> Result<Self> {
        in_flight_leg(&spec, total_out.currency(), acks_left)
            .and_then(|coin_in| leg_min_out(&spec, coin_in, total_out.currency(), querier))
            .map(|min_out| {
                Self::internal_new(
                    spec,
                    acks_left,
                    total_out,
                    min_out,
                    0,
                    0,
                    prev_nonce.saturating_add(1),
                )
            })
    }

    fn with_incremented_errors(self) -> Self {
        Self {
            errors: self.errors.saturating_add(1),
            ..self
        }
    }

    fn with_incremented_timeouts(self) -> Self {
        Self {
            timeouts: self.timeouts.saturating_add(1),
            ..self
        }
    }

    /// Advance the in-flight nonce ahead of a same-leg re-emission, so the
    /// superseded packet's late callback no longer matches and is absorbed.
    fn with_bumped_nonce(self) -> Self {
        let ret = Self {
            in_flight_nonce: self.in_flight_nonce.saturating_add(1),
            ..self
        };
        debug_assert!(ret.invariant_held());
        ret
    }

    fn internal_new(
        spec: SwapTask,
        acks_left: CoinsNb,
        total_out: CoinDTO<SwapTask::OutG>,
        in_flight_min_out: CoinDTO<SwapTask::OutG>,
        timeouts: CoinsNb,
        errors: CoinsNb,
        in_flight_nonce: u64,
    ) -> Self {
        let ret = Self {
            spec,
            acks_left,
            total_out,
            in_flight_min_out,
            timeouts,
            errors,
            in_flight_nonce,
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

    #[cfg(test)]
    pub(crate) fn in_flight_nonce(&self) -> u64 {
        self.in_flight_nonce
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

impl<SwapTask, SEnum> RemoteSwap<SwapTask, SEnum>
where
    SwapTask: SwapTaskT + RemoteSwapClient,
    Self: Into<SEnum>,
    SlippageAnomaly<SwapTask, SEnum>: Into<SEnum>,
{
    /// Escalate a leg whose timeout retry budget is spent, per the spec's
    /// policy: park the opened legs at the slippage-anomaly terminal, or
    /// re-emit the opening swap verbatim. An explicit error never reaches
    /// here - it parks unconditionally (see `on_remote_error`).
    fn escalate(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        match self.spec.slippage_escalation() {
            SlippageEscalation::ReEmit => self.reemit_in_flight(querier, env),
            SlippageEscalation::Park => self.park(),
        }
    }

    fn park(self) -> HandlerResult<Self> {
        let terminal = SlippageAnomaly::new(
            self.spec,
            self.acks_left,
            self.total_out,
            self.in_flight_min_out,
            self.in_flight_nonce,
        );
        let emitter = terminal.emit_parked();
        response::res_continue::<_, _, Self>(
            MessageResponse::messages_with_event(Batch::default(), emitter),
            terminal,
        )
        .into()
    }
}

impl<SwapTask, SEnum> Handler for RemoteSwap<SwapTask, SEnum>
where
    SwapTask: SwapTaskT + RemoteSwapClient,
    Self: Into<SEnum>,
    SlippageAnomaly<SwapTask, SEnum>: Into<SEnum>,
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
        nonce: u64,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        if nonce != self.in_flight_nonce {
            return self.absorb(ABSORB_NONCE_MISMATCH).into();
        }
        match self.spec.decode_response(data.as_slice()) {
            Ok(coin_out) => self.deliver_ack(coin_out, querier, env),
            Err(Error::UnexpectedResponseVariant(_details)) => {
                self.absorb(ABSORB_UNEXPECTED_VARIANT).into()
            }
            Err(_undecodable) => self.absorb(ABSORB_UNDECODABLE).into(),
        }
    }

    /// An explicit error on the in-flight leg is an under-floor rejection. With
    /// a leg already acknowledged (`total_out > 0`) it parks at the
    /// slippage-anomaly terminal unconditionally, with no retry budget and
    /// without consulting the spec's timeout escalation policy; an operator
    /// `heal` re-drives the parked leg.
    ///
    /// With nothing acknowledged yet (`total_out == 0`) a spec that opts in via
    /// [`SwapTask::unwind_on_zero_acked`][crate::SwapTask::unwind_on_zero_acked]
    /// clean-unwinds instead - the inputs are still wholly on the remote
    /// account, so the spec drains them home rather than freezing them. A
    /// folded output-currency input leaves `total_out` non-zero at entry, so it
    /// parks like an acknowledged leg: only a genuinely zero-acked open unwinds.
    ///
    /// Anomalies are deliberately not routed through the spec's `on_anomaly`,
    /// whose `Retry` treatment rebuilds the node from the spec and would
    /// re-issue the already-acknowledged legs. Only the in-flight leg's
    /// accumulated progress carries into the terminal.
    fn on_remote_error(
        self,
        _response: ICAErrorResponse,
        nonce: u64,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        if nonce != self.in_flight_nonce {
            return self.absorb(ABSORB_NONCE_MISMATCH).into();
        }
        if self.total_out.is_zero() && self.spec.unwind_on_zero_acked() {
            response::res_finished(self.spec.unwind(querier, &env))
        } else {
            self.with_incremented_errors().park()
        }
    }

    /// A timeout re-emits the in-flight leg up to the spec's per-op retry
    /// budget and escalates past it - the opened legs park while the opening
    /// swap keeps re-emitting unbounded.
    fn on_remote_timeout(
        self,
        nonce: u64,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        if nonce != self.in_flight_nonce {
            return self.absorb(ABSORB_NONCE_MISMATCH).into();
        }
        let bumped = self.with_incremented_timeouts();
        if bumped.timeouts <= bumped.spec.timeout_retry_budget() {
            bumped.reemit_in_flight(querier, env)
        } else {
            bumped.escalate(querier, env)
        }
    }

    /// The only operator recovery on this transport - there is neither a
    /// sudo timeout nor a time alarm. The re-emission repeats the pinned
    /// `in_flight_min_out`, the exact promise of the original emission, and
    /// carries a fresh nonce (via [`RemoteSwap::with_bumped_nonce`]) so the
    /// original packet's late callback is absorbed as `nonce-mismatch` rather
    /// than mis-credited - the heal is idempotent regardless of operator
    /// timing (see the module doc).
    fn heal(
        self,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _info: &MessageInfo,
    ) -> HandlerResult<Self> {
        let node = self.with_bumped_nonce();
        node.schedule()
            .and_then(|batch| {
                response::res_continue::<_, _, Self>(
                    MessageResponse::messages_with_event(batch, node.emit_heal()),
                    node,
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
                        NO_PRIOR_NONCE,
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

pub(super) fn swappable_coins<SwapTask>(
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

fn add_coins<G>(total: CoinDTO<G>, more: &CoinDTO<G>) -> Option<CoinDTO<G>>
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
        type Outcome = Option<CoinDTO<G>>;

        fn on<C>(self, total: Coin<C>) -> Self::Outcome
        where
            C: CurrencyDef,
            C::Group: MemberOf<G> + MemberOf<G::TopG>,
        {
            total
                .checked_add(self.more.as_specific(C::dto()))
                .map(Into::into)
        }
    }

    debug_assert_eq!(total.currency(), more.currency());

    total.with_coin(AddOther { more })
}

#[cfg(test)]
pub(super) mod mock {
    use serde::{Deserialize, Serialize};

    use currency::test::{SuperGroup, SuperGroupTestC1};
    use finance::coin::{Amount, Coin, CoinDTO};
    use platform::batch::Batch;
    use sdk::cosmwasm_std::{Addr, Env, MessageInfo, QuerierWrapper};
    use timealarms::stub::TimeAlarmsRef;

    use crate::{
        Account, AnomalyTreatment, CoinsNb, ContractInRemoteSwap, SlippageCalculator,
        SlippageEscalation, SwapOutputTask, SwapTask, WithCalculator, WithOutputTask,
        error::{Error, Result},
    };

    use super::RemoteSwapClient;

    pub const LABEL: &str = "RemoteSwapMock";
    pub const CONTROLLER: &str = "controller";
    pub const WRONG_VARIANT_PAYLOAD: &[u8] = b"wrong-variant";
    pub const SLIPPAGE_PROTECTION_SENTINEL: CoinsNb = CoinsNb::MAX;
    /// The distinctive [`SwapTask::Result`] the mock's
    /// [`RemoteSwapClient::unwind`] returns, so a test can tell the unwind
    /// path apart from a regular swap finish.
    pub const UNWIND_SENTINEL: Amount = 777;

    const DEFAULT_FLOOR: Amount = 1;
    const DEFAULT_BUDGET: CoinsNb = 3;

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    enum Escalation {
        #[default]
        Park,
        ReEmit,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub struct MockSpec {
        coins: Vec<CoinDTO<SuperGroup>>,
        floor: Amount,
        budget: CoinsNb,
        #[serde(default)]
        escalation: Escalation,
        #[serde(default)]
        anomaly_resolution_authorised: bool,
        #[serde(default)]
        unwinds_on_zero_acked: bool,
    }

    #[derive(Serialize)]
    struct SwapRequest {
        coin_in: CoinDTO<SuperGroup>,
        min_out: CoinDTO<SuperGroup>,
    }

    struct FloorCalculator {
        floor: Amount,
    }

    impl MockSpec {
        pub fn new(coins: Vec<CoinDTO<SuperGroup>>) -> Self {
            Self {
                coins,
                floor: DEFAULT_FLOOR,
                budget: DEFAULT_BUDGET,
                escalation: Escalation::Park,
                anomaly_resolution_authorised: true,
                unwinds_on_zero_acked: false,
            }
        }

        pub fn set_floor(&mut self, floor: Amount) {
            self.floor = floor;
        }

        pub fn set_budget(&mut self, budget: CoinsNb) {
            self.budget = budget;
        }

        pub fn set_reemit(&mut self) {
            self.escalation = Escalation::ReEmit;
        }

        pub fn deny_anomaly_resolution(&mut self) {
            self.anomaly_resolution_authorised = false;
        }

        pub fn set_unwinds_on_zero_acked(&mut self) {
            self.unwinds_on_zero_acked = true;
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

        fn authz_anomaly_resolution(
            &self,
            _querier: QuerierWrapper<'_>,
            _info: &MessageInfo,
        ) -> Result<()> {
            if self.anomaly_resolution_authorised {
                Ok(())
            } else {
                Err(Error::Unauthorized(
                    access_control::error::Error::Unauthorized {},
                ))
            }
        }

        fn timeout_retry_budget(&self) -> CoinsNb {
            self.budget
        }

        fn slippage_escalation(&self) -> SlippageEscalation {
            match self.escalation {
                Escalation::Park => SlippageEscalation::Park,
                Escalation::ReEmit => SlippageEscalation::ReEmit,
            }
        }

        fn unwind_on_zero_acked(&self) -> bool {
            self.unwinds_on_zero_acked
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

        fn anomaly_response(
            self,
            _acks_left: CoinsNb,
            _now: finance::instant::Instant,
            _due_projection: finance::duration::Duration,
            _querier: QuerierWrapper<'_>,
        ) -> Self::StateResponse {
            SLIPPAGE_PROTECTION_SENTINEL
        }
    }

    impl RemoteSwapClient for MockSpec {
        // The mock ignores the nonce in the emitted batch — the production
        // controller puts it on the packet envelope, but the mock's
        // `SwapRequest` payload keeps the pre-#636 coin_in/min_out shape so the
        // existing leg/timeout response assertions stay byte-identical. The
        // nonce is exercised through the node's callback-matching, not the
        // emitted batch.
        fn schedule_swap(
            &self,
            coin_in: &CoinDTO<SuperGroup>,
            min_out: &CoinDTO<SuperGroup>,
            _nonce: u64,
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

        fn unwind(self, _querier: QuerierWrapper<'_>, _env: &Env) -> Self::Result {
            Coin::<SuperGroupTestC1>::new(UNWIND_SENTINEL).into()
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
        Addr, Binary, Env, MessageInfo, QuerierWrapper,
        testing::{self, MockQuerier},
    };

    use crate::{
        CoinsNb, Contract,
        error::Error,
        impl_::response::{Handler, Result as HandlerResult},
    };

    use super::mock::{self, MockSpec};

    type OutG = <MockSpec as crate::SwapTask>::OutG;
    type Node = super::RemoteSwap<MockSpec, TestWorkflow>;
    type Terminal = crate::impl_::SlippageAnomaly<MockSpec, TestWorkflow>;

    enum TestWorkflow {
        RemoteSwap(Node),
        SlippageAnomaly(Terminal),
    }

    impl From<Node> for TestWorkflow {
        fn from(node: Node) -> Self {
            Self::RemoteSwap(node)
        }
    }

    impl From<Terminal> for TestWorkflow {
        fn from(terminal: Terminal) -> Self {
            Self::SlippageAnomaly(terminal)
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
        let nonce = node.in_flight_nonce;
        let (response, node) = continued(node.on_remote_response(
            payload(&coin_out(30)),
            nonce,
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

        let nonce = node.in_flight_nonce;
        let (response, node) = continued(node.on_remote_response(
            payload(&coin_out(PINNED_FLOOR)),
            nonce,
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
        let nonce = node.in_flight_nonce;
        assert_eq!(
            coin_out(120),
            finished(node.on_remote_response(
                payload(&coin_out(40)),
                nonce,
                querier,
                testing::mock_env()
            ))
        );
    }

    #[test]
    fn timeout_reemits_only_the_in_flight_leg() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let env = testing::mock_env();

        let node = after_first_ack(querier);
        let nonce = node.in_flight_nonce;
        let (response, node) = continued(node.on_remote_timeout(nonce, querier, env.clone()));
        assert_eq!(timeout_response(&coin_in(70), &env), response);
        assert_node(1, &coin_out(80), &min_out(), &node);
        assert_eq!(1, node.timeouts);
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

        let nonce = node.in_flight_nonce;
        let (response, node) = continued(node.on_remote_response(
            payload(&coin_out(PINNED_FLOOR - 1)),
            nonce,
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

    /// An error parks immediately - no retry - freezing the in-flight leg's
    /// progress at the terminal and emitting the on-entry anomaly event.
    #[test]
    fn error_parks_preserving_progress() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let env = testing::mock_env();

        let node = after_first_ack(querier);
        let nonce = node.in_flight_nonce;
        let (response, terminal) = parked(node.on_remote_error(
            ICAErrorResponse::from(String::from("swap failed")),
            nonce,
            querier,
            env,
        ));
        assert_eq!(parked_response(), response);
        assert_terminal(1, &coin_out(80), &min_out(), &terminal);
    }

    /// #638 (TARGET): an explicit error parks UNCONDITIONALLY at the
    /// node level - even for a spec whose `slippage_escalation()` is
    /// `ReEmit` (the opening swap). An under-floor rejection is a slippage
    /// anomaly regardless of the spec's timeout-escalation policy, so the
    /// in-flight leg freezes at the terminal rather than re-emitting.
    ///
    /// FAILS against the current code (a `ReEmit` spec re-emits on error via
    /// `escalate`); the #638 change routes `on_remote_error` straight to
    /// `park`, ignoring the spec policy.
    #[test]
    fn error_parks_even_when_spec_reemits() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let env = testing::mock_env();

        let mut spec = spec3();
        spec.set_reemit();
        let (_response, node) = continued(Node::start(spec, &testing::mock_env(), querier));
        let nonce = node.in_flight_nonce;
        let (_response, node) = continued(node.on_remote_response(
            payload(&coin_out(30)),
            nonce,
            querier,
            testing::mock_env(),
        ));

        let nonce = node.in_flight_nonce;
        let (response, terminal) = parked(node.on_remote_error(
            ICAErrorResponse::from(String::from("opening swap under floor")),
            nonce,
            querier,
            env,
        ));
        assert_eq!(parked_response(), response);
        assert_terminal(1, &coin_out(80), &min_out(), &terminal);
    }

    /// #658: a hard error with nothing acknowledged yet (`total_out == 0`) on a
    /// spec that opts into the zero-acked unwind finishes through the spec's
    /// `unwind` hook instead of parking - the inputs are still wholly on the
    /// remote account, so the spec drains them home.
    #[test]
    fn zero_acked_error_unwinds_when_spec_opts_in() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let env = testing::mock_env();

        let mut spec = two_swap_legs();
        spec.set_unwinds_on_zero_acked();
        let (_response, node) = continued(Node::start(spec, &testing::mock_env(), querier));
        let nonce = node.in_flight_nonce;

        let unwound = finished(node.on_remote_error(
            ICAErrorResponse::from(String::from("downpayment leg under floor")),
            nonce,
            querier,
            env,
        ));
        assert_eq!(
            CoinDTO::<OutG>::from(Coin::<SuperGroupTestC1>::new(mock::UNWIND_SENTINEL)),
            unwound,
        );
    }

    /// #658: the opt-in alone is not enough - a hard error parks once a leg has
    /// acknowledged (`total_out > 0`), because some output is already committed
    /// on the remote side and cannot be unwound by draining the inputs.
    #[test]
    fn acked_leg_error_parks_even_when_spec_opts_into_unwind() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let env = testing::mock_env();

        let mut spec = two_swap_legs();
        spec.set_unwinds_on_zero_acked();
        let (_response, node) = continued(Node::start(spec, &testing::mock_env(), querier));
        let nonce = node.in_flight_nonce;
        let (_response, node) = continued(node.on_remote_response(
            payload(&coin_out(30)),
            nonce,
            querier,
            testing::mock_env(),
        ));

        let nonce = node.in_flight_nonce;
        let (response, terminal) = parked(node.on_remote_error(
            ICAErrorResponse::from(String::from("principal leg under floor")),
            nonce,
            querier,
            env,
        ));
        assert_eq!(parked_response(), response);
        assert_terminal(1, &coin_out(30), &min_out(), &terminal);
    }

    /// #658: a spec that does NOT opt in parks a zero-acked error - the default
    /// behaviour for close, liquidation and repay specs is unchanged.
    #[test]
    fn zero_acked_error_parks_when_spec_does_not_opt_in() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let env = testing::mock_env();

        let (_response, node) =
            continued(Node::start(two_swap_legs(), &testing::mock_env(), querier));
        let nonce = node.in_flight_nonce;

        let (response, terminal) = parked(node.on_remote_error(
            ICAErrorResponse::from(String::from("downpayment leg under floor")),
            nonce,
            querier,
            env,
        ));
        assert_eq!(parked_response(), response);
        assert_terminal(2, &coin_out(0), &min_out(), &terminal);
    }

    /// #638 (D2 contrast): with a `ReEmit` spec a TIMEOUT still follows the
    /// spec policy - it re-emits past the budget and never parks - while an
    /// error parks (see `error_parks_even_when_spec_reemits`). Error and
    /// timeout escalation are decoupled: the spec knob governs the
    /// timeout-past-budget path only.
    #[test]
    fn timeout_reemits_when_spec_reemits() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let mut spec = spec3();
        spec.set_reemit();
        let (_response, node) = continued(Node::start(spec, &testing::mock_env(), querier));
        let nonce = node.in_flight_nonce;
        let (_response, mut node) = continued(node.on_remote_response(
            payload(&coin_out(30)),
            nonce,
            querier,
            testing::mock_env(),
        ));

        // re-emit well past the budget - a `ReEmit` spec never parks on timeout
        let rounds = u16::from(mock_budget()) + 3;
        for _ in 0..rounds {
            let nonce = node.in_flight_nonce;
            let (_response, next) =
                continued(node.on_remote_timeout(nonce, querier, testing::mock_env()));
            node = next;
        }
        assert_node(1, &coin_out(80), &min_out(), &node);
    }

    #[test]
    fn garbage_payload_absorbed_without_state_change() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let (_response, node) = continued(Node::start(spec3(), &testing::mock_env(), querier));
        let nonce = node.in_flight_nonce;
        let (response, node) = continued(node.on_remote_response(
            Binary::from(b"garbage".as_slice()),
            nonce,
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
        let nonce = node.in_flight_nonce;
        let (response, node) = continued(node.on_remote_response(
            Binary::from(mock::WRONG_VARIANT_PAYLOAD),
            nonce,
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
        let nonce = node.in_flight_nonce;
        let (response, node) = continued(node.on_remote_response(
            payload(&coin_in(30)),
            nonce,
            querier,
            testing::mock_env(),
        ));
        assert_eq!(absorb_response("out-currency-mismatch"), response);
        assert_node(2, &coin_out(50), &min_out(), &node);
    }

    #[test]
    fn overflowing_ack_absorbed_without_state_change() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let (_response, node) = continued(Node::start(spec3(), &testing::mock_env(), querier));
        let nonce = node.in_flight_nonce;
        let (response, node) = continued(node.on_remote_response(
            payload(&coin_out(Amount::MAX)),
            nonce,
            querier,
            testing::mock_env(),
        ));
        assert_eq!(absorb_response("output-overflow"), response);
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

        let (response, node) = continued(node.heal(querier, testing::mock_env(), &healer()));
        assert_eq!(
            MessageResponse::messages_with_event(
                mock::swap_request(&coin_in(70), &min_out()).expect("a valid swap request"),
                Emitter::of_type(mock::LABEL).emit("heal", "re-emit"),
            ),
            response
        );
        assert_node(1, &coin_out(80), &min_out(), &node);
    }

    /// A leg parks once its timeout budget is spent: the budget-th timeout
    /// still re-emits, the next one parks at the terminal.
    #[test]
    fn timeout_budget_spent_parks_at_terminal() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let terminal = parked_terminal(querier);
        assert_terminal(1, &coin_out(80), &min_out(), &terminal);
    }

    /// The `ReEmit` timeout-escalation policy re-emits past the budget
    /// instead of parking - the opening swap keeps the legacy unbounded
    /// behaviour on TIMEOUT (an error, by contrast, parks unconditionally
    /// after #638 - see `error_parks_even_when_spec_reemits`).
    #[test]
    fn reemit_escalation_never_parks() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let mut spec = spec3();
        spec.set_reemit();
        let (_response, node) = continued(Node::start(spec, &testing::mock_env(), querier));
        let nonce = node.in_flight_nonce;
        let (_response, mut node) = continued(node.on_remote_response(
            payload(&coin_out(30)),
            nonce,
            querier,
            testing::mock_env(),
        ));

        let rounds = u16::from(mock_budget()) + 3;
        for _ in 0..rounds {
            let nonce = node.in_flight_nonce;
            let (_response, next) =
                continued(node.on_remote_timeout(nonce, querier, testing::mock_env()));
            node = next;
        }
        assert_node(1, &coin_out(80), &min_out(), &node);
    }

    /// A late acknowledgment of the original packet reaching the parked
    /// terminal is absorbed without leaving the terminal.
    #[test]
    fn terminal_absorbs_late_ok_ack() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let terminal = parked_terminal(querier);
        let nonce = terminal.in_flight_nonce();
        let (response, terminal) = terminal_continued(terminal.on_remote_response(
            payload(&coin_out(40)),
            nonce,
            querier,
            testing::mock_env(),
        ));
        assert_eq!(parked_absorb_response("parked-response"), response);
        assert_terminal(1, &coin_out(80), &min_out(), &terminal);
    }

    /// A late error and a late timeout reaching the parked terminal are both
    /// absorbed without advancing or leaving it.
    #[test]
    fn terminal_absorbs_late_err_and_timeout() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let terminal = parked_terminal(querier);
        let nonce = terminal.in_flight_nonce();
        let (response, terminal) = terminal_continued(terminal.on_remote_error(
            ICAErrorResponse::from(String::from("late error")),
            nonce,
            querier,
            testing::mock_env(),
        ));
        assert_eq!(parked_absorb_response("parked-error"), response);

        let nonce = terminal.in_flight_nonce();
        let (response, terminal) =
            terminal_continued(terminal.on_remote_timeout(nonce, querier, testing::mock_env()));
        assert_eq!(parked_absorb_response("parked-timeout"), response);
        assert_terminal(1, &coin_out(80), &min_out(), &terminal);
    }

    /// A late callback must not mutate the parked progress: the frozen leg,
    /// total, and floor are byte-identical before and after.
    #[test]
    fn terminal_late_callback_does_not_mutate_counters() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let terminal = parked_terminal(querier);
        let before = sdk::cosmwasm_std::to_json_vec(&terminal).expect("a serializable terminal");

        let nonce = terminal.in_flight_nonce();
        let (_response, terminal) =
            terminal_continued(terminal.on_remote_timeout(nonce, querier, testing::mock_env()));
        let after = sdk::cosmwasm_std::to_json_vec(&terminal).expect("a serializable terminal");
        assert_eq!(before, after);
    }

    /// A heal re-quotes the SAME in-flight leg with a fresh floor, resets the
    /// retry counters, re-emits, and leaves the terminal back into the live
    /// swap node.
    #[test]
    fn terminal_heal_requotes_and_resets() {
        const RAISED_FLOOR: Amount = 100;
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let mut terminal = parked_terminal(querier);
        terminal.spec_mut().set_floor(RAISED_FLOOR);

        let (response, node) =
            terminal_healed(terminal.heal(querier, testing::mock_env(), &healer()));
        assert_eq!(
            MessageResponse::messages_with_event(
                mock::swap_request(&coin_in(70), &coin_out(RAISED_FLOOR))
                    .expect("a valid swap request"),
                Emitter::of_type(mock::LABEL).emit("heal", "re-emit"),
            ),
            response
        );
        assert_node(1, &coin_out(80), &coin_out(RAISED_FLOOR), &node);
        assert_eq!(0, node.timeouts);
        assert_eq!(0, node.errors);
    }

    /// The original packet reaching the parked terminal is absorbed, so the
    /// operator heal re-emits a single in-flight leg whose acknowledgment
    /// credits the total exactly once - the at-most-once, single-leg property
    /// that keeps a stale packet plus the heal re-emission from double-crediting.
    #[test]
    fn double_heal_absorbs_second_packet() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        // the original packet lands while parked and is absorbed - no credit
        let terminal = parked_terminal(querier);
        let nonce = terminal.in_flight_nonce();
        let (_response, terminal) = terminal_continued(terminal.on_remote_response(
            payload(&coin_out(40)),
            nonce,
            querier,
            testing::mock_env(),
        ));
        assert_terminal(1, &coin_out(80), &min_out(), &terminal);

        // the heal re-emits the SAME in-flight leg without advancing it
        let (_response, node) =
            terminal_healed(terminal.heal(querier, testing::mock_env(), &healer()));
        assert_node(1, &coin_out(80), &min_out(), &node);

        // exactly one acknowledgment of the re-emitted leg finishes the
        // workflow - the absorbed first packet did not also credit
        let nonce = node.in_flight_nonce;
        assert_eq!(
            coin_out(120),
            finished(node.on_remote_response(
                payload(&coin_out(40)),
                nonce,
                querier,
                testing::mock_env()
            ))
        );
    }

    /// An unauthorised operator `heal` of a parked leg is rejected before any
    /// re-quote - the leg stays frozen at the terminal.
    #[test]
    fn heal_rejects_unauthorised_operator() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let mut terminal = parked_terminal(querier);
        terminal.spec_mut().deny_anomaly_resolution();

        assert!(matches!(
            terminal.heal(querier, testing::mock_env(), &healer()),
            HandlerResult::Continue(Err(Error::Unauthorized(_)))
        ));
    }

    /// A live swap leg drops a price alarm silently, while the parked
    /// terminal reports a dropped-alarm event for monitoring.
    #[test]
    fn price_alarm_dropped_only_when_parked() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let live = after_first_ack(querier);
        assert!(live.price_alarm_dropped().is_none());

        let terminal = parked_terminal(querier);
        assert_eq!(
            Some(Emitter::of_type(mock::LABEL).emit("anomaly", "price-alarm-dropped")),
            terminal.price_alarm_dropped(),
        );
    }

    /// The parked terminal survives a serde round-trip and keeps absorbing
    /// late callbacks afterwards.
    #[test]
    fn terminal_serde_round_trips() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let terminal = parked_terminal(querier);
        let restored: Terminal = sdk::cosmwasm_std::to_json_vec(&terminal)
            .and_then(sdk::cosmwasm_std::from_json)
            .expect("the terminal should round-trip");
        assert_terminal(1, &coin_out(80), &min_out(), &restored);
    }

    #[test]
    fn state_serde_round_trips() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let mut node = after_first_ack(querier);
        node.timeouts = 4;
        node.errors = 2;
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
        assert_eq!(node.timeouts, restored.timeouts);
        assert_eq!(node.errors, restored.errors);
    }

    /// A lease persisted before #655 carries no `timeouts`/`errors` keys;
    /// `#[serde(default)]` must let it load with both counters cleared so
    /// the new code-id never bricks an in-flight lease.
    #[test]
    fn legacy_remote_swap_without_counters_deserializes_to_zero() {
        let legacy_remote_swap = br#"{"spec":{"coins":[{"amount":"100","ticker":"ticker#2"},{"amount":"50","ticker":"ticker#1"},{"amount":"70","ticker":"ticker#2"}],"floor":1,"budget":3},"acks_left":1,"total_out":{"amount":"80","ticker":"ticker#1"},"in_flight_min_out":{"amount":"1","ticker":"ticker#1"}}"#;

        let restored: Node = sdk::cosmwasm_std::from_json(legacy_remote_swap.as_slice())
            .expect("the pre-#655 state should deserialize");
        assert_eq!(0, restored.timeouts);
        assert_eq!(0, restored.errors);
    }

    #[test]
    fn counters_round_trip() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let mut node = after_first_ack(querier);
        node.timeouts = 3;
        node.errors = 1;
        let restored: Node = sdk::cosmwasm_std::to_json_vec(&node)
            .and_then(sdk::cosmwasm_std::from_json)
            .expect("the counters should round-trip");
        assert_eq!(3, restored.timeouts);
        assert_eq!(1, restored.errors);
    }

    #[test]
    fn spec_reports_its_timeout_retry_budget() {
        use crate::SwapTask;

        let mut spec = spec3();
        spec.set_budget(7);
        assert_eq!(7, spec.timeout_retry_budget());
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

    // -----------------------------------------------------------------------
    // #636 — per-emission nonce: callbacks of the CURRENT in-flight packet
    // (nonce == in_flight_nonce) are handled normally; a superseded packet
    // (smaller nonce) is absorbed with `nonce-mismatch`, leaving state intact.
    // -----------------------------------------------------------------------

    /// AC (#636): an ack carrying the node's current `in_flight_nonce` is the
    /// acknowledgment of the in-flight packet — it is accepted and advances the
    /// sequence exactly as the nonce-less ack did before the field existed.
    #[test]
    fn ack_with_matching_nonce_accepted() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let (_response, node) = continued(Node::start(spec3(), &testing::mock_env(), querier));
        let nonce = node.in_flight_nonce;
        let (response, node) = continued(node.on_remote_response(
            payload(&coin_out(30)),
            nonce,
            querier,
            testing::mock_env(),
        ));
        assert_eq!(
            leg_response(&coin_in(70), &min_out(), &coin_out(80)),
            response
        );
        assert_node(1, &coin_out(80), &min_out(), &node);
    }

    /// AC (#636): an ack carrying a smaller-than-current nonce is the
    /// acknowledgment of a superseded packet — it is absorbed with
    /// `nonce-mismatch` and leaves the node byte-identical (no advance, no
    /// credit).
    #[test]
    fn ack_with_stale_nonce_absorbed() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        // advance once so the in-flight nonce is strictly greater than the
        // first leg's nonce, giving a genuine stale value to replay
        let node = after_first_ack(querier);
        let stale = node.in_flight_nonce - 1;
        let before = sdk::cosmwasm_std::to_json_vec(&node).expect("a serializable node");

        let (response, node) = continued(node.on_remote_response(
            payload(&coin_out(40)),
            stale,
            querier,
            testing::mock_env(),
        ));
        assert_eq!(absorb_response("nonce-mismatch"), response);
        assert_node(1, &coin_out(80), &min_out(), &node);
        let after = sdk::cosmwasm_std::to_json_vec(&node).expect("a serializable node");
        assert_eq!(before, after);
    }

    /// AC (#636): a timeout of the current packet is handled (the in-flight leg
    /// re-emits, the timeout counter advances), and the re-emission carries a
    /// strictly greater nonce; the now-superseded original nonce is absorbed.
    #[test]
    fn timeout_with_matching_nonce_retries_then_bumps() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let env = testing::mock_env();

        let node = after_first_ack(querier);
        let original_nonce = node.in_flight_nonce;

        let (_response, node) =
            continued(node.on_remote_timeout(original_nonce, querier, env.clone()));
        assert_eq!(1, node.timeouts);
        assert!(
            original_nonce < node.in_flight_nonce,
            "the re-emission must carry a strictly greater nonce"
        );

        // the original packet's late ack now carries a stale nonce → absorbed
        let (response, node) = continued(node.on_remote_response(
            payload(&coin_out(40)),
            original_nonce,
            querier,
            testing::mock_env(),
        ));
        assert_eq!(absorb_response("nonce-mismatch"), response);
        assert_node(1, &coin_out(80), &min_out(), &node);
    }

    /// AC (#636): a superseded error callback (smaller nonce) is absorbed with
    /// `nonce-mismatch` and never escalates — the live leg is untouched.
    #[test]
    fn error_with_stale_nonce_absorbed() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let node = after_first_ack(querier);
        let stale = node.in_flight_nonce - 1;

        let (response, node) = continued(node.on_remote_error(
            ICAErrorResponse::from(String::from("superseded error")),
            stale,
            querier,
            testing::mock_env(),
        ));
        assert_eq!(absorb_response("nonce-mismatch"), response);
        assert_node(1, &coin_out(80), &min_out(), &node);
    }

    /// AC (#636) — core race: a heal re-emits with a strictly greater nonce, so
    /// the pre-heal original packet's late ack (old nonce) is absorbed while the
    /// healed re-emission's ack (new nonce) is accepted. No double-credit.
    #[test]
    fn heal_bumps_nonce() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let node = after_first_ack(querier);
        let original_nonce = node.in_flight_nonce;

        let (_response, node) = continued(node.heal(querier, testing::mock_env(), &healer()));
        let healed_nonce = node.in_flight_nonce;
        assert!(
            original_nonce < healed_nonce,
            "heal must re-emit with a strictly greater nonce"
        );

        // the original packet's ack (old nonce) is absorbed - no credit
        let (response, node) = continued(node.on_remote_response(
            payload(&coin_out(40)),
            original_nonce,
            querier,
            testing::mock_env(),
        ));
        assert_eq!(absorb_response("nonce-mismatch"), response);
        assert_node(1, &coin_out(80), &min_out(), &node);

        // the healed re-emission's ack (new nonce) is accepted and finishes
        assert_eq!(
            coin_out(120),
            finished(node.on_remote_response(
                payload(&coin_out(40)),
                healed_nonce,
                querier,
                testing::mock_env(),
            ))
        );
    }

    /// AC (#636): a heal from the parked SlippageAnomaly terminal bumps the
    /// nonce above the value persisted at park time — it is never reset to 0,
    /// so a late ack of the original parked packet is still recognised as stale.
    #[test]
    fn heal_from_parked_terminal_bumps_nonce() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let terminal = parked_terminal(querier);
        let parked_nonce = terminal.in_flight_nonce();

        let (_response, node) =
            terminal_healed(terminal.heal(querier, testing::mock_env(), &healer()));
        assert!(
            parked_nonce < node.in_flight_nonce,
            "heal from the terminal must bump above the persisted nonce, not reset"
        );

        // the original parked packet's ack carries the now-stale nonce → absorbed
        let (response, node) = continued(node.on_remote_response(
            payload(&coin_out(40)),
            parked_nonce,
            querier,
            testing::mock_env(),
        ));
        assert_eq!(absorb_response("nonce-mismatch"), response);
        assert_node(1, &coin_out(80), &min_out(), &node);
    }

    /// AC (#636): a lease persisted before #636 carries no `in_flight_nonce`
    /// key; `#[serde(default)]` must load it as 0 so the new code-id never
    /// bricks an in-flight lease.
    #[test]
    fn legacy_remote_swap_without_nonce_deserializes_to_zero() {
        let legacy_remote_swap = br#"{"spec":{"coins":[{"amount":"100","ticker":"ticker#2"},{"amount":"50","ticker":"ticker#1"},{"amount":"70","ticker":"ticker#2"}],"floor":1,"budget":3},"acks_left":1,"total_out":{"amount":"80","ticker":"ticker#1"},"in_flight_min_out":{"amount":"1","ticker":"ticker#1"},"timeouts":0,"errors":0}"#;

        let restored: Node = sdk::cosmwasm_std::from_json(legacy_remote_swap.as_slice())
            .expect("the pre-#636 state should deserialize");
        assert_eq!(0, restored.in_flight_nonce);
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

    fn assert_terminal(
        expected_acks: CoinsNb,
        expected_total: &CoinDTO<OutG>,
        expected_pinned: &CoinDTO<OutG>,
        terminal: &Terminal,
    ) {
        assert_eq!(expected_acks, terminal.acks_left());
        assert_eq!(*expected_total, *terminal.total_out());
        assert_eq!(*expected_pinned, *terminal.in_flight_min_out());
    }

    fn parked_response() -> MessageResponse {
        MessageResponse::messages_with_event(
            Batch::default(),
            Emitter::of_type(mock::LABEL).emit("anomaly", "slippage-anomaly-parked"),
        )
    }

    fn parked_absorb_response(reason: &str) -> MessageResponse {
        MessageResponse::messages_with_event(
            Batch::default(),
            Emitter::of_type(mock::LABEL).emit("absorbed", reason),
        )
    }

    fn continued(res: HandlerResult<Node>) -> (MessageResponse, Node) {
        match res {
            HandlerResult::Continue(Ok(resp)) => match resp.next_state {
                TestWorkflow::RemoteSwap(node) => (resp.response, node),
                TestWorkflow::SlippageAnomaly(_terminal) => {
                    panic!("expected the live swap node, got the parked terminal")
                }
            },
            HandlerResult::Continue(Err(err)) => panic!("expected a continuation, got {err}"),
            HandlerResult::Finished(_total) => panic!("expected a continuation, got a finish"),
        }
    }

    fn parked(res: HandlerResult<Node>) -> (MessageResponse, Terminal) {
        match res {
            HandlerResult::Continue(Ok(resp)) => match resp.next_state {
                TestWorkflow::SlippageAnomaly(terminal) => (resp.response, terminal),
                TestWorkflow::RemoteSwap(_node) => {
                    panic!("expected the parked terminal, got the live swap node")
                }
            },
            HandlerResult::Continue(Err(err)) => panic!("expected the terminal, got {err}"),
            HandlerResult::Finished(_total) => panic!("expected the terminal, got a finish"),
        }
    }

    fn terminal_continued(res: HandlerResult<Terminal>) -> (MessageResponse, Terminal) {
        match res {
            HandlerResult::Continue(Ok(resp)) => match resp.next_state {
                TestWorkflow::SlippageAnomaly(terminal) => (resp.response, terminal),
                TestWorkflow::RemoteSwap(_node) => {
                    panic!("expected the terminal to stay parked, got the live swap node")
                }
            },
            HandlerResult::Continue(Err(err)) => panic!("expected the terminal, got {err}"),
            HandlerResult::Finished(_total) => panic!("expected the terminal, got a finish"),
        }
    }

    fn terminal_healed(res: HandlerResult<Terminal>) -> (MessageResponse, Node) {
        match res {
            HandlerResult::Continue(Ok(resp)) => match resp.next_state {
                TestWorkflow::RemoteSwap(node) => (resp.response, node),
                TestWorkflow::SlippageAnomaly(_terminal) => {
                    panic!("expected the healed swap node, got the parked terminal")
                }
            },
            HandlerResult::Continue(Err(err)) => panic!("expected the healed node, got {err}"),
            HandlerResult::Finished(_total) => panic!("expected the healed node, got a finish"),
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
        let nonce = node.in_flight_nonce;
        let (_response, node) = continued(node.on_remote_response(
            payload(&coin_out(30)),
            nonce,
            querier,
            testing::mock_env(),
        ));
        node
    }

    /// Drive the in-flight leg into the parked terminal by exhausting the
    /// timeout budget: `budget` re-emits keep retrying, the next one parks.
    fn parked_terminal(querier: QuerierWrapper<'_>) -> Terminal {
        let node = after_first_ack(querier);
        let budget = mock_budget();
        let node = (0..budget).fold(node, |node, _round| {
            let nonce = node.in_flight_nonce;
            continued(node.on_remote_timeout(nonce, querier, testing::mock_env())).1
        });
        let nonce = node.in_flight_nonce;
        parked(node.on_remote_timeout(nonce, querier, testing::mock_env())).1
    }

    fn mock_budget() -> CoinsNb {
        use crate::SwapTask;

        spec3().timeout_retry_budget()
    }

    fn spec3() -> MockSpec {
        MockSpec::new(vec![coin_in(100), coin_out(50), coin_in(70)])
    }

    /// Two swap legs and no output-currency coin to fold, so the node starts
    /// with `total_out == 0` - the zero-acked precondition the unwind path
    /// gates on.
    fn two_swap_legs() -> MockSpec {
        MockSpec::new(vec![coin_in(100), coin_in(70)])
    }

    fn healer() -> MessageInfo {
        MessageInfo {
            sender: Addr::unchecked(mock::CONTROLLER),
            funds: vec![],
        }
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
