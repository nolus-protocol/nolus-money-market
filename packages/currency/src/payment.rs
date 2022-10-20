use finance::currency::{
    self, AnyVisitor, Group, MaybeAnyVisitResult, Member, Symbol, SymbolStatic,
};

use crate::{
    lease::{Atom, LeaseGroup, Osmo, Wbtc, Weth},
    lpn::{Lpns, Usdc},
    native::Nls,
    SingleVisitorAdapter,
};

impl Member<PaymentGroup> for Usdc {}
impl Member<PaymentGroup> for Osmo {}
impl Member<PaymentGroup> for Atom {}
impl Member<PaymentGroup> for Weth {}
impl Member<PaymentGroup> for Wbtc {}
impl Member<PaymentGroup> for Nls {}

pub struct PaymentGroup {}

impl Group for PaymentGroup {
    const DESCR: SymbolStatic = "payment";

    fn maybe_visit_on_ticker<V>(ticker: Symbol, visitor: V) -> MaybeAnyVisitResult<V>
    where
        V: AnyVisitor,
    {
        LeaseGroup::maybe_visit_on_ticker(ticker, visitor)
            .or_else(|v| Lpns::maybe_visit_on_ticker(ticker, v))
            .or_else(|v| {
                currency::maybe_visit_on_ticker::<Nls, _>(ticker, SingleVisitorAdapter::from(v))
            })
            .map_err(|v| v.0)
    }

    fn maybe_visit_on_bank_symbol<V>(bank_symbol: Symbol, visitor: V) -> MaybeAnyVisitResult<V>
    where
        Self: Sized,
        V: AnyVisitor,
    {
        LeaseGroup::maybe_visit_on_bank_symbol(bank_symbol, visitor)
            .or_else(|v| Lpns::maybe_visit_on_bank_symbol(bank_symbol, v))
            .or_else(|v| {
                currency::maybe_visit_on_bank_symbol::<Nls, _>(
                    bank_symbol,
                    SingleVisitorAdapter::from(v),
                )
            })
            .map_err(|v| v.0)
    }
}
