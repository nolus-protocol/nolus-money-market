use currency::{CurrencyDTO, Group, MemberOf};
use finance::coin::{self, Amount, CoinDTO};

use crate::SwapTask;

pub trait MonitoredTask
where
    Self: SwapTask + Sized,
{
    fn policy(&self) -> impl Policy<Self>;
}

pub trait Policy<SwapTaskT>
where
    SwapTaskT: SwapTask,
{
    /// Determine the minimum output amount of a swap
    ///
    /// An anomaly is triggered if the output amount cannot be satisfied. The
    /// workflow will continue as per the result of [`Policy::on_anomaly`].
    fn min_output<InG>(&self, input: &CoinDTO<InG>) -> CoinDTO<SwapTaskT::OutG>
    where
        InG: Group + MemberOf<SwapTaskT::InG>;

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
    fn on_anomaly(&self, task: SwapTaskT) -> Treatment<SwapTaskT>
    where
        Self: Sized;
}

pub enum Treatment<SwapTaskT>
where
    SwapTaskT: SwapTask,
{
    Retry(SwapTaskT),
    Exit(SwapTaskT::Result),
}

pub struct PanicPolicy {}

impl<SwapTaskT> Policy<SwapTaskT> for PanicPolicy
where
    SwapTaskT: SwapTask,
{
    fn min_output<InG>(&self, _input: &CoinDTO<InG>) -> CoinDTO<<SwapTaskT as SwapTask>::OutG>
    where
        InG: Group + MemberOf<<SwapTaskT as SwapTask>::InG>,
    {
        unimplemented!("swap is not expected")
    }

    fn on_anomaly(&self, _task: SwapTaskT) -> Treatment<SwapTaskT>
    where
        Self: Sized,
    {
        unimplemented!("anomaly is not supported")
    }
}

pub struct AcceptAnyNonZeroSwap<G>
where
    G: Group,
{
    out_currency: CurrencyDTO<G>,
}

impl<G> AcceptAnyNonZeroSwap<G>
where
    G: Group,
{
    pub fn on_task<SwapTaskT>(task: &SwapTaskT) -> Self
    where
        SwapTaskT: SwapTask<OutG = G>,
    {
        Self {
            out_currency: task.out_currency(),
        }
    }
}
impl<G, SwapTaskT> Policy<SwapTaskT> for AcceptAnyNonZeroSwap<G>
where
    G: Group,
    SwapTaskT: SwapTask<OutG = G>,
{
    fn min_output<InG>(&self, _input: &CoinDTO<InG>) -> CoinDTO<SwapTaskT::OutG>
    where
        InG: Group + MemberOf<SwapTaskT::InG>,
    {
        // before, it was None on Astroport and "1" on Osmosis.
        const MIN_AMOUNT_OUT: Amount = 1;
        coin::from_amount_ticker(MIN_AMOUNT_OUT, self.out_currency)
    }

    fn on_anomaly(&self, task: SwapTaskT) -> Treatment<SwapTaskT>
    where
        Self: Sized,
    {
        Treatment::Retry(task)
    }
}
