//! The parked terminal a remote swap leg enters on a slippage anomaly
//!
//! An opened lease whose in-flight leg keeps failing - an `OperationErr`, or
//! `OperationTimeout`s past the per-op retry budget - parks here instead of
//! retrying forever. The node carries exactly what a [`Handler::heal`] needs
//! to resume the SAME in-flight leg: the swap `spec`, the `acks_left`
//! countdown identifying the leg, the accumulated `total_out`, and the floor
//! `in_flight_min_out` pinned when the leg was last opened.
//!
//! While parked the lease answers state queries with
//! `SlippageProtectionActivated` and absorbs any late acknowledgment of the
//! original packet - reverting would strand the relayer. The at-most-once
//! transport makes such a late callback a defensive case rather than an
//! expected one. The operator-only heal re-quotes the leg against a fresh
//! oracle floor and transitions back to the live [`RemoteSwap`] sequence;
//! operators must invoke it only once the original in-flight operation is
//! known to be unresolvable, mirroring the [`RemoteSwap`] heal contract.

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
    error::Result,
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
#[derive(Serialize, Deserialize)]
#[serde(
    bound(
        serialize = "SwapTask: Serialize",
        deserialize = "SwapTask: Deserialize<'de> + SwapTaskT"
    ),
    deny_unknown_fields,
    rename_all = "snake_case"
)]
pub struct SlippageAnomaly<SwapTask, SEnum>
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

impl<SwapTask, SEnum> SlippageAnomaly<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
{
    pub(super) fn new(
        spec: SwapTask,
        acks_left: CoinsNb,
        total_out: CoinDTO<SwapTask::OutG>,
        in_flight_min_out: CoinDTO<SwapTask::OutG>,
    ) -> Self {
        Self {
            spec,
            acks_left,
            total_out,
            in_flight_min_out,
            _state_enum: PhantomData,
        }
    }

    fn emit_parked(&self) -> Emitter {
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
        _querier: QuerierWrapper<'_>,
        _env: Env,
    ) -> HandlerResult<Self> {
        self.absorb(ABSORB_PARKED_RESPONSE)
    }

    fn on_remote_error(
        self,
        _response: ICAErrorResponse,
        _querier: QuerierWrapper<'_>,
        _env: Env,
    ) -> HandlerResult<Self> {
        self.absorb(ABSORB_PARKED_ERROR)
    }

    fn on_remote_timeout(self, _querier: QuerierWrapper<'_>, _env: Env) -> HandlerResult<Self> {
        self.absorb(ABSORB_PARKED_TIMEOUT)
    }

    fn price_alarm_dropped(&self) -> Option<Emitter> {
        Some(self.emit_price_alarm_dropped())
    }

    /// Re-quote the SAME in-flight leg against a fresh oracle floor and
    /// transition back to the live swap sequence with the retry counters
    /// reset. `acks_left` is unchanged - the leg is re-opened, never
    /// advanced - so the heal re-emission promises a floor freshly pinned by
    /// `RemoteSwap::open_leg`. Operator-only: an unauthorised caller is
    /// rejected before any re-quote, leaving the leg parked.
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
            .anomaly_state(self.acks_left, now, due_projection, querier)
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

/// The on-entry event a leg emits as it parks at the terminal.
pub(super) fn emit_parked_on_entry<SwapTask, SEnum>(
    terminal: &SlippageAnomaly<SwapTask, SEnum>,
) -> Emitter
where
    SwapTask: SwapTaskT,
{
    terminal.emit_parked()
}
