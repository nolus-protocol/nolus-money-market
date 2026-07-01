use currency::{CurrencyDef, Group, MemberOf};
use finance::coin::{Coin, CoinDTO};
use sdk::cosmwasm_std::{Env, MessageInfo, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{Account, AnomalyTreatment, error::Result as DexResult, slippage::WithCalculator};

pub type CoinsNb = u8;

/// How a remote swap leg escalates once its in-flight leg keeps timing out
/// past the per-op retry budget. An explicit swap *error* is an under-floor
/// rejection and parks unconditionally; this policy governs the timeout path
/// only.
pub enum SlippageEscalation {
    /// Park the leg at the slippage-anomaly terminal: the leg freezes and
    /// waits for an operator heal instead of retrying forever.
    Park,
    /// Re-emit the in-flight leg verbatim, unbounded: the opening swap keeps
    /// re-driving a timed-out leg rather than parking it.
    ReEmit,
}

/// Specification of a swap process
///
/// Supports up to `CoinsNb::MAX` coins.
pub trait SwapTask
where
    Self: Sized,
{
    type InG: Group;
    type OutG: Group<TopG = <Self::InG as Group>::TopG>;
    type Label: Into<String>;
    type StateResponse;
    type Result;

    fn label(&self) -> Self::Label;
    fn dex_account(&self) -> &Account;
    fn time_alarm(&self) -> &TimeAlarmsRef;

    /// Authorise an inbound `RemoteLeaseCallback` against this task's
    /// owning contract.
    ///
    /// Implementations decide what "authorised" means (typically: only
    /// the remote lease controller is allowed to return a callback).
    /// Tasks that do not participate in the remote-lease protocol reject
    /// with `Error::Unauthorized`.
    fn authz_remote_callback(
        &self,
        querier: QuerierWrapper<'_>,
        info: &MessageInfo,
    ) -> DexResult<()>;

    /// Authorise an operator `heal` of a leg parked at the slippage-anomaly
    /// terminal.
    ///
    /// The re-quoting heal is operator-only: it re-pins the floor and revives
    /// a frozen leg. Implementations decide what "authorised" means (typically:
    /// only the lease admin). Tasks whose legs never park reject - they are
    /// never reached through this path.
    fn authz_anomaly_resolution(
        &self,
        querier: QuerierWrapper<'_>,
        info: &MessageInfo,
    ) -> DexResult<()>;

    /// The number of consecutive remote-swap timeouts tolerated on a leg
    /// before the slippage-anomaly terminal is entered.
    fn timeout_retry_budget(&self) -> CoinsNb;

    /// How this task's in-flight leg escalates once the timeout retry budget
    /// is spent - parking the opened legs while the opening swap keeps
    /// re-emitting verbatim. An explicit swap error parks unconditionally and
    /// does not consult this.
    fn slippage_escalation(&self) -> SlippageEscalation;

    /// Whether an explicit swap error with no leg yet acknowledged
    /// (`total_out == 0`) should clean-unwind the inputs home instead of
    /// parking at the slippage-anomaly terminal.
    ///
    /// Defaults to `false`: close, liquidation and repay specs park a hard
    /// error unconditionally. Only the opening swap overrides it - a failed
    /// open with nothing swapped yet has no live lease to park into, so it
    /// drains the inputs back and refunds rather than freezing them on the
    /// remote account. The unwind itself is produced by
    /// [`RemoteSwapClient::unwind`][crate::RemoteSwapClient::unwind]; this
    /// predicate only gates whether that path is taken. With a leg already
    /// acknowledged (`total_out > 0`) the error parks regardless of this
    /// predicate - some output is already committed on the remote side.
    fn unwind_on_zero_acked(&self) -> bool {
        false
    }

    /// Provide the coins, at least one, this swap is about.
    /// The iteration is done always in the same order.
    //
    // TODO define the Item type as an associative : AsRef<CoinDTO<Self::InG>> to allow iterating over values and references.
    // This would avoid clone-ing of values kept in the task. At the same time, we cannot iterate over '&' due to
    // having temporary instances in some of the tasks.
    fn coins(&self) -> impl IntoIterator<Item = CoinDTO<Self::InG>>;

    fn with_slippage_calc<WithCalc>(&self, with_calc: WithCalc) -> WithCalc::Output
    where
        WithCalc: WithCalculator<Self>;

    fn into_output_task<Cmd>(self, cmd: Cmd) -> Cmd::Output
    where
        Cmd: WithOutputTask<Self>;
}

pub trait WithOutputTask<SwapTaskT>
where
    SwapTaskT: SwapTask,
{
    type Output;

    fn on<OutC, OutputTaskT>(self, task: OutputTaskT) -> Self::Output
    where
        OutC: CurrencyDef,
        OutC::Group: MemberOf<<SwapTaskT::OutG as Group>::TopG> + MemberOf<SwapTaskT::OutG>,
        OutputTaskT: SwapOutputTask<SwapTaskT, OutC = OutC>;
}

pub trait SwapOutputTask<SwapTaskT>
where
    SwapTaskT: SwapTask,
{
    type OutC: CurrencyDef;

    fn as_spec(&self) -> &SwapTaskT;

    fn into_spec(self) -> SwapTaskT;

    /// Called when an anomaly is detected
    ///
    /// Determine how the current workflow should procceed.
    /// Simmilarly to [`SwapTask::finish`], this function may exit the current DEX swap task,
    /// a state composed of TransferOut, SwapExactIn, TransferIn, etc., and transition to a next state,
    /// or ask for a retry of the last operation.
    ///
    /// Due to the immaturity of the DEX Swap APIs' the particular error cannot be determined.
    /// If/once the APIs' get more mature we may want to recognize the error cause.
    /// An unsatisfied minimum output amount is always assumed whenever a swap error is received.
    fn on_anomaly(self) -> AnomalyTreatment<SwapTaskT>
    where
        Self: Sized;

    /// The final transition of this DEX composite state machine
    ///
    /// The states involve TransferOut, SwapExactIn, TransferIn, etc. This transition originates from one of them,
    /// and should point to a next state, sibling to this one in the higher-level state machine.
    /// For example, the DEX [`Lease::BuyAsset`] state transition to [`Lease::Active`] on finish.
    ///
    fn finish(
        self,
        amount_out: Coin<Self::OutC>,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> SwapTaskT::Result;
}
