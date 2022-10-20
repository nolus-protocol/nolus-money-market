use finance::currency::{AnyVisitor, Group, MaybeAnyVisitResult, Member, Symbol, SymbolStatic};

#[cfg(feature = "testing")]
use crate::test::{TestCurrencyA, TestCurrencyB, TestCurrencyC, TestCurrencyD};
use crate::{
    lease::{Atom, Osmo},
    lpn::Usdc,
    native::Nls,
    SingleVisitorAdapter,
};

impl Member<PaymentGroup> for Usdc {}
impl Member<PaymentGroup> for Osmo {}
impl Member<PaymentGroup> for Atom {}
impl Member<PaymentGroup> for Nls {}

pub struct PaymentGroup {}

impl Group for PaymentGroup {
    const DESCR: SymbolStatic = "payment";

    fn maybe_visit_on_ticker<V>(ticker: Symbol, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        V: AnyVisitor<Self>,
    {
        use finance::currency::maybe_visit_on_ticker as maybe_visit;
        let v: SingleVisitorAdapter<Self, _> = visitor.into();
        // TODO revisit the need to type parameterize AnyVisitor by Group
        // LeaseGroup::maybe_visit_on_ticker(symbol, visitor)
        //     .or_else(|v| Lpns::maybe_visit_on_ticker(symbol, v))
        //     .or_else(|v| maybe_visit::<Nls, _>(symbol, v))
        //     .map_err(|v| v.0)

        maybe_visit::<Usdc, _>(ticker, v)
            .or_else(|v| maybe_visit::<Osmo, _>(ticker, v))
            .or_else(|v| maybe_visit::<Atom, _>(ticker, v))
            .or_else(|v| maybe_visit::<Nls, _>(ticker, v))
            .or_else(|v| {
                #[cfg(feature = "testing")]
                maybe_visit::<TestCurrencyA, _>(ticker, v)
                    .or_else(|v| maybe_visit::<TestCurrencyB, _>(ticker, v))
                    .or_else(|v| maybe_visit::<TestCurrencyC, _>(ticker, v))
                    .or_else(|v| maybe_visit::<TestCurrencyD, _>(ticker, v))
            })
            .map_err(|v| v.0)
    }

    fn maybe_visit_on_bank_symbol<V>(
        bank_symbol: Symbol,
        visitor: V,
    ) -> MaybeAnyVisitResult<Self, V>
    where
        Self: Sized,
        V: AnyVisitor<Self>,
    {
        use finance::currency::maybe_visit_on_bank_symbol as maybe_visit;
        let v: SingleVisitorAdapter<Self, _> = visitor.into();
        maybe_visit::<Usdc, _>(bank_symbol, v)
            .or_else(|v| maybe_visit::<Osmo, _>(bank_symbol, v))
            .or_else(|v| maybe_visit::<Atom, _>(bank_symbol, v))
            .or_else(|v| maybe_visit::<Nls, _>(bank_symbol, v))
            .or_else(|v| {
                #[cfg(feature = "testing")]
                maybe_visit::<TestCurrencyA, _>(bank_symbol, v)
                    .or_else(|v| maybe_visit::<TestCurrencyB, _>(bank_symbol, v))
                    .or_else(|v| maybe_visit::<TestCurrencyC, _>(bank_symbol, v))
                    .or_else(|v| maybe_visit::<TestCurrencyD, _>(bank_symbol, v))
            })
            .map_err(|v| v.0)
    }
}
