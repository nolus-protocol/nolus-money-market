use currency::{CurrencyDef, Group, MemberOf};
use finance::coin::{Coin, CoinDTO};
use oracle::stub::SwapPath;
use sdk::cosmwasm_std::{Env, QuerierWrapper};
use timealarms::stub::TimeAlarmsRef;

use crate::{Account, AnomalyTreatment, slippage::WithCalculator};

pub type CoinsNb = u8;

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
    fn oracle(&self) -> &impl SwapPath<<Self::InG as Group>::TopG>;
    fn time_alarm(&self) -> &TimeAlarmsRef;

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
