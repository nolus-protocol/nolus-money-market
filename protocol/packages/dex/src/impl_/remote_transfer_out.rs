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
