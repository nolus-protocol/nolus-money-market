//! The parked terminal a remote swap leg enters on a slippage anomaly
//!
//! An opened lease whose in-flight leg keeps failing - an `OperationErr`, or
//! `OperationTimeout`s past the per-op retry budget - parks here instead of
//! retrying forever. The node carries exactly what a [`Handler::heal`] needs
//! to resume the SAME in-flight leg: the swap `spec`, the `acks_left`
//! countdown identifying the leg, the accumulated `total_out`, the floor
//! `in_flight_min_out` pinned when the leg was last opened, and the
//! `in_flight_nonce` the parked packet carried.
//!
//! While parked the lease answers state queries with
//! `SlippageProtectionActivated` and absorbs any late acknowledgment of the
//! original packet - reverting would strand the relayer. The operator-only
//! heal re-quotes the leg against a fresh oracle floor, bumps the nonce above
//! the parked value, and transitions back to the live [`RemoteSwap`] sequence;
//! the bumped nonce makes the heal idempotent - the original parked packet's
//! late ack is absorbed as stale rather than mis-credited - so it is safe
//! regardless of operator timing, mirroring the [`RemoteSwap`] heal contract.

use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
};

use serde::{Deserialize, Serialize};

use finance::{coin::CoinDTO, duration::Duration, instant::Instant};
use platform::{
    batch::{Batch, Emit, Emitter},
    ica::ErrorResponse as ICAErrorResponse,
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Binary, Env, MessageInfo, QuerierWrapper};

use crate::{
    CoinsNb, Contract, ContractInRemoteSwap, SwapTask as SwapTaskT, TimeAlarm,
    error::{Error, Result},
    impl_::{
        RemoteSwap, RemoteSwapClient,
        response::{self, Handler, Result as HandlerResult},
    },
};

#[cfg(feature = "migration")]
use super::migration::{InspectSpec, MigrateSpec};

const EVENT_KEY_ANOMALY: &str = "anomaly";
const EVENT_KEY_ABSORBED: &str = "absorbed";
const ANOMALY_PARKED: &str = "slippage-anomaly-parked";
const ANOMALY_PRICE_ALARM_DROPPED: &str = "price-alarm-dropped";
const ABSORB_PARKED_RESPONSE: &str = "parked-response";
const ABSORB_PARKED_ERROR: &str = "parked-error";
const ABSORB_PARKED_TIMEOUT: &str = "parked-timeout";

/// A remote swap leg parked on a slippage anomaly
///
/// The restore path runs through [`SlippageAnomalyRaw`] so a corrupted or
/// malformed stored terminal is rejected before it reaches the public
/// state/heal path: the constructor's `debug_assert!` is compiled out in
/// release and would not guard a `Deserialize`-restored instance anyway.
#[derive(Serialize, Deserialize)]
#[serde(
    bound(
        serialize = "SwapTask: Serialize",
        deserialize = "SwapTask: Deserialize<'de> + SwapTaskT"
    ),
    rename_all = "snake_case",
    try_from = "SlippageAnomalyRaw<SwapTask, SEnum>"
)]
pub struct SlippageAnomaly<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
{
    spec: SwapTask,
    acks_left: CoinsNb,
    total_out: CoinDTO<SwapTask::OutG>,
    in_flight_min_out: CoinDTO<SwapTask::OutG>,
    /// The nonce the parked leg was last emitted with (#636). A heal bumps
    /// above this value, never resetting, so a late ack of the original parked
    /// packet stays recognisable as stale.
    in_flight_nonce: u64,
    #[serde(skip)]
    _variant_set: PhantomData<SEnum>,
}

/// The wire mirror restored terminals deserialize through. [`TryFrom`]
/// re-runs the constructor's invariant before handing back a typed terminal,
/// so a corrupted store yields a typed [`Error`] rather than an unchecked
/// state that the `debug_assert!`-only constructor would not catch in release.
#[derive(Deserialize)]
#[serde(
    bound(deserialize = "SwapTask: Deserialize<'de> + SwapTaskT"),
    deny_unknown_fields,
    rename_all = "snake_case"
)]
struct SlippageAnomalyRaw<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
{
    spec: SwapTask,
    acks_left: CoinsNb,
    total_out: CoinDTO<SwapTask::OutG>,
    in_flight_min_out: CoinDTO<SwapTask::OutG>,
    /// `#[serde(default)]` lets a terminal parked before #636 restore with a
    /// zero nonce, matching the zero its old, nonce-less in-flight packet
    /// decodes to.
    #[serde(default)]
    in_flight_nonce: u64,
    #[serde(skip)]
    _variant_set: PhantomData<SEnum>,
}

impl<SwapTask, SEnum> TryFrom<SlippageAnomalyRaw<SwapTask, SEnum>>
    for SlippageAnomaly<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
{
    type Error = Error;

    fn try_from(raw: SlippageAnomalyRaw<SwapTask, SEnum>) -> Result<Self> {
        let ret = Self {
            spec: raw.spec,
            acks_left: raw.acks_left,
            total_out: raw.total_out,
            in_flight_min_out: raw.in_flight_min_out,
            in_flight_nonce: raw.in_flight_nonce,
            _variant_set: PhantomData,
        };
        if ret.invariant_held() {
            Ok(ret)
        } else {
            Err(Error::SlippageAnomalyInvariantViolated)
        }
    }
}

impl<SwapTask, SEnum> SlippageAnomaly<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
{
    pub(super) fn new(
        spec: SwapTask,
        acks_left: CoinsNb,
        total_out: CoinDTO<SwapTask::OutG>,
        in_flight_min_out: CoinDTO<SwapTask::OutG>,
        in_flight_nonce: u64,
    ) -> Self {
        let ret = Self {
            spec,
            acks_left,
            total_out,
            in_flight_min_out,
            in_flight_nonce,
            _variant_set: PhantomData,
        };
        debug_assert!(ret.invariant_held());
        ret
    }

    /// The frozen leg the terminal carries is a snapshot of the live
    /// [`RemoteSwap`] leg it parked from, so the same invariant holds: an
    /// in-flight leg the `acks_left` countdown points at within the spec's
    /// swappable coins, with the pinned floor denominated in the accumulated
    /// total's currency.
    fn invariant_held(&self) -> bool {
        0 < self.acks_left
            && usize::from(self.acks_left) <= self.legs_nb()
            && self.in_flight_min_out.currency() == self.total_out.currency()
    }

    fn legs_nb(&self) -> usize {
        super::remote_swap::swappable_coins(&self.spec, self.total_out.currency()).count()
    }

    /// The on-entry event a leg emits as it parks at the terminal.
    pub(super) fn emit_parked(&self) -> Emitter {
        Emitter::of_type(self.spec.label()).emit(EVENT_KEY_ANOMALY, ANOMALY_PARKED)
    }

    fn emit_absorbed(&self, reason: &str) -> Emitter {
        Emitter::of_type(self.spec.label()).emit(EVENT_KEY_ABSORBED, reason)
    }

    /// The dropped-price-alarm event a parked lease emits so monitoring sees
    /// the price move it would normally have acted on was ignored.
    fn emit_price_alarm_dropped(&self) -> Emitter {
        Emitter::of_type(self.spec.label()).emit(EVENT_KEY_ANOMALY, ANOMALY_PRICE_ALARM_DROPPED)
    }

    #[cfg(test)]
    pub(super) fn acks_left(&self) -> CoinsNb {
        self.acks_left
    }

    #[cfg(test)]
    pub(super) fn total_out(&self) -> &CoinDTO<SwapTask::OutG> {
        &self.total_out
    }

    #[cfg(test)]
    pub(super) fn in_flight_min_out(&self) -> &CoinDTO<SwapTask::OutG> {
        &self.in_flight_min_out
    }

    // #636: the persisted nonce the parked leg last emitted with. A heal must
    // bump above this value, never reset, so a late ack of the original parked
    // packet is still recognised as stale.
    #[cfg(test)]
    pub(super) fn in_flight_nonce(&self) -> u64 {
        self.in_flight_nonce
    }

    #[cfg(test)]
    pub(super) fn spec_mut(&mut self) -> &mut SwapTask {
        &mut self.spec
    }
}

impl<SwapTask, SEnum> SlippageAnomaly<SwapTask, SEnum>
where
    SwapTask: SwapTaskT + RemoteSwapClient,
    Self: Handler<Response = SEnum, SwapResult = SwapTask::Result> + Into<SEnum>,
{
    fn absorb(self, reason: &str) -> HandlerResult<Self> {
        let emitter = self.emit_absorbed(reason);
        response::res_continue::<_, _, Self>(
            MessageResponse::messages_with_event(Batch::default(), emitter),
            self,
        )
        .into()
    }
}

impl<SwapTask, SEnum> Handler for SlippageAnomaly<SwapTask, SEnum>
where
    SwapTask: SwapTaskT + RemoteSwapClient,
    Self: Into<SEnum>,
    SEnum: From<RemoteSwap<SwapTask, SEnum>>,
    RemoteSwap<SwapTask, SEnum>:
        Handler<Response = SEnum, SwapResult = SwapTask::Result> + Into<SEnum>,
{
    type Response = SEnum;
    type SwapResult = SwapTask::Result;

    fn authz_remote_callback(&self, querier: QuerierWrapper<'_>, info: &MessageInfo) -> Result<()> {
        self.spec.authz_remote_callback(querier, info)
    }

    /// A late acknowledgment of the original packet is absorbed without
    /// advancing or mutating the parked progress - reverting would strand
    /// the relayer, and the frozen leg stays put for the operator heal.
    fn on_remote_response(
        self,
        _data: Binary,
        _nonce: u64,
        _querier: QuerierWrapper<'_>,
        _env: Env,
    ) -> HandlerResult<Self> {
        self.absorb(ABSORB_PARKED_RESPONSE)
    }

    fn on_remote_error(
        self,
        _response: ICAErrorResponse,
        _nonce: u64,
        _querier: QuerierWrapper<'_>,
        _env: Env,
    ) -> HandlerResult<Self> {
        self.absorb(ABSORB_PARKED_ERROR)
    }

    fn on_remote_timeout(
        self,
        _nonce: u64,
        _querier: QuerierWrapper<'_>,
        _env: Env,
    ) -> HandlerResult<Self> {
        self.absorb(ABSORB_PARKED_TIMEOUT)
    }

    fn price_alarm_dropped(&self) -> Option<Emitter> {
        Some(self.emit_price_alarm_dropped())
    }

    /// Re-quote the SAME in-flight leg against a fresh oracle floor and
    /// transition back to the live swap sequence with the retry counters
    /// reset. `acks_left` is unchanged - the leg is re-opened, never
    /// advanced - so the heal re-emission promises a floor freshly pinned by
    /// `RemoteSwap::open_leg`. The persisted `in_flight_nonce` seeds the
    /// re-open so the healed emission carries a strictly greater nonce and the
    /// original parked packet's late ack is absorbed. Operator-only: an
    /// unauthorised caller is rejected before any re-quote, leaving the leg
    /// parked.
    fn heal(
        self,
        querier: QuerierWrapper<'_>,
        _env: Env,
        info: &MessageInfo,
    ) -> HandlerResult<Self> {
        match self.spec.authz_anomaly_resolution(querier, info) {
            Ok(()) => RemoteSwap::<SwapTask, SEnum>::open_leg(
                self.spec,
                self.acks_left,
                self.total_out,
                querier,
                self.in_flight_nonce,
            )
            .and_then(RemoteSwap::reemit_healed)
            .into(),
            Err(err) => HandlerResult::from(err),
        }
    }
}

impl<SwapTask, SEnum> Contract for SlippageAnomaly<SwapTask, SEnum>
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
            .anomaly_response(self.acks_left, now, due_projection, querier)
    }
}

impl<SwapTask, SEnum> Display for SlippageAnomaly<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str("SlippageAnomaly at ")
            .and_then(|()| f.write_str(&Into::<String>::into(self.spec.label())))
    }
}

impl<SwapTask, SEnum> TimeAlarm for SlippageAnomaly<SwapTask, SEnum>
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
    for SlippageAnomaly<SwapTask, SEnum>
where
    Self: Sized,
    SwapTask: SwapTaskT,
    SwapTaskNew: SwapTaskT<OutG = SwapTask::OutG>,
    SlippageAnomaly<SwapTaskNew, SEnumNew>: Into<SEnumNew>,
{
    type Out = SlippageAnomaly<SwapTaskNew, SEnumNew>;

    fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
    where
        MigrateFn: FnOnce(SwapTask) -> SwapTaskNew,
    {
        SlippageAnomaly::new(
            migrate_fn(self.spec),
            self.acks_left,
            self.total_out,
            self.in_flight_min_out,
            self.in_flight_nonce,
        )
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, R, SEnum> InspectSpec<SwapTask, R> for SlippageAnomaly<SwapTask, SEnum>
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
