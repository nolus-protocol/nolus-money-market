use currency::{CurrencyDef, Group, MemberOf};
use finance::coin::{Coin, CoinDTO};
use sdk::cosmwasm_std::QuerierWrapper;

use crate::{SwapTask as SwapTaskT, error::Result};

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

    fn on<CalculatorT>(self, calculator: &CalculatorT) -> Self::Output
    where
        CalculatorT: Calculator<SwapTask::InG>,
        <<CalculatorT as Calculator<SwapTask::InG>>::OutC as CurrencyDef>::Group:
            MemberOf<SwapTask::OutG> + MemberOf<<SwapTask::InG as Group>::TopG>;
}

/// A calculator of the minimum acceptable swap output
pub trait Calculator<G>
where
    G: Group,
{
    /// The output swap currency
    type OutC: CurrencyDef;

    /// Determine the minimum output amount of a swap
    ///
    /// An anomaly is triggered if the output amount cannot be satisfied. The
    /// workflow will continue as per the result of [`Policy::on_anomaly`].
    fn min_output(
        &self,
        input: &CoinDTO<G>,
        querier: QuerierWrapper<'_>,
    ) -> Result<Coin<Self::OutC>>;
}
