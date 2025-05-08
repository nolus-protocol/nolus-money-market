use currency::{AnyVisitor, AnyVisitorResult, CurrencyDTO, CurrencyDef, MemberOf, never::Never};
use dex::{AcceptAnyNonZeroSwap, SwapTask, WithCalculator};

use super::BuyAsset;

pub struct Factory<WithCalc> {
    with_calc: WithCalc,
}
impl<WithCalc> Factory<WithCalc> {
    pub fn from(with_calc: WithCalc) -> Self {
        Self { with_calc }
    }
}
impl<WithCalc> AnyVisitor<<BuyAsset as SwapTask>::OutG> for Factory<WithCalc>
where
    WithCalc: WithCalculator<BuyAsset>,
{
    type Output = WithCalc::Output;

    type Error = Never;

    fn on<C>(
        self,
        _def: &CurrencyDTO<C::Group>,
    ) -> AnyVisitorResult<<BuyAsset as SwapTask>::OutG, Self>
    where
        C: CurrencyDef,
        C::Group: MemberOf<<BuyAsset as SwapTask>::OutG>
            + MemberOf<<<BuyAsset as SwapTask>::OutG as currency::Group>::TopG>,
    {
        Ok(self.with_calc.on(AcceptAnyNonZeroSwap::<_, C>::default()))
    }
}
