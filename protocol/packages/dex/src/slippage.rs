use currency::{CurrencyDef, Group, MemberOf};
use finance::coin::{Coin, CoinDTO};

use crate::SwapTask as SwapTaskT;

/// Execute logic with a [`Calculator`]
///
/// This call-back style trait is needed to resolve the swap spec output currency in the calculator.
/// For some usecases, the swap spec output currency is known at compile time,
/// for example, on closing/liquidating a position.
/// In other usecases, the swap spec output currency is known only at run time,
/// for example, on opening a position.
pub trait WithCalculator<SwapTask>
where
    SwapTask: SwapTaskT,
{
    type Output;

    fn on<CalculatorT>(self, calculator: CalculatorT) -> Self::Output
    where
        CalculatorT: Calculator<SwapTask>,
        <<CalculatorT as Calculator<SwapTask>>::OutC as CurrencyDef>::Group:
            MemberOf<SwapTask::OutG> + MemberOf<<SwapTask::InG as Group>::TopG>;
}

/// Factory of [`Calculator`]-s
pub trait CalculatorFactory<SwapTask>
where
    SwapTask: SwapTaskT,
{
    type OutC: CurrencyDef;

    fn new_calc(&self) -> impl Calculator<SwapTask, OutC = Self::OutC>;
}

/// A calculator of the minimum acceptable swap output
pub trait Calculator<SwapTask>
where
    SwapTask: SwapTaskT,
{
    /// The output swap currency
    type OutC: CurrencyDef;

    /// The specification of a swap this calculator protects
    fn as_spec(&self) -> &SwapTask;

    /// Determine the minimum output amount of a swap
    ///
    /// An anomaly is triggered if the output amount cannot be satisfied. The
    /// workflow will continue as per the result of [`Policy::on_anomaly`].
    fn min_output<InG>(&self, input: &CoinDTO<InG>) -> Coin<Self::OutC>
    where
        InG: Group + MemberOf<SwapTask::InG>;
}
